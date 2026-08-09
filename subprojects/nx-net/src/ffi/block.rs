//! Single-block result allocator, packer, and deallocator.
//!
//! A C resolver hands its caller result structures it must release with
//! `freeaddrinfo` / `freehostent`. Each `addrinfo` node, and each `hostent`,
//! is allocated as **one** block so a single deallocation releases the struct
//! together with everything it points at. `nx-net` matches that contract.
//!
//! Because the global allocator's `dealloc` needs the original `Layout`, each
//! block is prefixed with a `usize` recording its total size:
//!
//! ```text
//! [ usize block_size ][ result struct ][ struct's pointees… ]
//!                     ^ pointer handed to the C caller
//! ```
//!
//! [`free_block`] steps back over the header, reads the size, and rebuilds the
//! `Layout` to release the whole block. Allocation goes through the `nx-alloc`
//! global allocator with a fixed 8-byte alignment, which satisfies every
//! `repr(C)` type packed here.

use alloc::alloc::{
    alloc,
    dealloc,
};
use core::{
    alloc::Layout,
    ffi::{
        c_char,
        c_int,
    },
    mem::{
        align_of,
        size_of,
    },
    net::SocketAddr,
    ptr,
};

use static_assertions::const_assert;

use crate::{
    ffi::abi::{
        AF_INET,
        AF_INET6,
        addrinfo,
        hostent,
        in_addr,
        in6_addr,
        sockaddr,
        sockaddr_in,
        sockaddr_in6,
        sockaddr_storage,
    },
    resolve::resolver::{
        HostEntry,
        ResolvedAddr,
    },
};

/// Fixed alignment of every result block.
///
/// Eight bytes satisfies the size header and every `repr(C)` result type on
/// the `aarch64` target — `addrinfo`, `hostent`, and `sockaddr_storage` are
/// all 8-byte aligned, as are the `*mut c_char` arrays a `hostent` carries.
const ALIGN: usize = 8;

/// Size, in bytes, of the leading block-size header.
const HEADER: usize = size_of::<usize>();

/// Length, in bytes, of an IPv4 address — the `h_length` of every `hostent`.
const IPV4_LEN: usize = 4;

const_assert!(align_of::<addrinfo>() <= ALIGN);
const_assert!(align_of::<hostent>() <= ALIGN);
const_assert!(align_of::<sockaddr_storage>() <= ALIGN);

/// Allocates a result block with room for `payload` bytes past its header.
///
/// Returns a pointer to the first payload byte, or null on allocation
/// failure. The header records the *total* allocation size so [`free_block`]
/// can reconstruct the `Layout`.
fn alloc_block(payload: usize) -> *mut u8 {
    let total = HEADER + payload;
    let Ok(layout) = Layout::from_size_align(total, ALIGN) else {
        return ptr::null_mut();
    };

    // SAFETY: `layout` has a non-zero size — `HEADER` alone is non-zero.
    let base = unsafe { alloc(layout) };
    if base.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: `base` owns `total` writable bytes, 8-aligned, so the leading
    // `usize` header is in bounds and aligned.
    unsafe { base.cast::<usize>().write(total) };
    // SAFETY: the payload begins `HEADER` bytes into the `total`-byte block.
    unsafe { base.add(HEADER) }
}

/// Releases a result block previously produced by this module.
///
/// # Safety
///
/// `ptr` must be a non-null pointer returned by [`alloc_addrinfo_node`] or
/// [`alloc_hostent_block`] that has not already been freed.
pub unsafe fn free_block(ptr: *mut u8) {
    // SAFETY: by contract `ptr` is `HEADER` bytes past the block base.
    let base = unsafe { ptr.sub(HEADER) };
    // SAFETY: `base` addresses the `usize` size header `alloc_block` wrote.
    let total = unsafe { base.cast::<usize>().read() };

    let Ok(layout) = Layout::from_size_align(total, ALIGN) else {
        return;
    };
    // SAFETY: `base` and `layout` reproduce the original allocation exactly.
    unsafe { dealloc(base, layout) };
}

