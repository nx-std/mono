//! # Reentrant Mutex for Nintendo Switch
//!
//! This module provides a reentrant mutex, a synchronization primitive that can be used to
//! protect shared data from being simultaneously accessed by multiple threads. It is designed
//! specifically for the Nintendo Switch homebrew environment.
//!
//! ## The layout is not ours to choose
//!
//! A C caller allocates the lock and hands it to this implementation, so its shape is fixed by the
//! declaration the C side compiles against. That declaration is eight bytes, a lock word followed
//! by a recursion counter:
//!
//! ```c
//! struct __lock_t { _LOCK_T lock; uint32_t counter; };
//! ```
//!
//! It is what every `__syscall_lock_*_recursive` entry point is handed, and there is deliberately
//! no owner field in it. On Horizon a locked mutex word *is* the owner's thread tag, so who holds
//! the lock is already recorded in the first four bytes and a second copy would only be one more
//! thing to keep in step. The size assertions below pin this, because a third field here would put
//! the counter one word past the end of every lock the C library allocates.
//!
//! ## Behavior
//!
//! A reentrant mutex operates on a "per-thread" basis. A single thread can acquire a lock on the
//! mutex multiple times. The mutex will not be released until the same thread has called `unlock`
//! for every time it called `lock`. Other threads attempting to acquire the lock will block until
//! the owning thread has fully released it.
//!
//! ## Safety enhancements
//!
//! While keeping that layout and its semantics, this implementation refuses two things the C
//! behaviour permits silently:
//!
//! - **Unlock guard**: unlocking from a thread that does not hold the lock panics, rather than
//!   decrementing regardless and releasing the owner's mutex out from under it.
//! - **Counter underflow protection**: the recursion count saturates instead of wrapping, so a
//!   stray unlock cannot turn into a lock that can never be released.

use core::{
    cell::UnsafeCell,
    mem::{
        align_of,
        offset_of,
        size_of,
    },
};

use static_assertions::const_assert_eq;

use super::mutex::Mutex;

/// A reentrant mutual exclusion primitive useful for protecting shared data.
///
/// This is the Rust equivalent of the C standard library's `_LOCK_RECURSIVE_T`.
#[repr(C)]
pub struct ReentrantMutex {
    /// The lock itself, whose word names the owning thread while it is held.
    mutex: Mutex,
    /// How many times the owner has taken the lock without releasing it.
    counter: UnsafeCell<u32>,
}

// The C declaration this replaces is two 32-bit words. A mismatch here would not fail a build or a
// test: it would read one of the C caller's fields as another and write the rest past the end of
// the object, which shows up much later as memory that changed on its own.
const_assert_eq!(size_of::<ReentrantMutex>(), 2 * size_of::<u32>());
const_assert_eq!(align_of::<ReentrantMutex>(), align_of::<u32>());
const _: () = {
    assert!(offset_of!(ReentrantMutex, mutex) == 0);
    assert!(offset_of!(ReentrantMutex, counter) == size_of::<u32>());
};

impl Default for ReentrantMutex {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: the recursion count is only ever read or written by the thread holding the inner mutex,
// which is the one thing every path here establishes before touching it, so concurrent access is
// serialized by the lock itself. The mutex word is an atomic and is sound to share on its own.
unsafe impl Send for ReentrantMutex {}
// SAFETY: the recursion count is only ever read or written by the thread holding the inner mutex,
// so sharing a reference across threads cannot produce concurrent access to it. The mutex word is
// an atomic and is sound to share on its own.
unsafe impl Sync for ReentrantMutex {}

impl ReentrantMutex {
    /// Creates a new `ReentrantMutex`.
    pub const fn new() -> Self {
        Self {
            mutex: Mutex::new(),
            counter: UnsafeCell::new(0),
        }
    }

