//! Reading and changing how a socket behaves.
//!
//! Options, descriptor flags, device requests and kernel parameters. What they have in common is
//! that none of them moves data: each names a setting and either reads it or replaces it.
//!
//! ## Options are bytes, deliberately
//!
//! A `level`/`optname` pair names a point in a namespace the service owns and can extend, so this
//! module does not enumerate them. What a caller supplies is a buffer and a length, and what goes
//! to the service is those bytes. The typed accessors [`nx_service_bsd`] offers are for a Rust
//! caller that knows which option it is asking about; a C caller has already decided, and passed
//! the buffer to prove it.

use alloc::vec;
use core::ffi::{
    CStr,
    c_char,
    c_int,
    c_uint,
    c_void,
};

use nx_service_bsd::{
    FcntlOp,
    StatusFlags,
};

use super::{
    abi::SockLenT,
    descriptor::with_socket,
    errno,
};
use crate::session;

/// Reads a socket option into the caller's buffer.
///
/// # Safety
///
/// `optval` must be null or point to at least `*optlen` writable bytes, and `optlen` must be null
/// or point to a writable length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__getsockopt(
    sockfd: c_int,
    level: c_int,
    optname: c_int,
    optval: *mut c_void,
    optlen: *mut SockLenT,
) -> c_int {
    if optval.is_null() || optlen.is_null() {
        return errno::fail(errno::EFAULT);
    }

    // SAFETY: the caller guarantees a writable length at a non-null pointer.
    let capacity = unsafe { *optlen } as usize;
    // SAFETY: the caller guarantees `capacity` writable bytes at `optval`.
    let value = unsafe { core::slice::from_raw_parts_mut(optval.cast::<u8>(), capacity) };

    // The service sizes its answer from the buffer it is given, so the caller's declared capacity
    // is what is asked for and what comes back is at most that.
    let written = match with_socket(sockfd, |svc, sock| {
        svc.get_sock_opt_bytes(sock, level, optname, value)
    }) {
        Ok(written) => written,
        Err(failure) => return failure,
    };

    // SAFETY: the caller guarantees a writable length.
    // `written` is bounded by `capacity`, which came from this same `SockLenT`.
    unsafe { *optlen = written as SockLenT };

    0
}

/// Writes a socket option from the caller's buffer.
///
/// # Safety
///
/// `optval` must be null or point to at least `optlen` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__setsockopt(
    sockfd: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: SockLenT,
) -> c_int {
    if optval.is_null() && optlen != 0 {
        return errno::fail(errno::EFAULT);
    }

    // SAFETY: the caller guarantees `optlen` readable bytes at `optval`.
    let value = unsafe { core::slice::from_raw_parts(optval.cast::<u8>(), optlen as usize) };

    match with_socket(sockfd, |svc, sock| {
        svc.set_sock_opt_bytes(sock, level, optname, value)
    }) {
        Ok(()) => 0,
        Err(failure) => failure,
    }
}

/// Reads or replaces a descriptor's status flags.
///
/// Only `F_GETFL` and `F_SETFL` are implemented, which is the whole of what the service offers and
/// what the C driver exposes.
///
/// # Safety
///
/// Variadic in C. The third argument is read as an `int` only for `F_SETFL`, which is the only
/// command that takes one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int {
    /// `F_GETFL`, as newlib numbers it.
    const F_GETFL: c_int = 3;
    /// `F_SETFL`, as newlib numbers it.
    const F_SETFL: c_int = 4;
    /// `O_NONBLOCK`, as newlib numbers it.
    const O_NONBLOCK: c_int = 0x4000;

    let op = match cmd {
        F_GETFL => FcntlOp::GetFlags,
        F_SETFL => {
            // The caller's word is in newlib's numbering and the service reads its own, so the
            // flag is translated rather than forwarded. Anything else in the word describes how
            // the descriptor was opened and cannot be replaced, so it is refused rather than
            // dropped: silently ignoring it would leave the caller believing it took.
            if arg & !O_NONBLOCK != 0 {
                return errno::fail(errno::EINVAL);
            }
            let mut flags = StatusFlags::empty();
            if arg & O_NONBLOCK != 0 {
                flags |= StatusFlags::NONBLOCK;
            }
            FcntlOp::SetFlags(flags)
        }
        // The service implements only these two, so anything else is refused here rather than
        // sent and rejected.
        _ => return errno::fail(errno::EINVAL),
    };

    match with_socket(fd, |svc, sock| svc.fcntl(sock, op)) {
        Ok(flags) => {
            if flags.contains(StatusFlags::NONBLOCK) {
                O_NONBLOCK
            } else {
                0
            }
        }
        Err(failure) => failure,
    }
}

