//! FFI bindings for the `nx-sys-sync` crate - Semaphore
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/semaphore.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/semaphore.h)

use crate::semaphore::Semaphore;

/// Initializes a semaphore with an initial counter value.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object to initialize
/// * `count` - Initial value for the semaphore's counter. It must be >= 1.
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to valid memory that is properly aligned for a Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__semaphore_init(sem: *mut Semaphore, count: u64) {
    unsafe { sem.write(Semaphore::new(count)) };
}

/// Increments the semaphore's counter and wakes one waiting thread.
///
/// This function is used when a thread is done with a resource, making it
/// available for other threads.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to a valid, initialized Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__semaphore_signal(sem: *mut Semaphore) {
    unsafe { &*sem }.signal()
}

/// Decrements the semaphore's counter, blocking if no resources are available.
///
/// If the counter is 0, the calling thread will block until another thread
/// signals the semaphore.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to a valid, initialized Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__semaphore_wait(sem: *mut Semaphore) {
    unsafe { &*sem }.wait()
}

/// Attempts to decrement the semaphore's counter without blocking.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object
///
/// # Returns
/// * `true` if the counter was successfully decremented
/// * `false` if the counter was 0 (no resources available)
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to a valid, initialized Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__semaphore_try_wait(sem: *mut Semaphore) -> bool {
    unsafe { &*sem }.try_wait()
}
