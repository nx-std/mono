//! C-ABI FFI surface — the `__nx_net__*` resolver symbols.
//!
//! This module is the hard shell and the crate's entire C-ABI: it is compiled
//! only with the `ffi` feature, and the idiomatic Rust API below it never
//! references anything here. It is built from two halves — the C-ABI types
//! and conversions, and the FFI runtime that the `extern "C"` exports stand
//! on:
//!
//! - [`abi`] — `repr(C)` mirrors of the BSD networking structs and the
//!   `AF_*` / `SOCK_*` / `AI_*` / `NI_*` / `EAI_*` / `NETDB_*` / errno integer
//!   constants;
//! - [`convert`] — `TryFrom<c_int>` parsers that validate untrusted C integers
//!   into the soft core's input enums;
//! - [`error_map`] — the mapping from the resolver's idiomatic Rust errors
//!   onto the `errno` / `h_errno` / `EAI_*` codes a C caller reads;
//! - [`session`] — the process-wide `sfdnsres` resolver session, connected
//!   lazily on first use and shared by every export;
//! - [`errno`] — writers for the C thread-local `errno` / `h_errno` a
//!   resolver caller observes;
//! - [`block`] — the single-block result allocator, packer, and deallocator
//!   that lets `freeaddrinfo` / `freehostent` release a result with one call.
//!
//! Each `extern "C"` entry point validates its raw C inputs, converts them
//! into the crate's validated core types, calls the resolver, and packs the
//! result back into the `repr(C)` structs. The producer build never activates
//! this feature; only consumers that link the `net_override.ld` script do.
//!
//! This file declares the C-ABI and runtime submodules and holds the full
//! resolver surface: the `getaddrinfo` / `freeaddrinfo` pair, the
//! `gethostby*` / `freehostent` host-lookup family, `getnameinfo`, and the
//! `gai_strerror` / `hstrerror` / `herror` error-string exports.

pub mod abi;
pub mod block;
pub mod convert;
pub mod errno;
pub mod error_map;
pub mod session;

use core::{
    ffi::{
        CStr,
        c_char,
        c_int,
        c_void,
    },
    mem::size_of,
    net::{
        Ipv4Addr,
        Ipv6Addr,
        SocketAddr,
        SocketAddrV4,
        SocketAddrV6,
    },
    ptr,
};

use nx_service_sfdnsres::NameInfoFlags;

use self::{
    abi::{
        addrinfo,
        hostent,
        sockaddr,
    },
    block::{
        alloc_addrinfo_node,
        alloc_hostent_block,
        free_block,
    },
    error_map::{
        gai_strerror,
        hstrerror,
    },
    session::with_resolver,
};
use crate::resolve::{
    family::{
        AddrFamily,
        Protocol,
        SockType,
    },
    hints::AddrInfoHints,
    hostname::Hostname,
    resolver::{
        HostEntry,
        lookup_addrinfo,
        lookup_host_by_addr,
        lookup_host_by_name,
        lookup_nameinfo,
    },
    service::ServiceSpec,
};

/// Length, in bytes, of an IPv4 address — the only `gethostbyaddr` input.
const IPV4_ADDR_LEN: u32 = 4;

