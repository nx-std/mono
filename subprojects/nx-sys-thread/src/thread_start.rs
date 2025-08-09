//! Thread start functionality for Arc+Pin pattern.
//!
//! This module provides safe, idiomatic wrappers around the `svcStartThread`
//! SVC for starting thread execution, updated to work with the new
//! Arc<Pin<ThreadInner>> thread handle pattern.
//!
//! ## Start Operation
//!
//! The [`start`] function maps to `svcStartThread`, which moves a newly-created
//! thread from the `Created` state to `Runnable`, allowing it to be scheduled
//! for the first time. Calling [`start`] on a thread that is already running
//! is harmless and treated as a no-op by the kernel.
//!
//! ## Related Modules
//!
//! For complete thread lifecycle management, see also:
//! - [`crate::thread_pause`] - Temporarily pause thread execution
//! - [`crate::thread_resume`] - Resume paused threads
//! - [`crate::thread_exit`] - Clean thread termination
//!
//! ## Arc+Pin Pattern Benefits
//!
//! This module leverages the Arc+Pin pattern to provide:
//! - **Safe concurrent access** to thread handles across multiple contexts
//! - **Cheap cloning** of thread handles without duplicating kernel resources
//! - **Memory safety** through Rust's ownership system
//! - **Compatibility** with async/await patterns and shared thread management

use nx_svc::{result::Error, thread as svc};

use crate::thread_handle::Thread;

/// Starts execution of the given [`Thread`].
///
/// The target thread must have been created in a suspended state and not
/// started yet. Invoking `start` on a thread that is already running is safe
/// and results in a no-op. Internally this wraps the kernel
/// `svcStartThread` call.
///
/// ## Arc+Pin Pattern
///
/// This function works with the Arc<Pin<ThreadInner>> pattern, allowing
/// the thread handle to be safely shared across multiple contexts while
/// maintaining kernel handle integrity.
///
/// ## Arguments
///
/// * `thread` - Reference to the Thread handle created via [`crate::thread_create::create`]
///
/// ## Returns
///
/// * `Ok(())` - Thread was successfully started or was already running
/// * `Err(ThreadStartError)` - Thread start failed (see error variants for details)
///
/// ## Example Usage
///
/// ```rust,no_run
/// use nx_sys_thread::{thread_create, thread_start};
///
/// // Create a thread (suspended by default)
/// let thread = thread_create::create(
///     my_thread_function,
///     std::ptr::null_mut(),
///     None,
///     0x4000, // 16KB stack
///     0x20,   // Priority
///     -2,     // Default CPU
/// )?;
///
/// // Start the thread
/// thread_start::start(&thread)?;
/// ```
pub fn start(thread: &Thread) -> Result<(), ThreadStartError> {
    svc::start(thread.handle()).map_err(Into::into)
}

/// Error type for [`start`].
#[derive(Debug, thiserror::Error)]
pub enum ThreadStartError {
    /// Supplied handle does not refer to a valid thread.
    ///
    /// This can occur if:
    /// - The thread handle was created incorrectly
    /// - The thread has already exited and been cleaned up
    /// - The handle was corrupted or invalidated
    /// - A kernel resource limit was exceeded
    #[error("Invalid thread handle")]
    InvalidHandle,

    /// Kernel returned an undocumented [`ResultCode`].
    ///
    /// The wrapped [`ResultCode`] is preserved so callers can inspect it
    /// using [`ResultCode::to_rc`] or display it for diagnostics. This
    /// typically indicates either a kernel bug or a rare edge case not
    /// covered by the documented error conditions.
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
