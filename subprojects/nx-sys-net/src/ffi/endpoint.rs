//! Creating sockets, naming them, and connecting them.
//!
//! The calls that establish what a socket *is* and who it talks to, as against the ones that move
//! data through it. Each produces or consumes an address, and none of them interprets one: an
//! address arrives as the bytes the caller supplied and leaves as the bytes the service reported.

use core::ffi::{
    c_int,
    c_void,
};

use nx_service_bsd::Shutdown;
use zerocopy::IntoBytes as _;

use super::{
    abi::{
        SockLenT,
        borrow_sockaddr,
        write_sockaddr,
    },
    descriptor::{
        to_c_fd,
        with_socket,
    },
    errno,
};
use crate::{
    device,
    session,
    socket::Socket,
};

/// Creates a socket.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__socket(domain: c_int, type_: c_int, protocol: c_int) -> c_int {
    create(|svc| svc.socket(domain, type_, protocol))
}

/// Creates a socket exempt from the system's socket accounting.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__socketExempt(
    domain: c_int,
    type_: c_int,
    protocol: c_int,
) -> c_int {
    create(|svc| svc.socket_exempt(domain, type_, protocol))
}

/// Assigns a local address to a socket.
///
/// # Safety
///
/// `addr` must be null or point to at least `addr_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__bind(
    sockfd: c_int,
    addr: *const c_void,
    addr_len: SockLenT,
) -> c_int {
    // SAFETY: the caller guarantees `addr_len` readable bytes at `addr`.
    let Some(raw) = (unsafe { borrow_sockaddr(addr, addr_len) }) else {
        return errno::fail(errno::EINVAL);
    };

    match with_socket(sockfd, |svc, sock| svc.bind(sock, &raw)) {
        Ok(()) => 0,
        Err(failure) => failure,
    }
}

/// Initiates a connection to a peer.
///
/// # Safety
///
/// `addr` must be null or point to at least `addr_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__connect(
    sockfd: c_int,
    addr: *const c_void,
    addr_len: SockLenT,
) -> c_int {
    // SAFETY: the caller guarantees `addr_len` readable bytes at `addr`.
    let Some(raw) = (unsafe { borrow_sockaddr(addr, addr_len) }) else {
        return errno::fail(errno::EINVAL);
    };

    match with_socket(sockfd, |svc, sock| svc.connect(sock, &raw)) {
        Ok(()) => 0,
        Err(failure) => failure,
    }
}

/// Marks a socket as accepting connections.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__listen(sockfd: c_int, backlog: c_int) -> c_int {
    match with_socket(sockfd, |svc, sock| svc.listen(sock, backlog)) {
        Ok(()) => 0,
        Err(failure) => failure,
    }
}

/// Takes the next connection off a listening socket's queue.
///
/// # Safety
///
/// `addr` must be null or point to at least `*addr_len` writable bytes, and `addr_len` must be
/// null or point to a writable length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__accept(
    sockfd: c_int,
    addr: *mut c_void,
    addr_len: *mut SockLenT,
) -> c_int {
    let (accepted, peer) = match with_socket(sockfd, |svc, sock| svc.accept(sock)) {
        Ok(pair) => pair,
        Err(failure) => return failure,
    };

    // The service has issued the descriptor, so from here on something must own it. Adopting it
    // immediately is what makes every path below release it rather than leak it.
    // SAFETY: `accept` just issued this descriptor and nothing else has taken it on.
    let socket = Socket::from_raw_unchecked(accepted);

    let fd = match device::adopt(socket) {
        Ok(fd) => fd,
        // `adopt` closed the socket, so nothing is left open; the connection is dropped, which is
        // what a caller with no descriptors to give it would have had to do anyway.
        Err(device::AdoptFailed::NotRegistered) => return errno::fail(errno::EBADF),
        Err(device::AdoptFailed::NoDescriptors) => return errno::fail(errno::EMFILE),
    };

    // SAFETY: the caller guarantees the buffer and length pointers described above.
    unsafe { write_sockaddr(addr, addr_len, &peer) };

    to_c_fd(fd.number())
}