/// Resolves a node name and/or service into a C-ABI `addrinfo`
/// chain.
///
/// Mirrors the BSD `getaddrinfo`: on success `*res` receives the head of an
/// `ai_next`-linked chain — each node a single block (see [`block`]) that
/// [`__nx_net__freeaddrinfo`] releases — and the call returns `0`. On failure
/// it returns a non-zero `EAI_*` code, leaves `*res` null, and sets `errno`
/// for the `EAI_SYSTEM` / `EAI_MEMORY` cases.
///
/// At least one of `node` / `service` must be a non-null, NUL-terminated C
/// string; a null argument means "absent".
///
/// # Safety
///
/// `node` and `service`, when non-null, must point to NUL-terminated C
/// strings. `hints`, when non-null, must point to a valid `addrinfo`. `res`
/// must be a non-null, writable pointer to a `*mut addrinfo` slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__getaddrinfo(
    node: *const c_char,
    service: *const c_char,
    hints: *const addrinfo,
    res: *mut *mut addrinfo,
) -> c_int {
    // A result slot is mandatory — without it there is nowhere to report
    // success, so the request cannot be honoured.
    if res.is_null() {
        return abi::EAI_FAIL;
    }
    // SAFETY: `res` is non-null and, by contract, a writable `*mut addrinfo`
    // slot. Clear it so every early return leaves the caller a null chain.
    unsafe { *res = ptr::null_mut() };

    // Parse the node and service arguments into validated core types; a
    // malformed argument is rejected here, at the hard shell.
    // SAFETY: `node` is null or a NUL-terminated C string, per the contract.
    let node = match unsafe { parse_arg::<Hostname>(node) } {
        Ok(node) => node,
        Err(()) => return fail(abi::EAI_NONAME, None),
    };
    // SAFETY: `service` is null or a NUL-terminated C string, per the contract.
    let service = match unsafe { parse_arg::<ServiceSpec>(service) } {
        Ok(service) => service,
        Err(()) => return fail(abi::EAI_NONAME, None),
    };
    if node.is_none() && service.is_none() {
        return fail(abi::EAI_NONAME, None);
    }

    // A null `hints` means "no constraints" — the all-default record.
    let hints = if hints.is_null() {
        AddrInfoHints::default()
    } else {
        // SAFETY: `hints` is non-null and points to a valid `addrinfo`.
        match read_hints(unsafe { &*hints }) {
            Ok(hints) => hints,
            Err(code) => return fail(code, None),
        }
    };

    // Run the lookup over the shared resolver session.
    let list =
        match with_resolver(|svc| lookup_addrinfo(svc, node.as_ref(), service.as_ref(), &hints)) {
            Ok(Ok(list)) => list,
            // The resolver classified the failure: surface its own codes.
            Ok(Err(err)) => return fail(err.gai_code(), Some(err.errno())),
            // The `sfdnsres` session could not be established.
            Err(_) => return fail(abi::EAI_AGAIN, Some(abi::EAGAIN)),
        };
    if list.is_empty() {
        return fail(abi::EAI_NODATA, None);
    }

    // Pack each decoded record into its own single block and link the chain.
    let mut head: *mut addrinfo = ptr::null_mut();
    let mut tail: *mut addrinfo = ptr::null_mut();
    for record in list.records() {
        let node = alloc_addrinfo_node(record);
        if node.is_null() {
            // SAFETY: `head` is a chain of nodes built by this loop.
            unsafe { free_chain(head) };
            return fail(abi::EAI_MEMORY, Some(abi::ENOMEM));
        }
        if tail.is_null() {
            head = node;
        } else {
            // SAFETY: `tail` is a node from `alloc_addrinfo_node`, writable.
            unsafe { (*tail).ai_next = node };
        }
        tail = node;
    }

    // SAFETY: `res` is the writable slot validated above.
    unsafe { *res = head };
    0
}

/// Releases an `addrinfo` chain produced by [`__nx_net__getaddrinfo`].
///
/// Mirrors the BSD `freeaddrinfo`: it walks `ai_next` and frees every node.
/// A null `ai` is a no-op.
///
/// # Safety
///
/// `ai` must be null or the head of a chain returned by
/// [`__nx_net__getaddrinfo`] that has not already been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__freeaddrinfo(ai: *mut addrinfo) {
    // SAFETY: `ai` is null or a chain produced by `__nx_net__getaddrinfo`.
    unsafe { free_chain(ai) };
}

/// Resolves a host name into a C-ABI `hostent`.
///
/// Mirrors the BSD `gethostbyname`: on success it returns a single-block
/// `hostent` (see [`block`]) that [`__nx_net__freehostent`] releases, and
/// leaves `h_errno` at `NETDB_SUCCESS`. On failure it returns null and sets
/// `h_errno` and `errno`.
///
/// # Safety
///
/// `name`, when non-null, must point to a NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__gethostbyname(name: *const c_char) -> *mut hostent {
    // SAFETY: `name` is null or a NUL-terminated C string, per the contract.
    unsafe { host_by_name(name, AddrFamily::Inet) }
}