    /// Locks the reentrant mutex.
    ///
    /// If the mutex is already locked by the current thread, the lock count is incremented.
    /// If the mutex is locked by another thread, this function will block until the mutex is
    /// released.
    pub fn lock(&self) {
        if !self.mutex.is_locked_by_current_thread() {
            self.mutex.lock();
        }

        // SAFETY: the lock is held by this thread from here on, so nothing else reaches the count.
        let counter = unsafe { &mut *self.counter.get() };
        *counter += 1;
    }

    /// Attempts to lock the reentrant mutex.
    ///
    /// If the mutex is already locked by the current thread, the lock count is incremented and
    /// `true` is returned.
    /// If the mutex is locked by another thread, this function returns `false` immediately.
    /// If the mutex is unlocked, it becomes locked by the current thread, and `true` is returned.
    pub fn try_lock(&self) -> bool {
        if !self.mutex.is_locked_by_current_thread() && !self.mutex.try_lock() {
            return false;
        }

        // SAFETY: control only reaches here when this thread already held the lock or has just
        // taken it, so it is the only thread that can reach the count.
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
    /// Panics when called by a thread that does not hold the mutex. Releasing it anyway would hand
    /// the owner's lock to whoever asked next, which the C behaviour permits and this refuses to.
    pub fn unlock(&self) {
        if !self.mutex.is_locked_by_current_thread() {
            panic!("Thread attempted to unlock mutex it did not lock: MUTEX_UNLOCK_ERROR");
        }

        // SAFETY: the lock is held by this thread, so nothing else reaches the count.
        let counter = unsafe { &mut *self.counter.get() };
        *counter = counter.saturating_sub(1);
        if *counter == 0 {
            self.mutex.unlock();
        }
    }

    /// Waits on a condition variable while holding this reentrant mutex.
    ///
    /// Used by libsysbase's `__syscall_cond_wait_recursive`. The calling thread must hold the
    /// mutex exactly once (counter == 1) for this operation to succeed.
    ///
    /// # Errors
    ///
    /// Returns [`NotHeldOnceError`] if the calling thread does not hold the mutex exactly once.
    /// Waiting would otherwise release the inner mutex while outer acquisitions still expect to
    /// hold it.
    #[cfg(feature = "ffi")]
    pub(crate) fn cond_wait(
        &self,
        condvar: &super::Condvar,
        timeout: Option<core::time::Duration>,
    ) -> Result<nx_svc::result::ResultCode, NotHeldOnceError> {
        // The recursion count belongs to whoever holds the inner mutex. Read from a thread that
        // does not hold it, it races with the owner and reports the owner's nesting depth as if
        // it were the caller's, so ownership is what decides this first.
        if !self.mutex.is_locked_by_current_thread() {
            return Err(NotHeldOnceError { held: 0 });
        }

        // SAFETY: the inner mutex is held by this thread, so nothing else reaches the count.
        let counter = unsafe { *self.counter.get() };
        if counter != 1 {
            return Err(NotHeldOnceError { held: counter });
        }

        // The wait releases the inner mutex, so the count has to say the lock is free for as long
        // as it is. Restoring it afterwards is what makes the wait invisible to the caller.
        //
        // SAFETY: the count was observed to be one above, so this thread holds the lock and is the
        // only one that can reach the count until the wait releases it.
        unsafe { *self.counter.get() = 0 };

        // SAFETY: this thread was observed to hold `self.mutex` above and nothing since has
        // released it, which is the ownership the wait requires.
        let result = unsafe { condvar.wait_timeout(&self.mutex, timeout) };

        // SAFETY: the wait returns with the inner mutex reacquired by this thread, so the count is
        // once again reachable only from here.
        unsafe { *self.counter.get() = 1 };

        Ok(result)
    }
}

/// An error indicating the reentrant mutex was not held exactly once by the caller.
#[cfg(feature = "ffi")]
#[derive(Debug, thiserror::Error)]
#[error("The reentrant mutex is held {held} times, not once")]
pub(crate) struct NotHeldOnceError {
    /// The recursion count observed, which is zero if the caller holds no lock at all.
    held: u32,
}
