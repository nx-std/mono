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
