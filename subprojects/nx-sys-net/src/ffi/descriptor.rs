//! Turning a C descriptor into a socket the service will answer for.
//!
//! Every export in this module tree starts the same way: it is handed a bare `int` and has to
//! decide whether that number names one of this process's sockets. Three things can be wrong with
//! it — it can be outside the descriptor table, it can name something that is not a socket, and
//! the driver may never have been initialized — and C has a different error number for each.
//!
//! [`resolve`] is that step, written once. What it returns is the service's own descriptor, copied
//! out from under the file lock so that the command using it runs unlocked; see
//! [`crate::device`] for why that matters.
//!
//! [`to_c_fd`] and [`adopt_reported`] are the same step in the other direction, for the exports
//! that produce a descriptor rather than consume one.

use core::ffi::c_int;

use nx_service_bsd::BsdSockFd;
use nx_sys_fd::table::Fd;

use super::errno;
use crate::device::{
    self,
    LookupError,
};

/// Resolves a C descriptor to the socket it names.
///
/// # Errors
///
/// Returns the error number C reports for the failure, so a caller does nothing but hand it to
/// [`errno::fail`]: `EBADF` for a number that names nothing, and `ENOTSOCK` for one that names
/// something other than a socket.
pub fn resolve(fd: c_int) -> Result<BsdSockFd, c_int> {
    let Ok(number) = usize::try_from(fd) else {
        return Err(errno::EBADF);
    };
    let Ok(fd) = Fd::try_from(number) else {
        return Err(errno::EBADF);
    };

    device::sock_of(fd).map_err(|err| match err {
        LookupError::BadDescriptor => errno::EBADF,
        LookupError::NotASocket => errno::ENOTSOCK,
    })
}

/// Runs `op` against the socket `fd` names, reporting every failure the C way.
///
/// The shape almost every export has: resolve the descriptor, run one command, and turn the answer
/// into a C return value. `op` produces the success value; a failure at any stage becomes `-1`
/// with the reason in `errno`.
pub fn with_socket<T>(
    fd: c_int,
    op: impl FnOnce(&nx_service_bsd::BsdService, BsdSockFd) -> Result<T, nx_service_bsd::CommandError>,
) -> Result<T, c_int> {
    let sock = resolve(fd).map_err(errno::fail)?;

    match crate::session::with_service(|svc| op(svc, sock)) {
        Err(_) => Err(errno::fail(errno::EBADF)),
        Ok(Err(err)) => Err(errno::report(err)),
        Ok(Ok(value)) => Ok(value),
    }
}

/// Reports a descriptor number as C's `int`.
///
/// The table's numbers are far below what an `int` holds, so the conversion cannot fail; it is
/// written as a fallible one anyway because a silent wrap here would hand back a descriptor that
/// names something else.
pub fn to_c_fd(number: usize) -> c_int {
    match c_int::try_from(number) {
        Ok(fd) => fd,
        Err(_) => errno::fail(errno::EMFILE),
    }
}

/// Gives a socket another service reported a process descriptor, reporting failure the C way.
///
/// The calls that hand a socket to the TLS stack answer with the descriptor they displaced, and
/// that socket is still open and still owed a close, so it needs a descriptor of its own. It
/// arrives as a bare number from a service this crate does not speak to, which is what separates
/// this from the adopt every other export does.
///
/// The descriptor arrives as a bare number rather than a type, and that is forced rather than
/// chosen: the crate that reported it carries its own descriptor newtype, and a conversion from
/// that to [`BsdSockFd`] would have to be written in `nx-service-bsd`, which cannot depend on a
/// service crate that already depends on it. So the number crosses this one seam untyped, and is
/// a [`BsdSockFd`] again on the far side of it.
///
/// A negative `raw_fd` says the command displaced nothing. That is not a failure of the command,
/// but there is no descriptor to return either, so it is reported as one: `ENOENT`, as the C
/// driver answers, and its comment there notes the caller is meant to ignore it.
pub fn adopt_reported(raw_fd: i32) -> c_int {
    if raw_fd < 0 {
        return errno::fail(errno::ENOENT);
    }

    // SAFETY: `adopt_raw_unchecked` requires a descriptor the BSD service issued that nothing else
    // will close. The caller read it out of a hand-off command's response, which is where the
    // service reports the socket it gave up rather than closed, and the negative sentinel for
    // "gave up nothing" is ruled out above.
    match device::adopt_raw_unchecked(raw_fd) {
        Ok(fd) => to_c_fd(fd.number()),
        Err(device::AdoptFailed::NotRegistered) => errno::fail(errno::EBADF),
        Err(device::AdoptFailed::NoDescriptors) => errno::fail(errno::EMFILE),
    }
}
