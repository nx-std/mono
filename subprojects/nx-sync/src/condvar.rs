//! Condition Variable
//!
//! A condition variable is a synchronization primitive that enables threads to wait
//! until a particular condition occurs. Condition variables are used in conjunction
//! with mutexes to handle situations where a thread needs to wait for some condition
//! that depends on other threads.

use nx_svc::{
    error::ToRawResultCode,
    sync::{WaitProcessWideKeyError, signal_process_wide_key, wait_process_wide_key_atomic},
};
use nx_thread::raw::Handle;

use crate::mutex::Mutex;

/// Result type
type Result = u32;

/// A condition variable primitive for thread synchronization.
///
/// Condition variables are used in conjunction with mutexes to allow threads to wait
/// until a particular condition occurs. This is a low-level implementation that
/// directly interfaces with the Nintendo Switch's synchronization primitives.
#[repr(C)]
pub struct Condvar(u32);

impl Condvar {
    /// Creates a new condition variable initialized to 0.
    pub const fn new() -> Self {
        Condvar(0)
    }
    /// Returns a raw pointer to the underlying integer.
    ///
    /// # Safety
    ///
    /// This function is intended for FFI purposes and should be used with care.
    /// The caller must ensure that:
    /// - The pointer is not used after the condition variable is dropped
    /// - The pointer is only used with Nintendo Switch kernel synchronization primitives
    /// - The pointer is properly aligned and valid for the lifetime of the condition variable
    pub fn as_ptr(&self) -> *mut u32 {
        &self.0 as *const _ as *mut u32
    }

    /// Waits on the condition variable until notified or a timeout occurs.
    ///
    /// This function atomically releases the mutex and suspends the current thread until either:
    /// - Another thread calls `wake()`, `wake_one()` or `wake_all()`
    /// - The specified timeout duration elapses
    ///
    /// When the function returns, the mutex is guaranteed to be re-acquired.
    ///
    /// # Arguments
    /// * `mutex` - The mutex protecting the condition
    /// * `timeout` - Maximum time to wait in nanoseconds
    ///
    /// # Returns
    /// * `0` on successful wait and wake
    /// * Error code if the wait timed out or another error occurred
    pub fn wait_timeout(&self, mutex: &Mutex, timeout: u64) -> Result {
        let curr_thread_handle = get_curr_thread_handle();

        let result = unsafe {
            wait_process_wide_key_atomic(self.as_ptr(), mutex.as_ptr(), curr_thread_handle, timeout)
        };

        // Handle the timeout case specially since we need to re-acquire the mutex
        if let Err(WaitProcessWideKeyError::TimedOut) = result {
            mutex.lock();
        }

        // Map result to return codes
        result.map_or_else(ToRawResultCode::to_rc, |_| 0)
    }

    /// Waits on the condition variable indefinitely until notified.
    ///
    /// This function atomically releases the mutex and suspends the current thread until
    /// another thread calls `wake()`, `wake_one()` or `wake_all()`. When the function
    /// returns, the mutex is guaranteed to be re-acquired.
    ///
    /// # Arguments
    /// * `mutex` - The mutex protecting the condition
    ///
    /// # Returns
    /// * `0` on successful wait and wake
    /// * Error code if an error occurred
    #[inline]
    pub fn wait(&self, mutex: &Mutex) -> Result {
        self.wait_timeout(mutex, u64::MAX)
    }

    /// Wakes up a specified number of threads waiting on the condition variable.
    ///
    /// # Arguments
    /// * `num` - Number of threads to wake:
    ///   - If positive, wakes up to that many threads
    ///   - If zero or negative, wakes all waiting threads
    pub fn wake(&self, num: i32) {
        unsafe { signal_process_wide_key(self.as_ptr(), num) };
    }

    /// Wakes up a single thread waiting on the condition variable.
    ///
    /// If multiple threads are waiting, the highest priority thread will be woken.
    #[inline]
    pub fn wake_one(&self) {
        self.wake(1);
    }

    /// Wakes up all threads waiting on the condition variable.
    #[inline]
    pub fn wake_all(&self) {
        self.wake(-1);
    }
}

impl Default for Condvar {
    fn default() -> Self {
        Self::new()
    }
}

/// Initializes a condition variable.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to valid memory that can hold a `Condvar`
/// * The memory pointed to by `condvar` remains valid for the entire lifetime of the condition variable
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_init(condvar: *mut Condvar) {
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
pub unsafe extern "C" fn __nx_sync_condvar_wait_timeout(
    condvar: *mut Condvar,
    mutex: *mut Mutex,
    timeout: u64,
) -> Result {
    let condvar = unsafe { &*condvar };
    let mutex = unsafe { &*mutex };
    condvar.wait_timeout(mutex, timeout)
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
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wait(
    condvar: *mut Condvar,
    mutex: *mut Mutex,
) -> Result {
    let condvar = unsafe { &*condvar };
    let mutex = unsafe { &*mutex };
    condvar.wait_timeout(mutex, u64::MAX)
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
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wake(condvar: *mut Condvar, num: i32) -> Result {
    let condvar = unsafe { &*condvar };
    condvar.wake(num);
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
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wake_one(condvar: *mut Condvar) -> Result {
    let condvar = unsafe { &*condvar };
    condvar.wake_one();
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
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wake_all(condvar: *mut Condvar) -> Result {
    let condvar = unsafe { &*condvar };
    condvar.wake_all();
    0
}

/// Get the current thread's kernel handle
#[inline(always)]
fn get_curr_thread_handle() -> Handle {
    unsafe { nx_thread::raw::__nx_thread_get_current_thread_handle() }
}
