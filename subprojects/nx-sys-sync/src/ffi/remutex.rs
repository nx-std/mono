//! FFI bindings for the `nx-sys-sync` crate - Reentrant Mutex

use crate::remutex::ReentrantMutex;

/// Initializes the reentrant mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Writes to the memory pointed to by `mutex`
/// - Requires that `mutex` is valid and properly aligned
/// - Requires that `mutex` points to memory that can be safely written to
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_remutex_init(mutex: *mut ReentrantMutex) {
    unsafe { mutex.write(ReentrantMutex::new()) }
}

/// Locks the reentrant mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid ReentrantMutex instance
/// - Requires that `mutex` is properly aligned
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_remutex_lock(mutex: *mut ReentrantMutex) {
    unsafe { &*mutex }.lock()
}

/// Attempts to lock the reentrant mutex without waiting.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid ReentrantMutex instance
/// - Requires that `mutex` is properly aligned
///
/// # Returns
///
/// Returns `true` if the mutex was successfully locked, `false` if it was already locked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_remutex_try_lock(mutex: *mut ReentrantMutex) -> bool {
    unsafe { &*mutex }.try_lock()
}

/// Unlocks the reentrant mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid ReentrantMutex instance
/// - Requires that `mutex` is properly aligned
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_remutex_unlock(mutex: *mut ReentrantMutex) {
    unsafe { &*mutex }.unlock()
}