/// Resolves a host name into a `hostent`, restricted to address family `af`.
///
/// The musl extension to [`__nx_net__gethostbyname`]: `af` narrows the result
/// the way musl's `gethostbyname2` does. `sfdnsres` only ever returns IPv4
/// host records, so an `AF_INET6` request keeps the entry's name and aliases
/// but yields no addresses. An `af` that is not a supported `AF_*` value fails
/// with `h_errno = HOST_NOT_FOUND` / `errno = EAFNOSUPPORT`.
///
/// # Safety
///
/// `name`, when non-null, must point to a NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__gethostbyname2(name: *const c_char, af: c_int) -> *mut hostent {
    let family = match AddrFamily::try_from(af) {
        Ok(family) => family,
        Err(_) => return host_fail(abi::HOST_NOT_FOUND, abi::EAFNOSUPPORT),
    };
    // SAFETY: `name` is null or a NUL-terminated C string, per the contract.
    unsafe { host_by_name(name, family) }
}

/// Reverse-resolves an IPv4 address into a C-ABI `hostent`.
///
/// Mirrors the BSD `gethostbyaddr`: `addr` must point to a four-octet IPv4
/// address, `len` must be `4`, and `addr_type` must be `AF_INET` — the only
/// family `sfdnsres` reverse-resolves. On success it returns a single-block
/// `hostent`; on failure it returns null and sets `h_errno` / `errno`.
///
/// # Safety
///
/// `addr`, when non-null, must point to at least `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__gethostbyaddr(
    addr: *const c_void,
    len: u32,
    addr_type: c_int,
) -> *mut hostent {
    // A missing address cannot be resolved.
    if addr.is_null() || len == 0 {
        return host_fail(abi::HOST_NOT_FOUND, abi::EINVAL);
    }
    // `sfdnsres` reverse-resolves IPv4 only.
    if addr_type != abi::AF_INET {
        return host_fail(abi::HOST_NOT_FOUND, abi::EOPNOTSUPP);
    }
    // An IPv4 address is exactly four octets — reject any other length.
    if len != IPV4_ADDR_LEN {
        return host_fail(abi::HOST_NOT_FOUND, abi::EINVAL);
    }

    // SAFETY: `addr` is non-null and, with `len == 4`, points to four readable
    // octets; `read_unaligned` tolerates any pointer alignment.
    let octets = unsafe { ptr::read_unaligned(addr.cast::<[u8; 4]>()) };
    let ip = Ipv4Addr::from(octets);

    match with_resolver(|svc| lookup_host_by_addr(svc, ip)) {
        Ok(Ok(entry)) => pack_hostent(&entry),
        Ok(Err(err)) => host_fail(err.h_errno(), err.errno()),
        Err(_) => host_fail(abi::NETDB_INTERNAL, abi::EAGAIN),
    }
}

/// Releases a `hostent` produced by the `__nx_net__gethostby*` family.
///
/// Mirrors the BSD `freehostent`: it releases the single result block. A null
/// `he` is a no-op.
///
/// # Safety
///
/// `he` must be null or a `hostent` returned by a `__nx_net__gethostby*`
/// function that has not already been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__freehostent(he: *mut hostent) {
    if he.is_null() {
        return;
    }
    // SAFETY: `he` is the payload pointer of a block from `alloc_hostent_block`,
    // not yet freed.
    unsafe { free_block(he.cast::<u8>()) };
}

