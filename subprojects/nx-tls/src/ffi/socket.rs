//! Giving a socket to a TLS connection, from C.
//!
//! Upstream splits these across two files. `services/ssl.c` holds the commands that take a socket
//! descriptor the BSD service issued, and `runtime/devices/socket.c` holds the wrappers a program
//! actually calls, which take a descriptor from the *process's* table, translate it, and delegate.
//! These are the wrappers, and the translation is the whole of what they add.
//!
//! It is why they cannot live in either neighbour: [`nx_service_ssl`] does not know the descriptor
//! table exists, and [`nx_sys_net`] does not know what a TLS connection is. This crate knows both.
//!
//! The commands they delegate to are in [`super::connection`], with the rest of the connection's
//! surface. These are separate because they answer in `errno` rather than a result code: they are
//! socket calls, and a program reaching them is in the middle of socket code.

use core::ffi::{
    c_int,
    c_void,
};

use nx_sf::error::{
    LibnxError,
    ToResultCode as _,
    libnx_error,
};
use nx_sys_net::ffi::{
    abi as net_abi,
    abi::SockLenT,
    descriptor,
    errno,
};

use super::{
    firmware,
    object,
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
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return errno::report_result(libnx_error(LibnxError::NotInitialized));
    };

    report(connection.set_socket_descriptor(sock))
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
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return errno::report_result(libnx_error(LibnxError::NotInitialized));
    };

    report(connection.get_socket_descriptor())
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
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return errno::report_result(libnx_error(LibnxError::NotInitialized));
    };

    if !firmware::offers_dtls() {
        return errno::report_result(libnx_error(LibnxError::IncompatSysVer));
    }

    // SAFETY: the caller guarantees `addr_len` readable bytes at `addr`, which is this function's
    // own precondition; `borrow_sockaddr` handles the null pointer itself.
    let Some(addr) = (unsafe { net_abi::borrow_sockaddr(addr, addr_len) }) else {
        return errno::fail(errno::EINVAL);
    };

    report(connection.set_dtls_socket_descriptor(sock, &addr))
}

/// Reports the descriptor a hand-off displaced, in the process's own numbering.
///
/// The three entry points above answer the same three ways, so the mapping is written once. A
/// connection that held no socket reports a negative sentinel rather than a descriptor, which
/// becomes `ENOENT`: there was nothing to give back.
fn report(
    outcome: Result<Option<nx_service_ssl::SocketFd>, nx_sf::service::DispatchError>,
) -> c_int {
    match outcome {
        Ok(Some(displaced)) => descriptor::adopt_reported(displaced.to_raw()),
        Ok(None) => errno::fail(errno::ENOENT),
        Err(err) => errno::report_result(err.to_rc()),
    }
}
