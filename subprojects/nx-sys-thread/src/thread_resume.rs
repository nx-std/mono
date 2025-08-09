//! Thread resume functionality for Arc+Pin pattern.
//!
//! This module provides safe, idiomatic wrappers around the `svcSetThreadActivity`
//! SVC for resuming thread execution, updated to work with the new
//! Arc<Pin<ThreadInner>> thread handle pattern.
//!
//! ## Resume Operation
//!
//! The [`resume`] function maps to `svcSetThreadActivity` with activity `Runnable`,
//! which resumes a previously paused thread. Resuming a thread that is already
//! running is harmless and treated as a no-op by the kernel.
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

/// Resumes execution of a previously paused [`Thread`].
///
/// This function restores normal scheduling for a thread that was previously
/// paused using [`crate::thread_pause::pause`]. Resuming a thread that is
/// already running is allowed but has no effect.
///
/// ## Arc+Pin Pattern
///
/// This function works with the Arc<Pin<ThreadInner>> pattern, allowing
/// thread resumption from any context that holds a thread handle clone.
/// This is particularly useful in scenarios where different parts of the
/// application need to manage thread lifecycle, such as thread pools or
/// work-stealing schedulers.
///
/// ## Threading Notes
///
/// - **State Continuity**: Resumed threads will continue execution from
///   exactly where they were paused, with all registers, stack contents,
///   and thread-local storage preserved
/// - **Priority Preservation**: Thread priority and CPU affinity settings
///   are maintained across pause/resume cycles
/// - **Resource Restoration**: Any kernel resources (handles, memory mappings)
///   associated with the thread remain intact and functional
/// - **Timing Considerations**: There may be a brief scheduling delay before
///   the resumed thread actually begins executing
///
/// ## Scheduling Behavior
///
/// When a thread is resumed:
/// 1. The kernel marks the thread as `Runnable` in the scheduler
/// 2. The thread becomes eligible for CPU time based on its priority
/// 3. Execution resumes at the next scheduler quantum
/// 4. No thread-local state or memory mappings are reset
///
/// ## Use Cases
///
/// - **Cooperative Scheduling**: Implement custom thread scheduling algorithms
/// - **Resource Management**: Resume threads when resources become available
/// - **Load Balancing**: Distribute work by resuming paused worker threads  
/// - **Debugging**: Continue execution after inspection points
/// - **Power Management**: Resume threads after power-saving periods
///
/// ## Arguments
///
/// * `thread` - Reference to the Thread handle to resume
///
/// ## Returns
///
/// * `Ok(())` - Thread was successfully resumed or was already running
/// * `Err(ThreadResumeError)` - Thread resume failed (see error variants for details)
pub fn resume(thread: &Thread) -> Result<(), ThreadResumeError> {
    svc::resume(thread.handle()).map_err(Into::into)
}

/// Error type for [`resume`].
#[derive(Debug, thiserror::Error)]
pub enum ThreadResumeError {
    /// Supplied handle does not refer to a valid thread.
    ///
    /// This mirrors [`svc::ResumeThreadError::InvalidHandle`] and can occur if:
    /// - The thread handle is invalid or has been corrupted through memory issues
    /// - The target thread has exited and been cleaned up by the kernel
    /// - Kernel resource limits have been reached (rare but possible)
    /// - The handle refers to a non-thread kernel object (programming error)
    /// - The thread was terminated while paused, making the handle stale
    /// - Arc reference counting issues have corrupted the handle
    /// - System-wide thread limit has been exceeded
    #[error("Invalid thread handle")]
    InvalidHandle,

    /// Kernel returned an undocumented [`ResultCode`].
    ///
    /// The wrapped [`ResultCode`] is preserved so callers can inspect it
    /// using [`ResultCode::to_rc`] or display it for diagnostics. This
    /// indicates a kernel response that falls outside the documented
    /// error conditions, potentially signaling a kernel issue or rare
    /// edge case.
    ///
    /// This may indicate:
    /// - Internal kernel consistency errors
    /// - Race conditions in kernel thread management
    /// - Hardware-level scheduling issues
    /// - System update compatibility problems  
    /// - Resource exhaustion at the kernel scheduler level
    /// - Timing-dependent kernel bugs
    /// - Memory pressure affecting kernel operations
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
