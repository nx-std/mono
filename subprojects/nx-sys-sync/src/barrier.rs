//! # Barrier
//!
//! A synchronization primitive that enables multiple threads to synchronize at a specific point.
//! The barrier ensures that no thread can proceed past the barrier point until all participating
//! threads have reached it.
//!
//! This implementation is built on top of mutex and condition variables to provide thread
//! synchronization capabilities. It maintains an internal counter that tracks the number of
//! threads that have reached the barrier.

use core::{cell::UnsafeCell, num::NonZeroU32};

use static_assertions::const_assert_eq;

use super::{Condvar, Mutex};
use crate::wait::WakeCount;

/// Barrier structure
///
/// A synchronization primitive used to ensure that a group of threads all reach a particular
/// point of execution before any of them proceed.
///
/// The barrier is created with a specified number of threads that must call `wait()` before
/// any of them are allowed to proceed. When a thread calls `wait()`, it blocks until all
/// other threads have also called `wait()`. Once the last thread calls `wait()`, all threads
/// are unblocked and can continue execution.
pub struct Barrier {
    /// Number of threads to reach the barrier
    count: UnsafeCell<u64>,
    /// Number of threads to wait on
    total: u64,
    /// Mutex for synchronization
    mutex: Mutex,
    /// Condition variable for thread waiting
    condvar: Condvar,
}

// Ensure that the Barrier has a 24 bytes size, and is properly aligned
const_assert_eq!(size_of::<Barrier>(), 24);
const_assert_eq!(align_of::<Barrier>(), align_of::<u64>());

// SAFETY: `Barrier`'s `count` cell is mutated only while its internal `Mutex`
// is held, so concurrent access from multiple threads is serialized. Sharing
// the barrier across threads is therefore sound — as it must be for a
// synchronization primitive.
unsafe impl Send for Barrier {}
unsafe impl Sync for Barrier {}

impl Barrier {
    /// Initializes a barrier and the number of threads to wait on.
    ///
    /// # Arguments
    /// * `thread_count` - The number of threads that must call `wait()` before any can proceed.
    ///   Must be greater than 0.
    ///
    /// # Panics
    ///
    /// Will not panic as long as `thread_count` is greater than 0.
    pub fn new(thread_count: u64) -> Self {
        Barrier {
            count: UnsafeCell::new(0),
            total: thread_count - 1,
            mutex: Mutex::new(),
            condvar: Condvar::new(),
        }
    }

    /// Blocks the current thread until all threads have reached this point.
    ///
    /// When the specified number of threads have called this function, all threads will be
    /// unblocked and the barrier will be reset, ready for reuse.
    ///
    /// # Panics
    ///
    /// Panics if the internal counter would overflow, which is extremely unlikely in
    /// practice as it would require more threads than the maximum value of u64.
    pub fn wait(&self) {
        self.mutex.lock();

        let count = unsafe { &mut *self.count.get() };
        if *count == self.total {
            *count = 0;
            // A barrier's participant count always fits a `u32`. A count of zero reaches
            // the SVC as "wake all", which is what the previous `wake(0)` did and is
            // harmless: a barrier with no other participants has no waiters to release.
            let waiters = u32::try_from(self.total).unwrap_or(u32::MAX);
            self.condvar.wake(match NonZeroU32::new(waiters) {
                Some(count) => WakeCount::AtMost(count),
                None => WakeCount::All,
            });
        } else {
            *count = count.checked_add(1).expect("Barrier count overflow");
            self.condvar.wait(&self.mutex);
        }

        self.mutex.unlock();
    }
}