/// Reverse-resolves a socket address into its host and service names.
///
/// Mirrors the BSD `getnameinfo`: it decodes the caller's `sockaddr`, asks the
/// resolver to translate it, and copies the resolved host and service names —
/// each NUL-terminated — into the caller's `host` / `serv` buffers. On success
/// it returns `0`; on failure it returns a non-zero `EAI_*` code and sets
/// `errno` for the cases the resolver classified.
///
/// A null `host` or a zero `hostlen` means "the host name is not wanted"; the
/// same holds for `serv` / `servlen`. A buffer too small for the resolved name
/// plus its terminator fails with `EAI_OVERFLOW`. `flags` is the bitwise-or of
/// the `NI_*` constants and is passed through to the resolver unchanged.
///
/// # Safety
///
/// `sa` must be non-null and point to at least `salen` readable bytes.
/// `host`, when non-null, must point to at least `hostlen` writable bytes;
/// `serv`, when non-null, to at least `servlen` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__getnameinfo(
    sa: *const sockaddr,
    salen: u32,
    host: *mut c_char,
    hostlen: u32,
    serv: *mut c_char,
    servlen: u32,
    flags: c_int,
) -> c_int {
    // A missing address has nothing to reverse-resolve.
    if sa.is_null() {
        return fail(abi::EAI_FAIL, None);
    }
    // Decode the caller's `sockaddr` into a validated Rust socket address;
    // an unsupported family or a too-short `salen` is rejected here.
    // SAFETY: `sa` is non-null and, by contract, points to `salen` readable
    // bytes.
    let addr = match unsafe { read_sockaddr(sa, salen) } {
        Ok(addr) => addr,
        Err(code) => return fail(code, None),
    };

    // Run the lookup over the shared resolver session.
    // The C caller's bitmask is adopted here, at the boundary it arrives
    // through, so the resolver below never handles an untyped word.
    let flags = NameInfoFlags::from_raw(flags as u32);
    let info = match with_resolver(|svc| lookup_nameinfo(svc, &addr, flags)) {
        Ok(Ok(info)) => info,
        // The resolver classified the failure: surface its own codes.
        Ok(Err(err)) => return fail(err.gai_code(), Some(err.errno())),
        // The `sfdnsres` session could not be established.
        Err(_) => return fail(abi::EAI_AGAIN, Some(abi::EAGAIN)),
    };

    // Copy each resolved name into the caller's buffer; a buffer too small for
    // the name plus its terminator is an overflow.
    // SAFETY: `host` / `serv` are null or point to `hostlen` / `servlen`
    // writable bytes, per the contract.
    let copied = unsafe {
        copy_cstr(host, hostlen, info.host()).and(copy_cstr(serv, servlen, info.service()))
    };
    if copied.is_err() {
        return fail(abi::EAI_OVERFLOW, None);
    }
    0
}

/// Returns the textual description of a `getaddrinfo` `EAI_*` error code.
///
/// Mirrors the BSD `gai_strerror`: the returned pointer addresses a static,
/// NUL-terminated string owned by the library — the caller must not free it.
/// An unrecognized code yields a generic description rather than null.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_net__gai_strerror(err: c_int) -> *const c_char {
    gai_strerror(err).as_ptr()
}

/// Returns the textual description of a host-lookup `h_errno` error code.
///
/// Mirrors the BSD `hstrerror`: the returned pointer addresses a static,
/// NUL-terminated string owned by the library — the caller must not free it.
/// An unrecognized code yields a generic description rather than null.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_net__hstrerror(err: c_int) -> *const c_char {
    hstrerror(err).as_ptr()
}

/// Prints a host-lookup diagnostic for the current `h_errno` to standard
/// error.
///
/// Mirrors the BSD `herror`: it writes `"<s>: <description>\n"` — or just
/// `"<description>\n"` when `s` is null or empty — where `<description>` is
/// [`__nx_net__hstrerror`] of the calling thread's current `h_errno`.
///
/// # Safety
///
/// `s` must be null or point to a NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_net__herror(s: *const c_char) {
    /// POSIX standard-error file descriptor.
    const STDERR_FILENO: c_int = 2;

    unsafe extern "C" {
        // newlib's raw `write(2)` — the diagnostic's only output path.
        fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    }

    // Writes a byte slice to standard error; a short write is ignored, as a
    // best-effort diagnostic has nothing to recover to.
    let emit = |bytes: &[u8]| {
        // SAFETY: `bytes` is a valid slice of `bytes.len()` readable bytes.
        unsafe { write(STDERR_FILENO, bytes.as_ptr().cast::<c_void>(), bytes.len()) };
    };

    // Optional caller-supplied prefix: "<s>: ".
    if !s.is_null() {
        // SAFETY: `s` is non-null and, by contract, a NUL-terminated C string.
        let prefix = unsafe { CStr::from_ptr(s) }.to_bytes();
        if !prefix.is_empty() {
            emit(prefix);
            emit(b": ");
        }
    }

    // The description of the current `h_errno`, then a newline.
    emit(hstrerror(errno::get_h_errno()).to_bytes());
    emit(b"\n");
}

