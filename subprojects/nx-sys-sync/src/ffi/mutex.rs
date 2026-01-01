//! FFI bindings for the `nx-sys-sync` crate - Mutex
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/mutex.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/mutex.h)

use crate::mutex::Mutex;

/// Initializes the mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Writes to the memory pointed to by `mutex`
/// - Requires that `mutex` is valid and properly aligned
/// - Requires that `mutex` points to memory that can be safely written to
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__mutex_init(mutex: *mut Mutex) {
    unsafe { mutex.write(Mutex::new()) }
}

/// Locks the mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__mutex_lock(mutex: *mut Mutex) {
    unsafe { &*mutex }.lock()
}

/// Attempts to lock the mutex without waiting.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
///
/// # Returns
///
/// Returns `true` if the mutex was successfully locked, `false` if it was already locked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__mutex_try_lock(mutex: *mut Mutex) -> bool {
    unsafe { &*mutex }.try_lock()
}

/// Unlocks the mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__mutex_unlock(mutex: *mut Mutex) {
    unsafe { &*mutex }.unlock()
}

/// Checks if the mutex is locked by the current thread.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
///
/// # Returns
///
/// Returns `true` if the mutex is currently locked by the calling thread,
/// `false` otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__mutex_is_locked_by_current_thread(
    mutex: *mut Mutex,
) -> bool {
    unsafe { &*mutex }.is_locked_by_current_thread()
}