/// Allocates and packs a single `addrinfo` result node.
///
/// The `addrinfo` struct, a `sockaddr_storage` holding its address, and the
/// canonical-name string share one block. `ai_next` is left null — the caller
/// links nodes into a chain. Returns null on allocation failure.
pub fn alloc_addrinfo_node(record: &ResolvedAddr) -> *mut addrinfo {
    let canon = record.canonname();
    // Reserve the canonical name plus its NUL terminator; a node without a
    // canonical name still reserves the terminator byte so every block has
    // the same fixed shape.
    let canon_len = canon.map_or(0, str::len) + 1;
    let payload = size_of::<addrinfo>() + size_of::<sockaddr_storage>() + canon_len;

    let block = alloc_block(payload);
    if block.is_null() {
        return ptr::null_mut();
    }

    // The three regions are laid out back-to-back in the payload.
    let node = block.cast::<addrinfo>();
    // SAFETY: `block` owns `payload` bytes; the storage region starts right
    // after the `addrinfo` struct and is fully in bounds.
    let storage = unsafe { block.add(size_of::<addrinfo>()) }.cast::<sockaddr_storage>();
    // SAFETY: the canonical-name region follows the storage region in bounds.
    let canon_ptr = unsafe { block.add(size_of::<addrinfo>() + size_of::<sockaddr_storage>()) }
        .cast::<c_char>();

    // Pack the socket address into the storage region, if the record has one.
    let addr = record.socket_addr();
    let ai_addrlen = match addr {
        // SAFETY: `storage` addresses a full `sockaddr_storage`, large enough
        // for either concrete `sockaddr`.
        Some(sa) => unsafe { write_sockaddr(storage, &sa) },
        None => 0,
    };

    // Pack the canonical name as a NUL-terminated C string.
    match canon {
        Some(name) => {
            // SAFETY: `canon_ptr` begins a `canon_len`-byte region and
            // `canon_len == name.len() + 1`, so the bytes and terminator fit.
            unsafe {
                ptr::copy_nonoverlapping(name.as_ptr().cast::<c_char>(), canon_ptr, name.len());
                canon_ptr.add(name.len()).write(0);
            }
        }
        // SAFETY: one byte is always reserved for the terminator.
        None => unsafe { canon_ptr.write(0) },
    }

    let info = addrinfo {
        ai_flags: record.flags(),
        ai_family: record.family(),
        ai_socktype: record.socktype(),
        ai_protocol: record.protocol(),
        ai_addrlen,
        ai_canonname: if canon.is_some() {
            canon_ptr
        } else {
            ptr::null_mut()
        },
        ai_addr: if addr.is_some() {
            storage.cast::<sockaddr>()
        } else {
            ptr::null_mut()
        },
        ai_next: ptr::null_mut(),
    };
    // SAFETY: `node` addresses an 8-aligned, `addrinfo`-sized region.
    unsafe { node.write(info) };
    node
}

