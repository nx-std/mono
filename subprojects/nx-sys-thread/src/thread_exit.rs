//! Thread exit implementation
//!
//! This module provides thread termination functionality for threads created
//! using the Arc+Pin pattern.

use nx_svc::thread as svc;

use crate::thread_handle::Thread;

/// Exits the current thread
///
/// This function performs proper cleanup and termination of a thread created
/// using the Arc+Pin pattern. It is the equivalent of the original libnx
/// thread exit functionality, adapted for the new thread handle design.
///
/// ## Cleanup Operations
///
/// The function performs the following cleanup operations in sequence:
///
/// 1. **TLS Destructors** - Runs destructors for any thread-local storage
///    that has been allocated for this thread. This ensures proper cleanup
///    of thread-specific resources and prevents memory leaks.
///
/// 2. **Registry Removal** - Removes the thread from the global thread
///    registry, ensuring the registry remains consistent and preventing
///    dangling references to the terminated thread.
///
/// 3. **Thread Termination** - Calls `svcExitThread` to actually terminate
///    the thread's execution at the kernel level.
///
/// ## Safety Requirements
///
/// This function is `unsafe` because it performs low-level thread termination
/// and the caller must guarantee:
///
/// * **Current Thread Only** - Must be called from the thread that is being
///   exited. Calling this function from a different thread results in
///   undefined behavior and may corrupt the process state.
///
/// * **Valid Thread Handle** - The thread handle must be valid and represent
///   the currently executing thread. Using an invalid or mismatched handle
///   can lead to registry corruption or kernel errors.
///
/// * **No Outstanding References** - The caller must ensure that no other
///   parts of the program are holding active references to thread-local
///   resources that would be invalidated by the exit process.
///
/// ## Arguments
///
/// * `thread` - The thread handle for the current thread. This handle is used
///   to identify which thread resources need to be cleaned up and removed
///   from the registry.
///
/// ## Behavior
///
/// This function **never returns** as it terminates the calling thread. Any
/// code placed after a call to this function will never be executed.
///
/// ## Implementation Notes
///
/// Currently, some cleanup operations are disabled as TODOs while the
/// system is being migrated to the Arc+Pin pattern:
///
/// - TLS destructors are disabled until dynamic TLS slots are re-enabled
/// - Registry removal is disabled until the registry is updated for Arc+Pin
///
/// These will be re-enabled as the migration progresses.
pub unsafe fn exit(_thread: Thread) -> ! {
    // SAFETY: Called on the current thread.
    // TODO: Run TLS destructors when dynamic TLS is re-enabled
    // This matches the original implementation:
    // unsafe { slots::run_destructors() };

    // Remove thread from the global registry
    // SAFETY: `thread` was previously inserted during creation; removing it
    // now is valid and ensures the registry is kept consistent.
    // TODO: Re-enable when registry is updated for Arc+Pin pattern
    // unsafe { registry::remove(&thread) };

    // TODO: Clear pointer fields to catch use-after-free bugs in debug builds
    // This would be done if we had mutable access to thread fields:
    // thread.tls_slots = None;

    // Terminate the thread via svcExitThread (never returns)
    svc::exit();
}
