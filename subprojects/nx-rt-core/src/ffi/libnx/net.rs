//! Name resolution entry points that need the service-manager session.
//!
//! The resolver lives in [`nx_net`], and most of its C symbols are exported from there — the ones
//! that only format a string or free a block need nothing from this crate. These five are the ones
//! that talk to `sfdnsres`, and acquiring that service needs the process's service-manager
//! session, which this crate owns.
//!
//! A process gets one `sm:` session. Opening a second does not get a second session, it fails, so
//! the resolver borrows this one rather than opening its own. That is the same arrangement
//! `socketInitialize` uses, and for the same reason; see [`super::socket`].
//!
//! Each function here is a wrapper and nothing more: the argument parsing, the result blocks and
//! the `h_errno` reporting all stay in [`nx_net`], which is where the resolver's behaviour belongs.

use core::ffi::{
    c_char,
    c_int,
    c_uint,
};

use nx_net::ffi::abi::{
    addrinfo,
    hostent,
    sockaddr,
};

use crate::services::sm;

/// Resolves a node name and/or service into a list of socket addresses.
///
/// # Safety
///
/// As [`nx_net::ffi::getaddrinfo`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_getaddrinfo(
    node: *const c_char,
    service: *const c_char,
    hints: *const addrinfo,
    res: *mut *mut addrinfo,
) -> c_int {
    let guard = sm::sm_session();
    let Some(sm) = guard.as_ref() else {
        return nx_net::ffi::abi::EAI_AGAIN;
    };

    // SAFETY: the pointer contract is this function's own, forwarded unchanged.
    unsafe { nx_net::ffi::getaddrinfo(sm, node, service, hints, res) }
}

/// Resolves a host name into a `hostent`.
///
/// # Safety
///
/// As [`nx_net::ffi::gethostbyname`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_gethostbyname(name: *const c_char) -> *mut hostent {
    let guard = sm::sm_session();
    let Some(sm) = guard.as_ref() else {
        return core::ptr::null_mut();
    };

    // SAFETY: the pointer contract is this function's own, forwarded unchanged.
    unsafe { nx_net::ffi::gethostbyname(sm, name) }
}

/// Resolves a host name in a named address family.
///
/// # Safety
///
/// As [`nx_net::ffi::gethostbyname2`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_gethostbyname2(
    name: *const c_char,
    af: c_int,
) -> *mut hostent {
    let guard = sm::sm_session();
    let Some(sm) = guard.as_ref() else {
        return core::ptr::null_mut();
    };

    // SAFETY: the pointer contract is this function's own, forwarded unchanged.
    unsafe { nx_net::ffi::gethostbyname2(sm, name, af) }
}

/// Resolves an address into a `hostent`.
///
/// # Safety
///
/// As [`nx_net::ffi::gethostbyaddr`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_gethostbyaddr(
    addr: *const core::ffi::c_void,
    len: c_uint,
    af: c_int,
) -> *mut hostent {
    let guard = sm::sm_session();
    let Some(sm) = guard.as_ref() else {
        return core::ptr::null_mut();
    };

    // SAFETY: the pointer contract is this function's own, forwarded unchanged.
    unsafe { nx_net::ffi::gethostbyaddr(sm, addr, len, af) }
}

/// Resolves a socket address into a host and service name.
///
/// # Safety
///
/// As [`nx_net::ffi::getnameinfo`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_getnameinfo(
    addr: *const sockaddr,
    addr_len: u32,
    host: *mut c_char,
    host_len: u32,
    serv: *mut c_char,
    serv_len: u32,
    flags: c_int,
) -> c_int {
    let guard = sm::sm_session();
    let Some(sm) = guard.as_ref() else {
        return nx_net::ffi::abi::EAI_AGAIN;
    };

    // SAFETY: the pointer contract is this function's own, forwarded unchanged.
    unsafe { nx_net::ffi::getnameinfo(sm, addr, addr_len, host, host_len, serv, serv_len, flags) }
}
