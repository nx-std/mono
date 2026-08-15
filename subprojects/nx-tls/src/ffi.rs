//! Giving a socket to a TLS connection, from C.
//!
//! Upstream splits these across two files. `services/ssl.c` holds the commands that take a socket
//! descriptor the BSD service issued, and `runtime/devices/socket.c` holds the wrappers a program
//! actually calls, which take a descriptor from the *process's* table, translate it, and delegate.
//! These are the wrappers.
//!
//! The translation is the whole of what they add, and it is why they cannot live in either
//! neighbour: [`nx_service_ssl`] does not know the descriptor table exists, and [`nx_sys_net`] does
//! not know what a TLS connection is. This crate knows both.
//!
//! ## Reading a connection the C side owns
//!
//! Each takes a pointer to a libnx `SslConnection`, whose first member is the service struct
//! [`nx_sf::ffi::Service`] mirrors. [`nx_sf::ffi::Service::as_domain_object`] addresses what it
//! names without adopting it: the C caller created the connection and closes it, and nothing here
//! may do either.
//!
//! A struct naming no object is one the C side never converted to a domain. libnx tolerates that,
//! because its own conversion is allowed to fail, and dispatches on the plain session instead. This
//! does not: [`nx_service_ssl`] models the interface as a domain object, so there is nothing to
//! send a command through, and the call reports the failure an inactive service would rather than
//! guessing.
//!
//! ## The firmware gate stays here
//!
//! `SetDtlsSocketDescriptor` arrived in `[16.0.0]` and does not exist below it. This crate sits
//! above `nx-rt-core`, so it reads the running firmware itself and answers the way libnx does,
//! rather than pushing the symbol up to the runtime to be near a version.

use core::ffi::{
    c_int,
    c_void,
};

use nx_service_ssl::ffi::ForeignSslConnection;
use nx_sf::{
    error::{
        LibnxError,
        ToResultCode as _,
        libnx_error,
    },
    ffi::Service,
    service::ForeignDomainObject,
};
use nx_sys_net::ffi::{
    abi::{
        self,
        SockLenT,
    },
    descriptor,
    errno,
};

/// Hands a socket descriptor to a TLS connection.
///
/// Returns a process descriptor for the socket the connection gave up, which is what the command
/// answers with.
///
/// # Safety
///
/// `connection` must point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__socketSslConnectionSetSocketDescriptor(
    connection: *mut c_void,
    sockfd: c_int,
) -> c_int {
    // The descriptor is resolved first, as the C driver does it: it calls `_socketGetFd` before
    // the service function that tests the connection.
    let sock = match descriptor::resolve(sockfd) {
        Ok(sock) => sock,
        Err(number) => return errno::fail(number),
    };

    // SAFETY: the caller guarantees a readable `SslConnection` at `connection`, whose first member
    // is the service struct this reads.
    let Some(object) = (unsafe { domain_object_at(connection) }) else {
        return errno::report_result(libnx_error(LibnxError::NotInitialized));
    };

    match ForeignSslConnection::new(object).set_socket_descriptor(sock) {
        Ok(Some(displaced)) => descriptor::adopt_reported(displaced.to_raw()),
        Ok(None) => errno::fail(errno::ENOENT),
        Err(err) => errno::report_result(err.to_rc()),
    }
}

/// Takes a socket descriptor back from a TLS connection.
///
/// # Safety
///
/// `connection` must point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__socketSslConnectionGetSocketDescriptor(
    connection: *mut c_void,
) -> c_int {
    // SAFETY: as the set counterpart above: the caller guarantees a readable `SslConnection` at
    // `connection`, and the service struct is its first member.
    let Some(object) = (unsafe { domain_object_at(connection) }) else {
        return errno::report_result(libnx_error(LibnxError::NotInitialized));
    };

    match ForeignSslConnection::new(object).get_socket_descriptor() {
        Ok(Some(reported)) => descriptor::adopt_reported(reported.to_raw()),
        Ok(None) => errno::fail(errno::ENOENT),
        Err(err) => errno::report_result(err.to_rc()),
    }
}

/// Hands a datagram socket descriptor to a TLS connection.
///
/// Returns a process descriptor for the socket the connection gave up, as the set counterpart
/// does.
///
/// # Safety
///
/// `connection` must point to a readable libnx `SslConnection`, and `addr` must point to
/// `addr_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__socketSslConnectionSetDtlsSocketDescriptor(
    connection: *mut c_void,
    sockfd: c_int,
    addr: *const c_void,
    addr_len: SockLenT,
) -> c_int {
    // The three guards run in the order the C driver applies them: it resolves the descriptor
    // before calling the service function, and that function tests the connection before the
    // firmware. A caller that gets two of them wrong must be told about the same one either
    // implementation would have named.
    let sock = match descriptor::resolve(sockfd) {
        Ok(sock) => sock,
        Err(number) => return errno::fail(number),
    };

    // SAFETY: as the set counterpart above: the caller guarantees a readable `SslConnection` at
    // `connection`, and the service struct is its first member.
    let Some(object) = (unsafe { domain_object_at(connection) }) else {
        return errno::report_result(libnx_error(LibnxError::NotInitialized));
    };

    if !offers_dtls_socket_descriptor() {
        return errno::report_result(libnx_error(LibnxError::IncompatSysVer));
    }

    // SAFETY: the caller guarantees `addr_len` readable bytes at `addr`, which is this function's
    // own precondition; `borrow_sockaddr` handles the null pointer itself.
    let Some(addr) = (unsafe { abi::borrow_sockaddr(addr, addr_len) }) else {
        return errno::fail(errno::EINVAL);
    };

    match ForeignSslConnection::new(object).set_dtls_socket_descriptor(sock, &addr) {
        Ok(Some(displaced)) => descriptor::adopt_reported(displaced.to_raw()),
        Ok(None) => errno::fail(errno::ENOENT),
        Err(err) => errno::report_result(err.to_rc()),
    }
}

/// Whether the running firmware implements `SetDtlsSocketDescriptor`.
///
/// The command arrived in `[16.0.0]` and does not exist below it, so what a caller needs to know is
/// whether it is there rather than which release it is on. This is the one place in the crate a
/// firmware version is compared, and the version is a run-constant the entry crate stored once
/// during startup, so nothing here recomputes something that could have moved.
fn offers_dtls_socket_descriptor() -> bool {
    use nx_rt_core::env::hos_version::{
        self,
        HosVersion,
    };

    hos_version::get() >= HosVersion::new(16, 0, 0)
}

/// Reads the libnx service struct at `ptr` and addresses the connection it names.
///
/// Returns `None` when the struct names no object, which is what a service the C side never
/// converted to a domain looks like.
///
/// # Safety
///
/// `ptr` must be null or point to a readable libnx service struct, which an `SslConnection` begins
/// with, so a pointer to one is a pointer to this.
unsafe fn domain_object_at(ptr: *mut c_void) -> Option<ForeignDomainObject<'static>> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: the caller guarantees a readable service struct at a non-null `ptr`.
    let service = unsafe { *ptr.cast::<Service>() };
    service.as_domain_object()
}
