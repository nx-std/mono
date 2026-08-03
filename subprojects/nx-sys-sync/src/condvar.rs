//! Condition Variable
//!
//! A condition variable is a synchronization primitive that enables threads to wait
//! until a particular condition occurs. Condition variables are used in conjunction
//! with mutexes to handle situations where a thread needs to wait for some condition
//! that depends on other threads.

use core::{num::NonZeroU32, sync::atomic::AtomicU32};

use nx_svc::{
    error::ToResultCode,
    result::ResultCode,
    sync::{WaitProcessWideKeyError, signal_process_wide_key, wait_process_wide_key_atomic},
};
use static_assertions::const_assert_eq;

use super::Mutex;
use crate::{
    tag::ThreadTag,
    wait::{Timeout, WakeCount},
};

/// A condition variable primitive for thread synchronization.
///
/// Condition variables are used in conjunction with mutexes to allow threads to wait
/// until a particular condition occurs. This is a low-level implementation that
/// directly interfaces with the Nintendo Switch's synchronization primitives.
// NOTE: The in-memory representation must be u32 for FFI compatibility with libnx's `CondVar`.
#[repr(C)]
pub struct Condvar(AtomicU32);

// Ensure the in-memory size of the Condvar is the same as u32
const_assert_eq!(size_of::<Condvar>(), size_of::<u32>());

impl Condvar {
    /// Creates a new condition variable initialized to 0.
    pub const fn new() -> Self {
        Condvar(AtomicU32::new(0))
    }

    /// Returns a raw pointer to the underlying integer.
    ///
    /// The kernel writes through this pointer while the caller holds only a `&Condvar`, which is
    /// why the word is an `AtomicU32`: taking a `*mut` from a plain `&u32` and letting the kernel
    /// store through it would be a mutation through a shared reference.
    ///
    /// # Safety
    ///
    /// This function is intended for FFI purposes and should be used with care.
    /// The caller must ensure that:
    /// - The pointer is not used after the condition variable is dropped
    /// - The pointer is only used with Nintendo Switch kernel synchronization primitives
    /// - The pointer is properly aligned and valid for the lifetime of the condition variable
    pub fn as_ptr(&self) -> *mut u32 {
        self.0.as_ptr()
    }

    /// Waits on the condition variable until notified or a timeout occurs.
    ///
    /// This function atomically releases the mutex and suspends the current thread until either:
    /// - Another thread calls `wake()`, `wake_one()` or `wake_all()`
    /// - The specified timeout duration elapses
    ///
    /// When the function returns, the mutex is guaranteed to be re-acquired.
    ///
    /// Returns `0` on a successful wait and wake, or the result code of the wait otherwise; a
    /// timeout is reported as an error rather than as a distinct success.
    pub fn wait_timeout(&self, mutex: &Mutex, timeout: Timeout) -> ResultCode {
        let curr_thread_tag = ThreadTag::current();

        let result = unsafe {
            wait_process_wide_key_atomic(
                self.as_ptr(),
                mutex.as_ptr(),
                curr_thread_tag.to_raw(),
                timeout.to_raw(),
            )
        };

        // Handle the timeout case specially since we need to re-acquire the mutex
        if let Err(WaitProcessWideKeyError::TimedOut) = result {
            mutex.lock();
        }

        // Map result to return codes
        result.map_or_else(ToResultCode::to_rc, |_| 0)
    }

    /// Waits on the condition variable indefinitely until notified.
    ///
    /// This function atomically releases the mutex and suspends the current thread until
    /// another thread calls `wake()`, `wake_one()` or `wake_all()`. When the function
    /// returns, the mutex is guaranteed to be re-acquired.
    ///
    /// Returns `0` on a successful wait and wake, or the result code of the wait otherwise.
    #[inline]
    pub fn wait(&self, mutex: &Mutex) -> ResultCode {
        self.wait_timeout(mutex, Timeout::Infinite)
    }

    /// Wakes threads waiting on the condition variable.
    pub fn wake(&self, count: WakeCount) {
        unsafe { signal_process_wide_key(self.as_ptr(), count.to_raw()) };
    }

    /// Wakes up a single thread waiting on the condition variable.
    ///
    /// If multiple threads are waiting, the highest priority thread will be woken.
    #[inline]
    pub fn wake_one(&self) {
        self.wake(WakeCount::AtMost(NonZeroU32::MIN));
    }

    /// Wakes up all threads waiting on the condition variable.
    #[inline]
    pub fn wake_all(&self) {
        self.wake(WakeCount::All);
    }
}

impl Default for Condvar {
    fn default() -> Self {
        Self::new()
    }
}
