//! Thread waiting utilities
//!
//! High-level wrappers around the `svcWaitSynchronization` family that make it
//! ergonomic to wait until **another thread** terminates.
//!
//! Under the hood a small retry loop transparently re-issues the
//! `svcWaitSynchronization` if it returns the *Cancelled* kernel result, which
//! mirrors the semantics of libnx's `wait.c::_waitLoop` helper. All remaining
//! kernel errors are mapped to the typed [`WaitForExitError`].

use core::time::Duration;

use nx_svc::{
    result::Error,
    sync::{self, WaitSyncError},
    thread::Handle,
};
use nx_time::Instant;

use super::handle::Thread;

/// Blocks until the supplied thread [`Handle`] becomes *signalled*, i.e. the
/// referenced thread has fully exited and entered the _dead_ state.
pub fn wait_handle_exit(handle: &Handle) -> Result<(), WaitForExitError> {
    wait_handle_exit_with_timeout(handle, Duration::MAX)
}

/// Same as [`wait_handle_exit`] but aborts with [`WaitForExitError::TimedOut`]
/// once `timeout` elapses. Passing `Duration::MAX` disables the upper bound
/// and behaves like an infinite wait.
pub fn wait_handle_exit_with_timeout(
    handle: &Handle,
    timeout: Duration,
) -> Result<(), WaitForExitError> {
    let deadline = if timeout != Duration::MAX {
        Some(Instant::now() + timeout)
    } else {
        None
    };

    loop {
        // Compute the per-iteration timeout (ns). When `deadline` is `None`
        // we forward `u64::MAX` which the kernel interprets as an infinite
        // wait. Otherwise we return the *saturating* difference so that we
        // never panic on underflow.
        let this_timeout_ns = match deadline {
            Some(d) => d.saturating_duration_since(Instant::now()).as_nanos() as u64,
            None => u64::MAX,
        };

        // SAFETY: We forward a single valid handle
        match unsafe { sync::wait_synchronization_single(handle, this_timeout_ns) } {
            Ok(()) => return Ok(()),
            Err(err) => match err {
                WaitSyncError::Cancelled => {
                    // Retry transparently.
                    continue;
                }
                WaitSyncError::TimedOut => {
                    match deadline {
                        Some(_) => return Err(err.into()), // real timeout reached
                        None => continue,                  // infinite wait: retry defensively
                    }
                }
                _ => return Err(err.into()),
            },
        }
    }
}

/// Convenience helper that extracts the underlying handle from a [`Thread`] and
/// forwards it to [`wait_handle_exit`].
#[inline]
pub fn wait_thread_exit(thread: &Thread) -> Result<(), WaitForExitError> {
    wait_handle_exit(&thread.handle)
}

/// Convenience helper that forwards to [`wait_handle_exit_with_timeout`] using
/// the handle stored in `thread`.
#[inline]
pub fn wait_thread_exit_with_timeout(
    thread: &Thread,
    timeout: Duration,
) -> Result<(), WaitForExitError> {
    wait_handle_exit_with_timeout(&thread.handle, timeout)
}

/// Error type returned by the *wait-for-exit* helpers.
#[derive(Debug, thiserror::Error)]
pub enum WaitForExitError {
    /// One (or more) of the supplied handles is invalid.
    #[error("Invalid handle")]
    InvalidHandle,
    /// The wait was cancelled via `svcCancelSynchronization`.
    #[error("Wait cancelled")]
    Cancelled,
    /// The wait operation timed out (should not occur with an infinite timeout
    /// but is kept for completeness).
    #[error("Operation timed out")]
    TimedOut,
    /// Kernel reported that the handle list was out of range (should be
    /// unreachable because we forward exactly one handle).
    #[error("Out of range")]
    OutOfRange,
    /// Any unforeseen kernel error – the wrapped [`Error`] allows callers to
    /// inspect the raw [`ResultCode`].
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl From<WaitSyncError> for WaitForExitError {
    fn from(value: WaitSyncError) -> Self {
        match value {
            WaitSyncError::InvalidHandle => WaitForExitError::InvalidHandle,
            WaitSyncError::Cancelled => WaitForExitError::Cancelled,
            WaitSyncError::TimedOut => WaitForExitError::TimedOut,
            WaitSyncError::OutOfRange => WaitForExitError::OutOfRange,
            WaitSyncError::Unknown(err) => WaitForExitError::Unknown(err),
        }
    }
}

#[cfg(feature = "ffi")]
impl nx_svc::error::ToRawResultCode for WaitForExitError {
    fn to_rc(self) -> nx_svc::result::ResultCode {
        match self {
            WaitForExitError::InvalidHandle => WaitSyncError::InvalidHandle.to_rc(),
            WaitForExitError::Cancelled => WaitSyncError::Cancelled.to_rc(),
            WaitForExitError::TimedOut => WaitSyncError::TimedOut.to_rc(),
            WaitForExitError::OutOfRange => WaitSyncError::OutOfRange.to_rc(),
            WaitForExitError::Unknown(err) => err.to_raw(),
        }
    }
}
