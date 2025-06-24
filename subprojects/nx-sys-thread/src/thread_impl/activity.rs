//! Thread activity utilities.
//!
//! This module contains safe, idiomatic wrappers around the raw SVCs that
//! control a thread's scheduling state on Horizon OS. The two public
//! entry-points, [`pause`] and [`resume`], correspond to the
//! kernel operation `svcSetThreadActivity` with the activity set to
//! `Paused` or `Runnable` respectively.
//!
//! Compared to the C bindings exposed by libnx, these Rust versions return
//! high-level, Rust-friendly error types ([`ThreadPauseError`] and
//! [`ThreadResumeError`]) which internally wrap the lower-level
//! [`nx_svc::thread`] errors.  This enables callers to pattern-match on
//! individual variants and handle error conditions in a structured way.
//!
//! Calling [`pause`] on a thread that is already paused (or
//! [`resume`] on an already-running thread) is harmless and treated
//! as a no-op by the kernel.

#[cfg(feature = "ffi")]
use nx_svc::error::ToRawResultCode;
use nx_svc::{error::ResultCode, thread as svc};

use super::info::Thread;

/// Temporarily pauses the scheduler for the given [`Thread`].
///
/// The kernel will stop running the target thread until [`resume`] is
/// invoked.  Pausing a thread that is already paused is allowed but has no
/// effect.
pub fn pause(thread: &Thread) -> Result<(), ThreadPauseError> {
    svc::pause(thread.handle).map_err(Into::into)
}

/// Error type for [`pause`].
#[derive(Debug, thiserror::Error)]
pub enum ThreadPauseError {
    /// Supplied handle does not refer to a valid thread.
    ///
    /// Mirrors [`svc::PauseThreadError::InvalidHandle`].
    #[error("Invalid thread handle")]
    InvalidHandle,

    /// Kernel returned an undocumented [`ResultCode`].
    ///
    /// The wrapped [`ResultCode`] is preserved so callers can inspect it
    /// using [`ResultCode::to_rc`] or display it for diagnostics.
    #[error("Unknown error: {0}")]
    Unknown(ResultCode),
}

impl From<svc::PauseThreadError> for ThreadPauseError {
    fn from(value: svc::PauseThreadError) -> Self {
        match value {
            svc::PauseThreadError::InvalidHandle => ThreadPauseError::InvalidHandle,
            svc::PauseThreadError::Unknown(err) => ThreadPauseError::Unknown(err.to_raw()),
        }
    }
}

#[cfg(feature = "ffi")]
impl ToRawResultCode for ThreadPauseError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => svc::PauseThreadError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_rc(),
        }
    }
}

/// Resumes execution of a previously paused [`Thread`].
pub fn resume(thread: &Thread) -> Result<(), ThreadResumeError> {
    svc::resume(thread.handle).map_err(Into::into)
}

/// Error type for [`resume`].
#[derive(Debug, thiserror::Error)]
pub enum ThreadResumeError {
    /// Supplied handle does not refer to a valid thread.
    ///
    /// Mirrors [`svc::ResumeThreadError::InvalidHandle`].
    #[error("Invalid thread handle")]
    InvalidHandle,

    /// Kernel returned an undocumented [`ResultCode`].
    ///
    /// The wrapped [`ResultCode`] is preserved so callers can inspect it
    /// using [`ResultCode::to_rc`] or display it for diagnostics.
    #[error("Unknown error: {0}")]
    Unknown(ResultCode),
}

impl From<svc::ResumeThreadError> for ThreadResumeError {
    fn from(value: svc::ResumeThreadError) -> Self {
        match value {
            svc::ResumeThreadError::InvalidHandle => ThreadResumeError::InvalidHandle,
            svc::ResumeThreadError::Unknown(err) => ThreadResumeError::Unknown(err.to_raw()),
        }
    }
}

#[cfg(feature = "ffi")]
impl ToRawResultCode for ThreadResumeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => svc::ResumeThreadError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_rc(),
        }
    }
}
