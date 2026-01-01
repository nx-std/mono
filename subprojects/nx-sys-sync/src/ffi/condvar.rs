//! FFI bindings for the `nx-sys-sync` crate - Condvar
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/condvar.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/condvar.h)

use nx_svc::result::ResultCode;

use crate::{condvar::Condvar, mutex::Mutex};

/// Initializes a condition variable.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to valid memory that can hold a `Condvar`
/// * The memory pointed to by `condvar` remains valid for the entire lifetime of the condition variable
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__condvar_init(condvar: *mut Condvar) {
    unsafe { condvar.write(Condvar::new()) };
}

/// Waits on a condition variable with a timeout
///
/// This function atomically releases the mutex and waits on the condition variable.
/// When the function returns, regardless of the reason, the mutex is guaranteed to be
/// re-acquired.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
/// * `mutex` points to a valid initialized mutex
/// * The current thread owns the mutex
///
/// # Parameters
///
/// * `condvar`: Pointer to the condition variable to wait on
/// * `mutex`: Pointer to the mutex protecting the condition
/// * `timeout`: Maximum time to wait in nanoseconds
///
/// # Returns
///
/// * `0` on successful wait and wake
/// * `0xEA01` if the wait timed out
/// * Other values indicate an error
///
/// # Notes
///
/// On function return, the underlying mutex is guaranteed to be acquired, even in case
/// of timeout or error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__condvar_wait_timeout(
    condvar: *mut Condvar,
    mutex: *mut Mutex,
    timeout: u64,
) -> ResultCode {
    let mutex = unsafe { &*mutex };
    unsafe { &*condvar }.wait_timeout(mutex, timeout)
}

/// Waits on a condition variable indefinitely
///
/// This function atomically releases the mutex and waits on the condition variable
/// with no timeout. When the function returns, the mutex is guaranteed to be re-acquired.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
/// * `mutex` points to a valid initialized mutex
/// * The current thread owns the mutex
///
/// # Parameters
///
/// * `condvar`: Pointer to the condition variable to wait on
/// * `mutex`: Pointer to the mutex protecting the condition
///
/// # Returns
///
/// * `0` on successful wait and wake
/// * Non-zero value indicates an error
///
/// # Notes
///
/// On function return, the underlying mutex is guaranteed to be acquired.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__condvar_wait(
    condvar: *mut Condvar,
    mutex: *mut Mutex,
) -> ResultCode {
    let mutex = unsafe { &*mutex };
    unsafe { &*condvar }.wait_timeout(mutex, u64::MAX)
}

/// Wakes up a specified number of threads waiting on a condition variable.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
///
/// # Parameters
///
/// * `condvar`: Pointer to the condition variable
/// * `num`: Maximum number of threads to wake up
///   * If positive, wake up to that many threads
///   * If <= 0, e.g., -1, wake up all waiting threads
///
/// # Returns
///
/// * `0` on success
/// * Non-zero value indicates an error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__condvar_wake(
    condvar: *mut Condvar,
    num: i32,
) -> ResultCode {
    unsafe { &*condvar }.wake(num);
    0
}

/// Wakes up a single thread waiting on a condition variable
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
///
/// # Returns
///
/// * `0` on success
/// * Non-zero value indicates an error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__condvar_wake_one(condvar: *mut Condvar) -> ResultCode {
    unsafe { &*condvar }.wake_one();
    0
}

/// Wakes up all threads waiting on a condition variable.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
///
/// # Returns
///
/// * `0` on success
/// * Non-zero value indicates an error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__condvar_wake_all(condvar: *mut Condvar) -> ResultCode {
    unsafe { &*condvar }.wake_all();
    0
}