/// Issues a device control request.
///
/// # Safety
///
/// Variadic in C. `argp` must be null or point to a buffer valid for the request, which decides
/// how many bytes it reads and writes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__ioctl(
    fd: c_int,
    request: c_int,
    argp: *mut c_void,
) -> c_int {
    /// How many bytes a request's argument occupies, encoded in the request itself.
    const IOC_SIZE_SHIFT: c_int = 16;
    /// Mask selecting that size.
    const IOC_SIZE_MASK: c_int = 0x1FFF;

    let len = ((request >> IOC_SIZE_SHIFT) & IOC_SIZE_MASK) as usize;

    if argp.is_null() && len != 0 {
        return errno::fail(errno::EFAULT);
    }

    // SAFETY: the request encodes the length of its own argument, and the caller supplied a buffer
    // matching the request it chose.
    let data = unsafe { core::slice::from_raw_parts_mut(argp.cast::<u8>(), len) };

    match with_socket(fd, |svc, sock| svc.ioctl(sock, request, data)) {
        Ok(ret) => ret,
        Err(failure) => failure,
    }
}

/// Reads or writes a kernel networking parameter, named by a MIB.
///
/// # Safety
///
/// `name` must point to `namelen` readable integers. `oldp`/`oldlenp` and `newp` must be null or
/// valid for the lengths they declare.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__sysctl(
    name: *const c_int,
    namelen: c_uint,
    oldp: *mut c_void,
    oldlenp: *mut usize,
    newp: *const c_void,
    newlen: usize,
) -> c_int {
    if name.is_null() || namelen == 0 {
        return errno::fail(errno::EINVAL);
    }

    // SAFETY: the caller guarantees `namelen` readable integers at `name`.
    let mib = unsafe { core::slice::from_raw_parts(name, namelen as usize) };

    let new_value: &[u8] = if newp.is_null() || newlen == 0 {
        &[]
    } else {
        // SAFETY: the caller guarantees `newlen` readable bytes at `newp`.
        unsafe { core::slice::from_raw_parts(newp.cast::<u8>(), newlen) }
    };

    let capacity = if oldlenp.is_null() {
        0
    } else {
        // SAFETY: the caller guarantees a readable length at a non-null pointer.
        unsafe { *oldlenp }
    };

    let mut old_value = vec![0u8; capacity];

    let written = match session::with_service(|svc| svc.sysctl(mib, new_value, &mut old_value)) {
        Err(_) => return errno::fail(errno::EBADF),
        Ok(Err(err)) => return errno::report(err),
        // The service reports a length into a buffer this process allocated, so it fits a `usize`.
        Ok(Ok(written)) => written as usize,
    };

    if !oldp.is_null() && capacity != 0 {
        let copied = core::cmp::min(capacity, written);
        // SAFETY: `copied` is bounded by both the caller's capacity and the staging buffer.
        unsafe { core::ptr::copy_nonoverlapping(old_value.as_ptr(), oldp.cast::<u8>(), copied) };
    }
    if !oldlenp.is_null() {
        // SAFETY: the caller guarantees a writable length at a non-null pointer.
        unsafe { *oldlenp = written };
    }

    0
}