/// Resolves a host name into a `hostent`, shared by the `gethostbyname`
/// family.
///
/// A null or malformed name is rejected here, at the hard shell, as the C
/// resolver reports a missing name (`h_errno = HOST_NOT_FOUND`,
/// `errno = EINVAL`).
///
/// # Safety
///
/// `name` must be null or point to a NUL-terminated C string.
unsafe fn host_by_name(name: *const c_char, family: AddrFamily) -> *mut hostent {
    // SAFETY: `name` is null or a NUL-terminated C string, per the contract.
    let name = match unsafe { parse_arg::<Hostname>(name) } {
        Ok(Some(name)) => name,
        Ok(None) | Err(()) => return host_fail(abi::HOST_NOT_FOUND, abi::EINVAL),
    };

    match with_resolver(|svc| lookup_host_by_name(svc, &name, family)) {
        Ok(Ok(entry)) => pack_hostent(&entry),
        // The resolver classified the failure: surface its own codes.
        Ok(Err(err)) => host_fail(err.h_errno(), err.errno()),
        // The `sfdnsres` session could not be established.
        Err(_) => host_fail(abi::NETDB_INTERNAL, abi::EAGAIN),
    }
}

/// Packs a decoded host entry into its single result block.
///
/// On success `h_errno` is cleared to `NETDB_SUCCESS`; an allocation failure
/// is reported as `h_errno = NETDB_INTERNAL` / `errno = ENOMEM`.
fn pack_hostent(entry: &HostEntry) -> *mut hostent {
    let block = alloc_hostent_block(entry);
    if block.is_null() {
        return host_fail(abi::NETDB_INTERNAL, abi::ENOMEM);
    }
    errno::set_h_errno(abi::NETDB_SUCCESS);
    block
}

/// Reports a host-lookup failure: sets `h_errno` and `errno`, returns null.
fn host_fail(h_errno: c_int, errno: c_int) -> *mut hostent {
    errno::set_h_errno(h_errno);
    errno::set_errno(errno);
    ptr::null_mut()
}

/// Parses an optional NUL-terminated C string argument into a validated input.
///
/// A null pointer yields `Ok(None)` — the argument is absent. A non-null
/// pointer is decoded as a C string and parsed into `T`; a parse failure is
/// reported as `Err(())` so the caller can map it to an `EAI_*` code.
///
/// # Safety
///
/// `ptr` must be null or point to a NUL-terminated C string.
unsafe fn parse_arg<T>(ptr: *const c_char) -> Result<Option<T>, ()>
where
    for<'a> T: TryFrom<&'a CStr>,
{
    if ptr.is_null() {
        return Ok(None);
    }
    // SAFETY: `ptr` is non-null and, by contract, a NUL-terminated C string.
    let cstr = unsafe { CStr::from_ptr(ptr) };
    T::try_from(cstr).map(Some).map_err(|_| ())
}

/// Converts a caller-supplied C `addrinfo` hints record into the resolver's
/// validated [`AddrInfoHints`].
///
/// Returns the rejecting `EAI_*` code when a selector field names a family,
/// socket type, or protocol the resolver does not support.
fn read_hints(hints: &addrinfo) -> Result<AddrInfoHints, c_int> {
    let family = AddrFamily::try_from(hints.ai_family).map_err(|_| abi::EAI_FAMILY)?;
    let socktype = SockType::try_from(hints.ai_socktype).map_err(|_| abi::EAI_SOCKTYPE)?;
    let protocol = Protocol::try_from(hints.ai_protocol).map_err(|_| abi::EAI_BADHINTS)?;
    Ok(AddrInfoHints {
        flags: hints.ai_flags,
        family,
        socktype,
        protocol,
    })
}

