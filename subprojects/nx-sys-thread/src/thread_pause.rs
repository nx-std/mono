//! Thread pause functionality for Arc+Pin pattern.
//!
//! This module provides safe, idiomatic wrappers around the `svcSetThreadActivity`
//! SVC for pausing thread execution, updated to work with the new
//! Arc<Pin<ThreadInner>> thread handle pattern.
//!
//! ## Pause Operation
//!
//! The [`pause`] function maps to `svcSetThreadActivity` with activity `Paused`,
//! which stops scheduling of an already-running thread until it is resumed.
//! Pausing a thread that is already paused is harmless and treated as a no-op
//! by the kernel.
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

/// Temporarily pauses the scheduler for the given [`Thread`].
///
/// The kernel will stop running the target thread until it is resumed via
/// [`crate::thread_resume::resume`]. Pausing a thread that is already paused
/// is allowed but has no effect.
///
/// ## Arc+Pin Pattern
///
/// This function works with the Arc<Pin<ThreadInner>> pattern, enabling
/// safe pausing of threads from any context that holds a thread handle
/// clone. This is particularly useful in thread pool management or when
/// implementing cooperative scheduling systems.
///
/// ## Safety Considerations
///
/// - **Deadlock Prevention**: Pausing a thread while it holds critical
///   resources (mutexes, semaphores, etc.) can lead to deadlocks if other
///   threads are waiting for those resources
/// - **Resource Management**: The thread remains paused indefinitely until
///   explicitly resumed - ensure proper cleanup mechanisms are in place
/// - **System Stability**: System threads should not be paused as this may
///   destabilize the process or cause timeouts in system services
/// - **State Preservation**: Thread-local state, stack contents, and register
///   values are preserved across the pause operation
///
/// ## Use Cases
///
/// - **Debugging**: Temporarily halt execution for inspection
/// - **Load Balancing**: Pause threads during high-load periods
/// - **Resource Management**: Temporarily stop threads to free up CPU resources
/// - **Cooperative Scheduling**: Implement custom thread scheduling policies
///
/// ## Arguments
///
/// * `thread` - Reference to the Thread handle to pause
///
/// ## Returns
///
/// * `Ok(())` - Thread was successfully paused or was already paused
/// * `Err(ThreadPauseError)` - Thread pause failed (see error variants for details)
pub fn pause(thread: &Thread) -> Result<(), ThreadPauseError> {
    svc::pause(thread.handle()).map_err(Into::into)
}

/// Error type for [`pause`].
#[derive(Debug, thiserror::Error)]
pub enum ThreadPauseError {
    /// Supplied handle does not refer to a valid thread.
    ///
    /// This mirrors [`svc::PauseThreadError::InvalidHandle`] and can occur if:
    /// - The thread handle is invalid or corrupted during Arc operations
    /// - The target thread has already exited and been cleaned up by the kernel
    /// - Kernel resources have been exhausted (unlikely but possible)
    /// - The handle refers to a different type of kernel object (programming error)
    /// - Memory corruption has affected the thread handle's internal state
    /// - The thread was forcibly terminated by the system
    #[error("Invalid thread handle")]
    InvalidHandle,

    /// Kernel returned an undocumented [`ResultCode`].
    ///
    /// The wrapped [`ResultCode`] is preserved so callers can inspect it
    /// using [`ResultCode::to_rc`] or display it for diagnostics. This
    /// represents an unexpected kernel response not covered by the
    /// documented error conditions.
    ///
    /// This may indicate:
    /// - A kernel bug or inconsistency
    /// - New error conditions introduced in system updates
    /// - Hardware-level issues affecting thread scheduling
    /// - Resource exhaustion at the kernel level
    /// - Rare race conditions in kernel thread management
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
