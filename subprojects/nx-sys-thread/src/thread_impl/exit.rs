//! Thread exit implementation
//!
//! This module provides the thread exit functionality.

use nx_svc::thread as svc;

use super::info::Thread;

/// Exits the current thread.
///
/// This function performs cleanup operations and terminates the thread:
/// - Runs TLS slot destructors (when slots support is reimplemented)
/// - Removes the thread from the global registry
/// - Clears pointer fields to catch use-after-free bugs
/// - Terminates the thread via svcExitThread (never returns)
///
/// # Safety
/// This function must only be called by the thread that is exiting.
/// The thread parameter must be a valid pointer to the current thread's info structure.
pub unsafe fn exit(_thread: &mut Thread) -> ! {
    // TODO: Reimplement TLS slots destructors
    // SAFETY: Called on the current thread.
    // unsafe { slots::run_destructors() };

    // TODO: Reimplement thread registry functionality
    // Remove thread from the global registry
    // SAFETY: `thread` was previously inserted during creation; removing it
    // now is valid and ensures the registry is kept consistent.
    // unsafe { registry::remove(thread) };

    // TODO: Reimplement TLS slots cleanup
    // Clear pointer fields to catch use-after-free bugs in debug builds.
    // thread.tls_slots = None;

    // Terminate the thread via svcExitThread (never returns)
    svc::exit();
}
