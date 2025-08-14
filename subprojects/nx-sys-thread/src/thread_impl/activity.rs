//! Thread activity utilities.
//!
//! This module contains safe, idiomatic wrappers around the raw SVCs that
//! control a thread's scheduling state on Horizon OS.
//!
//! The three public entry-points — [`start`], [`pause`] and [`resume`] — map
//! to the following kernel operations:
//! * [`start`] → `svcStartThread` – moves a newly-created thread from the
//!   `Created` state to `Runnable`, allowing it to be scheduled for the first
//!   time.
//! * [`pause`] → `svcSetThreadActivity` with activity `Paused` – stops
//!   scheduling of an already-running thread until it is resumed.
//! * [`resume`] → `svcSetThreadActivity` with activity `Runnable` – resumes a
//!   previously paused thread.
//!
//! Compared to the C bindings exposed by libnx, these Rust versions return
//! high-level, Rust-friendly error types ([`ThreadStartError`],
//! [`ThreadPauseError`] and [`ThreadResumeError`]) which internally wrap the
//! lower-level [`nx_svc::thread`] errors. This enables callers to
//! pattern-match on individual variants and handle error conditions in a
//! structured way.
//!
//! Calling [`start`] on a thread that is already running, or [`pause`] on a
//! thread that is already paused (and likewise [`resume`] on a running
//! thread) is harmless and treated as a no-op by the kernel.

use nx_svc::{result::Error, thread as svc};

use super::handle::Thread;

/// Starts execution of the given [`Thread`].
///
/// The target thread must have been created in a suspended state and not
/// started yet. Invoking `start` on a thread that is already running is safe
/// and results in a no-op. Internally this wraps the kernel
/// `svcStartThread` call.
pub fn start(thread: &Thread) -> Result<(), ThreadStartError> {
    svc::start(thread.handle).map_err(Into::into)
}

/// Error type for [`start`].
#[derive(Debug, thiserror::Error)]
pub enum ThreadStartError {
    /// Supplied handle does not refer to a valid thread.
    #[error("Invalid thread handle")]
    InvalidHandle,

    /// Kernel returned an undocumented [`ResultCode`].
    ///
    /// The wrapped [`ResultCode`] is preserved so callers can inspect it
    /// using [`ResultCode::to_rc`] or display it for diagnostics.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl From<svc::StartThreadError> for ThreadStartError {
    fn from(value: svc::StartThreadError) -> Self {
        match value {
            svc::StartThreadError::InvalidHandle => ThreadStartError::InvalidHandle,
            svc::StartThreadError::Unknown(err) => ThreadStartError::Unknown(err),
        }
    }
}

#[cfg(feature = "ffi")]
impl nx_svc::error::ToRawResultCode for ThreadStartError {
    fn to_rc(self) -> nx_svc::error::ResultCode {
        match self {
            Self::InvalidHandle => svc::StartThreadError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_rc(),
        }
    }
}

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
    Unknown(Error),
}

impl From<svc::PauseThreadError> for ThreadPauseError {
    fn from(value: svc::PauseThreadError) -> Self {
        match value {
            svc::PauseThreadError::InvalidHandle => ThreadPauseError::InvalidHandle,
            svc::PauseThreadError::Unknown(err) => ThreadPauseError::Unknown(err),
        }
    }
}

#[cfg(feature = "ffi")]
impl nx_svc::error::ToRawResultCode for ThreadPauseError {
    fn to_rc(self) -> nx_svc::error::ResultCode {
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
    Unknown(Error),
}

impl From<svc::ResumeThreadError> for ThreadResumeError {
    fn from(value: svc::ResumeThreadError) -> Self {
        match value {
            svc::ResumeThreadError::InvalidHandle => ThreadResumeError::InvalidHandle,
            svc::ResumeThreadError::Unknown(err) => ThreadResumeError::Unknown(err),
        }
    }
}

#[cfg(feature = "ffi")]
impl nx_svc::error::ToRawResultCode for ThreadResumeError {
    fn to_rc(self) -> nx_svc::error::ResultCode {
        match self {
            Self::InvalidHandle => svc::ResumeThreadError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_rc(),
        }
    }
}