/// Decodes a caller-supplied C `sockaddr` into a validated Rust socket
/// address.
///
/// Returns the rejecting `EAI_*` code when `salen` is too short for the
/// address family's `sockaddr_*` struct, or when `sa_family` names a family
/// the resolver does not reverse-resolve.
///
/// # Safety
///
/// `sa` must be non-null and point to at least `salen` readable bytes.
unsafe fn read_sockaddr(sa: *const sockaddr, salen: u32) -> Result<SocketAddr, c_int> {
    let salen = salen as usize;
    // The shared `sa_len` / `sa_family` header must be readable to classify
    // the address family.
    if salen < 2 {
        return Err(abi::EAI_FAMILY);
    }
    // SAFETY: at least two bytes are readable; `sockaddr` has alignment 1, so
    // reading the `sa_family` field needs no extra alignment.
    let family = c_int::from(unsafe { (*sa).sa_family });
    match family {
        abi::AF_INET => {
            if salen < size_of::<abi::sockaddr_in>() {
                return Err(abi::EAI_FAMILY);
            }
            // SAFETY: `sa` points to at least `size_of::<sockaddr_in>()`
            // readable bytes; `read_unaligned` tolerates any alignment.
            let sin = unsafe { ptr::read_unaligned(sa.cast::<abi::sockaddr_in>()) };
            let ip = Ipv4Addr::from(sin.sin_addr.s_addr.to_ne_bytes());
            Ok(SocketAddr::V4(SocketAddrV4::new(
                ip,
                u16::from_be(sin.sin_port),
            )))
        }
        abi::AF_INET6 => {
            if salen < size_of::<abi::sockaddr_in6>() {
                return Err(abi::EAI_FAMILY);
            }
            // SAFETY: `sa` points to at least `size_of::<sockaddr_in6>()`
            // readable bytes; `read_unaligned` tolerates any alignment.
            let sin6 = unsafe { ptr::read_unaligned(sa.cast::<abi::sockaddr_in6>()) };
            let ip = Ipv6Addr::from(sin6.sin6_addr.s6_addr);
            Ok(SocketAddr::V6(SocketAddrV6::new(
                ip,
                u16::from_be(sin6.sin6_port),
                sin6.sin6_flowinfo,
                sin6.sin6_scope_id,
            )))
        }
        _ => Err(abi::EAI_FAMILY),
    }
}

/// Copies a resolved name, NUL-terminated, into a caller-supplied C buffer.
///
/// A null `dst` or a zero `len` means the caller did not request this name —
/// the copy is skipped and reported as success. Otherwise the text plus its
/// terminator must fit within `len` bytes; a buffer too small is reported as
/// `Err(())` so the caller can map it to `EAI_OVERFLOW`.
///
/// # Safety
///
/// `dst` must be null or point to at least `len` writable bytes.
unsafe fn copy_cstr(dst: *mut c_char, len: u32, text: &str) -> Result<(), ()> {
    if dst.is_null() || len == 0 {
        return Ok(());
    }
    let bytes = text.as_bytes();
    // The terminator must fit alongside the text.
    if bytes.len() + 1 > len as usize {
        return Err(());
    }
    // SAFETY: `dst` points to at least `len` writable bytes, and
    // `bytes.len() + 1 <= len`; the buffers do not overlap.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), dst, bytes.len());
        *dst.add(bytes.len()) = 0;
    }
    Ok(())
}

/// Reports a `getaddrinfo` failure: sets `errno` when one was classified and
/// returns the `EAI_*` code.
fn fail(gai_code: c_int, errno: Option<c_int>) -> c_int {
    if let Some(code) = errno {
        errno::set_errno(code);
    }
    gai_code
}

/// Frees every node of an `ai_next`-linked `addrinfo` chain.
///
/// # Safety
///
/// `ai` must be null or the head of a chain of nodes produced by
/// [`alloc_addrinfo_node`], none of which has already been freed.
unsafe fn free_chain(ai: *mut addrinfo) {
    let mut cur = ai;
    while !cur.is_null() {
        // SAFETY: `cur` is a live node; `ai_next` links the rest of the chain.
        let next = unsafe { (*cur).ai_next };
        // SAFETY: `cur` is the payload pointer of a block from
        // `alloc_addrinfo_node`, not yet freed.
        unsafe { free_block(cur.cast::<u8>()) };
        cur = next;
    }
}