/// Reports the socket's own address.
///
/// # Safety
///
/// As [`__nx_sys_net__accept`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__getsockname(
    sockfd: c_int,
    addr: *mut c_void,
    addr_len: *mut SockLenT,
) -> c_int {
    match with_socket(sockfd, |svc, sock| svc.get_sock_name(sock)) {
        // SAFETY: the caller guarantees the buffer and length pointers.
        Ok(reported) => {
            unsafe { write_sockaddr(addr, addr_len, &reported) };
            0
        }
        Err(failure) => failure,
    }
}

/// Reports the connected peer's address.
///
/// # Safety
///
/// As [`__nx_sys_net__accept`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__getpeername(
    sockfd: c_int,
    addr: *mut c_void,
    addr_len: *mut SockLenT,
) -> c_int {
    match with_socket(sockfd, |svc, sock| svc.get_peer_name(sock)) {
        // SAFETY: the caller guarantees the buffer and length pointers.
        Ok(reported) => {
            unsafe { write_sockaddr(addr, addr_len, &reported) };
            0
        }
        Err(failure) => failure,
    }
}

/// Disables further sends, receives, or both.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__shutdown(sockfd: c_int, how: c_int) -> c_int {
    /// `SHUT_RD`.
    const SHUT_RD: c_int = 0;
    /// `SHUT_WR`.
    const SHUT_WR: c_int = 1;
    /// `SHUT_RDWR`.
    const SHUT_RDWR: c_int = 2;

    let how = match how {
        SHUT_RD => Shutdown::Read,
        SHUT_WR => Shutdown::Write,
        SHUT_RDWR => Shutdown::Both,
        _ => return errno::fail(errno::EINVAL),
    };

    match with_socket(sockfd, |svc, sock| svc.shutdown(sock, how)) {
        Ok(()) => 0,
        Err(failure) => failure,
    }
}

/// Reports whether the socket is at an out-of-band mark.
///
/// There is no command for this, so it is asked as a device control request: `SIOCATMARK` answers
/// with the flag as its argument, and the answer is what this returns rather than the request's
/// own status.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_sys_net__sockatmark(sockfd: c_int) -> c_int {
    /// `_IOR('s', 7, int)`: read a four-byte answer from group `'s'`, request 7.
    const SIOCATMARK: c_int = 0x4004_7307;

    let mut at_mark: c_int = 0;
    let answer = at_mark.as_mut_bytes();

    match with_socket(sockfd, |svc, sock| svc.ioctl(sock, SIOCATMARK, answer)) {
        Ok(_) => at_mark,
        Err(failure) => failure,
    }
}

/// Creates a connected pair of sockets.
///
/// Not implemented: the service offers no command that creates a pair, and there is no way to
/// build one out of the commands it does offer. The C driver reports the same.
///
/// # Safety
///
/// `sv` is not dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_net__socketpair(
    domain: c_int,
    type_: c_int,
    protocol: c_int,
    sv: *mut c_int,
) -> c_int {
    let _ = (domain, type_, protocol, sv);
    errno::fail(errno::ENOSYS)
}

/// Runs a command that creates a socket, and gives the result a process descriptor.
///
/// The shared half of `socket` and its exempt variant: both differ only in which command they
/// send, and everything after — adopting the descriptor, handing it to the table, reporting the
/// number — is the same.
fn create(
    op: impl FnOnce(
        &nx_service_bsd::BsdService,
    ) -> Result<nx_service_bsd::SocketFd, nx_service_bsd::CommandError>,
) -> c_int {
    let created = match session::with_service(op) {
        Err(_) => return errno::fail(errno::EBADF),
        Ok(Err(err)) => return errno::report(err),
        Ok(Ok(sock)) => sock,
    };

    // SAFETY: the command just issued this descriptor and nothing else has taken it on.
    let socket = Socket::from_raw_unchecked(created);

    match device::adopt(socket) {
        Ok(fd) => to_c_fd(fd.number()),
        Err(device::AdoptFailed::NotRegistered) => errno::fail(errno::EBADF),
        Err(device::AdoptFailed::NoDescriptors) => errno::fail(errno::EMFILE),
    }
}