/// Allocates and packs a `hostent` result block.
///
/// The `hostent` struct, its NULL-terminated alias and address pointer
/// arrays, the address bytes, and every name string share one block. `nx-net`
/// resolves IPv4 host records only, so `h_addrtype` is `AF_INET` and
/// `h_length` is `4`. Returns null on allocation failure.
pub fn alloc_hostent_block(entry: &HostEntry) -> *mut hostent {
    let name = entry.name();
    let aliases = entry.aliases();
    let addresses = entry.addresses();

    let ptr_size = size_of::<*mut c_char>();
    // Both pointer arrays are NULL-terminated, hence the extra slot each.
    let alias_array = (aliases.len() + 1) * ptr_size;
    let addr_array = (addresses.len() + 1) * ptr_size;
    let addr_data = addresses.len() * IPV4_LEN;
    let name_len = name.len() + 1;
    let alias_bytes: usize = aliases.iter().map(|alias| alias.len() + 1).sum();

    let payload =
        size_of::<hostent>() + alias_array + addr_array + addr_data + name_len + alias_bytes;
    let block = alloc_block(payload);
    if block.is_null() {
        return ptr::null_mut();
    }

    // Region offsets within the payload, laid out back-to-back.
    let alias_off = size_of::<hostent>();
    let addr_arr_off = alias_off + alias_array;
    let addr_data_off = addr_arr_off + addr_array;
    let name_off = addr_data_off + addr_data;
    let alias_str_off = name_off + name_len;

    // SAFETY: every offset below is in bounds of the `payload`-byte block,
    // and the pointer arrays start at 8-aligned offsets (the `hostent` struct
    // and each preceding array are multiples of 8 bytes).
    let h_aliases = unsafe { block.add(alias_off) }.cast::<*mut c_char>();
    let h_addr_list = unsafe { block.add(addr_arr_off) }.cast::<*mut c_char>();

    // Write the official host name.
    // SAFETY: the name region holds `name_len == name.len() + 1` bytes.
    let name_ptr = unsafe { block.add(name_off) }.cast::<c_char>();
    unsafe {
        ptr::copy_nonoverlapping(name.as_ptr().cast::<c_char>(), name_ptr, name.len());
        name_ptr.add(name.len()).write(0);
    }

    // Write each alias string and record its pointer in the alias array.
    let mut cursor = alias_str_off;
    for (i, alias) in aliases.iter().enumerate() {
        // SAFETY: `cursor` walks the alias-string region exactly, advancing
        // by `alias.len() + 1` per entry; `i` indexes within the array.
        let string = unsafe { block.add(cursor) }.cast::<c_char>();
        unsafe {
            ptr::copy_nonoverlapping(alias.as_ptr().cast::<c_char>(), string, alias.len());
            string.add(alias.len()).write(0);
            h_aliases.add(i).write(string);
        }
        cursor += alias.len() + 1;
    }
    // SAFETY: the alias array has one slot past the entries for the terminator.
    unsafe { h_aliases.add(aliases.len()).write(ptr::null_mut()) };

    // Write each address blob and record its pointer in the address array.
    for (i, address) in addresses.iter().enumerate() {
        // SAFETY: the address-data region holds `addresses.len()` IPv4 blobs.
        let data = unsafe { block.add(addr_data_off + i * IPV4_LEN) }.cast::<c_char>();
        let octets = address.octets();
        unsafe {
            ptr::copy_nonoverlapping(octets.as_ptr().cast::<c_char>(), data, IPV4_LEN);
            h_addr_list.add(i).write(data);
        }
    }
    // SAFETY: the address array has one slot past the entries for the terminator.
    unsafe { h_addr_list.add(addresses.len()).write(ptr::null_mut()) };

    let host = hostent {
        h_name: name_ptr,
        h_aliases,
        h_addrtype: AF_INET,
        h_length: IPV4_LEN as c_int,
        h_addr_list,
    };
    // SAFETY: `block` addresses an 8-aligned, `hostent`-sized region.
    unsafe { block.cast::<hostent>().write(host) };
    block.cast::<hostent>()
}

/// Writes `addr` into a `sockaddr_storage` region as the matching concrete
/// `sockaddr_in` / `sockaddr_in6`, returning its length in bytes.
///
/// The port — and, for IPv4, the address — is stored in network byte order,
/// matching the BSD `sockaddr` contract.
///
/// # Safety
///
/// `storage` must address at least `size_of::<sockaddr_storage>()` writable,
/// 8-aligned bytes.
unsafe fn write_sockaddr(storage: *mut sockaddr_storage, addr: &SocketAddr) -> u32 {
    match addr {
        SocketAddr::V4(v4) => {
            let value = sockaddr_in {
                sin_len: size_of::<sockaddr_in>() as u8,
                sin_family: AF_INET as u8,
                sin_port: v4.port().to_be(),
                sin_addr: in_addr {
                    s_addr: u32::from_ne_bytes(v4.ip().octets()),
                },
                sin_zero: [0; 8],
            };
            // SAFETY: a `sockaddr_in` fits within the `sockaddr_storage`.
            unsafe { storage.cast::<sockaddr_in>().write(value) };
            size_of::<sockaddr_in>() as u32
        }
        SocketAddr::V6(v6) => {
            let value = sockaddr_in6 {
                sin6_family: AF_INET6 as u8,
                sin6_port: v6.port().to_be(),
                sin6_flowinfo: v6.flowinfo(),
                sin6_addr: in6_addr {
                    s6_addr: v6.ip().octets(),
                },
                sin6_scope_id: v6.scope_id(),
            };
            // SAFETY: a `sockaddr_in6` fits within the `sockaddr_storage`.
            unsafe { storage.cast::<sockaddr_in6>().write(value) };
            size_of::<sockaddr_in6>() as u32
        }
    }
}
