//! What a command answers with.
//!
//! The C surface reports success and failure as a Horizon result code, so every entry point ends
//! by producing one. Naming the three failures a boundary can raise on its own keeps the version
//! gates and the argument checks reading as what they mean rather than as constants.
//!
//! A failure the *service* raised is not here: it arrives as a [`DispatchError`] carrying the code
//! the server chose, and [`report`] forwards it untouched.

use nx_sf::{
    error::{
        LibnxError,
        ResultCode,
        ToResultCode as _,
        libnx_error,
    },
    service::DispatchError,
};

/// What a command that worked reports.
pub(super) const OK: ResultCode = 0;

/// What every entry point reports when the service, the context or the connection is not there.
pub(super) fn not_initialized() -> ResultCode {
    libnx_error(LibnxError::NotInitialized)
}

/// What a command reports when the running firmware does not implement it.
pub(super) fn incompat_sys_ver() -> ResultCode {
    libnx_error(LibnxError::IncompatSysVer)
}

/// What an entry point reports when an argument fails validation at the boundary.
pub(super) fn bad_input() -> ResultCode {
    libnx_error(LibnxError::BadInput)
}

/// Reports the outcome of a command that answers with nothing but success or failure.
pub(super) fn report(outcome: Result<(), DispatchError>) -> ResultCode {
    match outcome {
        Ok(()) => OK,
        Err(err) => err.to_rc(),
    }
}
