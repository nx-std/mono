//! # Reentrant Mutex for Nintendo Switch
//!
//! This module provides a reentrant mutex, a synchronization primitive that can be used to
//! protect shared data from being simultaneously accessed by multiple threads. It is designed
//! specifically for the Nintendo Switch homebrew environment.
//!
//! ## FFI Compatibility with libnx
//!
//! This implementation is FFI-compatible with `libnx`'s `RMutex` type. This allows for seamless
//! interoperability between Rust code and existing C/C++ code that uses `libnx` for synchronization.
//! The memory layout and core locking/unlocking logic are identical to ensure this compatibility.
//!
//! ## Behavior
//!
//! A reentrant mutex operates on a "per-thread" basis. A single thread can acquire a lock on the
//! mutex multiple times. The mutex will not be released until the same thread has called `unlock`
//! for every time it called `lock`. Other threads attempting to acquire the lock will block until
//! the owning thread has fully released it.
//!
//! ## Safety Enhancements
//!
//! While maintaining compatibility with `libnx`, this Rust implementation introduces several
//! key safety improvements:
//!
//! - **Unlock Guard**: This implementation will trigger a panic if a thread attempts to
//!   unlock a mutex it does not own. In `libnx`, this is undefined behavior that can lead to
//!   crashes or data corruption.
//! - **Counter Underflow Protection**: The internal lock counter is protected from underflowing
//!   using saturating subtraction, preventing another potential class of bugs.

use core::cell::UnsafeCell;

use nx_svc::raw::{Handle, INVALID_HANDLE};

use super::mutex::Mutex;

/// A reentrant mutual exclusion primitive useful for protecting shared data.
///
/// This is the Rust equivalent of `RMutex` from `libnx`.
#[repr(C)]
pub struct ReentrantMutex {
    mutex: Mutex,
    thread_tag: UnsafeCell<Handle>,
    counter: UnsafeCell<u32>,
}

impl Default for ReentrantMutex {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for ReentrantMutex {}
unsafe impl Sync for ReentrantMutex {}

impl ReentrantMutex {
    /// Creates a new `ReentrantMutex`.
    pub const fn new() -> Self {
        Self {
            mutex: Mutex::new(),
            thread_tag: UnsafeCell::new(INVALID_HANDLE),
            counter: UnsafeCell::new(0),
        }
    }

    /// Locks the reentrant mutex.
    ///
    /// If the mutex is already locked by the current thread, the lock count is incremented.
    /// If the mutex is locked by another thread, this function will block until the mutex is released.
    pub fn lock(&self) {
        let current_thread_handle = get_curr_thread_handle();
        let thread_tag = unsafe { *self.thread_tag.get() };

        if thread_tag != current_thread_handle {
            self.mutex.lock();
            unsafe {
                *self.thread_tag.get() = current_thread_handle;
            }
        }
        let counter = unsafe { &mut *self.counter.get() };
        *counter += 1;
    }

    /// Attempts to lock the reentrant mutex.
    ///
    /// If the mutex is already locked by the current thread, the lock count is incremented and `true` is returned.
    /// If the mutex is locked by another thread, this function returns `false` immediately.
    /// If the mutex is unlocked, it becomes locked by the current thread, and `true` is returned.
    pub fn try_lock(&self) -> bool {
        let current_thread_handle = get_curr_thread_handle();
        let thread_tag = unsafe { *self.thread_tag.get() };

        if thread_tag != current_thread_handle {
            if !self.mutex.try_lock() {
                return false;
            }
            unsafe {
                *self.thread_tag.get() = current_thread_handle;
            }
        }
        let counter = unsafe { &mut *self.counter.get() };
        *counter += 1;
        true
    }

    /// Unlocks the reentrant mutex.
    ///
    /// The mutex is only released when the lock count reaches zero.
    ///
    /// # Panics
    ///
    /// This function will panic if it is called by a thread that has not locked the mutex.
    pub fn unlock(&self) {
        let current_thread_handle = get_curr_thread_handle();
        let thread_tag = unsafe { *self.thread_tag.get() };

        if thread_tag != current_thread_handle {
            // Reentrant mutexes are not allowed to be unlocked by a thread that did not lock them.
            // This can lead to premature unlocking of the mutex, which can lead to undefined behavior.
            // This is undefined behavior in libnx, but we can catch it.
            panic!("Thread attempted to unlock mutex it did not lock: MUTEX_UNLOCK_ERROR");
        }

        let counter = unsafe { &mut *self.counter.get() };
        *counter = counter.saturating_sub(1);
        if *counter == 0 {
            unsafe {
                *self.thread_tag.get() = 0;
            }
            self.mutex.unlock();
        }
    }
}

/// Get the current thread's kernel handle.
#[inline(always)]
fn get_curr_thread_handle() -> Handle {
    nx_sys_thread_tls::get_current_thread_handle().to_raw()
}