/// Reads or writes a kernel networking parameter, named by a string.
///
/// Resolves the name to a MIB with [`__nx_sys_net__sysctlnametomib`], then reads or writes it as
/// [`__nx_sys_net__sysctl`] does. The service offers no command that takes a name, so the two
/// steps are what the name lookup is: there is no shortcut to add later.
///
/// # Safety
///
/// `name` must be a null-terminated string. `oldp`/`oldlenp` and `newp` must be null or valid for
/// the lengths they declare.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__sysctlbyname(
    name: *const c_char,
    oldp: *mut c_void,
    oldlenp: *mut usize,
    newp: *const c_void,
    newlen: usize,
) -> c_int {
    // Sized as the C driver sizes it: the deepest MIB the interface defines, plus the two the
    // name lookup itself prepends.
    let mut mib = [0 as c_int; CTL_MAXNAME + 2];
    let mut mib_len = mib.len();

    // SAFETY: the caller guarantees a null-terminated string, and `mib`/`mib_len` are this
    // function's own storage.
    if unsafe { __nx_sys_net__sysctlnametomib(name, mib.as_mut_ptr(), &raw mut mib_len) } != 0 {
        return -1;
    }

    // The lookup reports how many components it wrote, which is what names the parameter.
    let Ok(namelen) = c_uint::try_from(mib_len) else {
        return errno::fail(errno::EINVAL);
    };

    // SAFETY: `mib` holds `mib_len` components the lookup just wrote, and the caller's buffers are
    // forwarded exactly as they were given.
    unsafe { __nx_sys_net__sysctl(mib.as_ptr(), namelen, oldp, oldlenp, newp, newlen) }
}

/// Resolves a parameter name to the MIB that names it.
///
/// The lookup is itself a parameter: `{0, 3}` is `sysctl.name2oid`, which takes the name as the
/// value being written and answers with the components. So this is one [`__nx_sys_net__sysctl`]
/// call rather than a command of its own.
///
/// `sizep` carries the caller's capacity in *components* on the way in and the count written on
/// the way out, while the underlying call counts bytes — hence the four-byte conversion on each
/// side.
///
/// # Safety
///
/// `name` must be a null-terminated string, `mibp` must point to at least `*sizep` writable
/// components, and `sizep` must point to a readable and writable count.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__sysctlnametomib(
    name: *const c_char,
    mibp: *mut c_int,
    sizep: *mut usize,
) -> c_int {
    if name.is_null() || mibp.is_null() || sizep.is_null() {
        return errno::fail(errno::EFAULT);
    }

    /// `sysctl.name2oid`, the parameter that answers with a MIB.
    const NAME_TO_OID: [c_int; 2] = [0, 3];

    // SAFETY: the caller guarantees a readable count at a non-null pointer.
    let capacity_components = unsafe { *sizep };
    let Some(capacity_bytes) = capacity_components.checked_mul(size_of::<c_int>()) else {
        return errno::fail(errno::EINVAL);
    };

    // SAFETY: the caller guarantees a null-terminated string.
    let name_len = unsafe { CStr::from_ptr(name) }.to_bytes().len();

    let mut written_bytes = capacity_bytes;
    // SAFETY: `mibp` has room for `capacity_bytes`, `name` is readable for `name_len`, and the
    // remaining pointers are this function's own storage.
    let rc = unsafe {
        __nx_sys_net__sysctl(
            NAME_TO_OID.as_ptr(),
            NAME_TO_OID.len() as c_uint,
            mibp.cast::<c_void>(),
            &raw mut written_bytes,
            name.cast::<c_void>(),
            name_len,
        )
    };
    if rc != 0 {
        return rc;
    }

    // SAFETY: the caller guarantees a writable count at a non-null pointer.
    unsafe { *sizep = written_bytes / size_of::<c_int>() };

    0
}

/// Largest number of components a MIB can name.
///
/// Fixed by the interface, which is what sizes a caller's own MIB buffer.
const CTL_MAXNAME: usize = 24;
