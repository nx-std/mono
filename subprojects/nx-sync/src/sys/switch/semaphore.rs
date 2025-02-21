//! # Semaphore
//!
//! A semaphore is a synchronization primitive that maintains a count of available resources.
//! It allows threads to wait for and release resources in a thread-safe manner. The semaphore's
//! internal counter represents the number of available resources.

use core::cell::UnsafeCell;

use static_assertions::const_assert_eq;

use super::{Condvar, Mutex};

/// A counting semaphore synchronization primitive.
///
/// The semaphore maintains an internal counter of available resources. Threads can
/// wait for resources (decrementing the counter) or signal when they're done
/// (incrementing the counter).
#[repr(C)]
pub struct Semaphore {
    /// Condition variable for thread synchronization
    condvar: Condvar,
    /// Mutex for protecting the internal counter
    mutex: Mutex,
    /// Number of available resources
    count: UnsafeCell<u64>,
}

// Ensure that the Semaphore object has a 16 bytes size, and is properly aligned
const_assert_eq!(size_of::<Semaphore>(), 16);
const_assert_eq!(align_of::<Semaphore>(), align_of::<u64>());

impl Semaphore {
    /// Creates a new Semaphore with the specified initial count.
    ///
    /// # Arguments
    /// * `count` - Initial value for the semaphore's counter, typically representing
    ///   the number of available resources. It must be >= 1.
    pub const fn new(count: u64) -> Self {
        Self {
            condvar: Condvar::new(),
            mutex: Mutex::new(),
            count: UnsafeCell::new(count),
        }
    }

    /// Signals the semaphore, incrementing its counter and potentially waking a waiting thread.
    pub fn signal(&self) {
        self.mutex.lock();

        // Increment the count and wake one waiting thread
        let count = unsafe { &mut *self.count.get() };
        *count = count.checked_add(1).expect("semaphore count overflow");
        self.condvar.wake_one();

        self.mutex.unlock();
    }

    /// Waits for the semaphore, decrementing its counter when a resource becomes available.
    ///
    /// This call will block if no resources are currently available.
    pub fn wait(&self) {
        self.mutex.lock();

        // If count is 0, wait until signaled
        let count = unsafe { &mut *self.count.get() };
        #[allow(clippy::while_immutable_condition)]
        while *count == 0 {
            self.condvar.wait(&self.mutex);
        }
        *count = count.checked_sub(1).expect("semaphore count underflow");

        self.mutex.unlock();
    }

    /// Attempts to wait for the semaphore without blocking.
    ///
    /// Returns `true` if a resource was acquired, `false` if no resources were available.
    ///
    /// # Safety
    /// This function is safe to call with an immutable reference because the internal
    /// synchronization is handled by the Mutex in __nx_sync_semaphore_try_wait.
    pub fn try_wait(&self) -> bool {
        self.mutex.lock();

        // Check and immediately return result
        let count = unsafe { &mut *self.count.get() };
        let result = if *count > 0 {
            *count = count.checked_sub(1).expect("semaphore count underflow");
            true // Successfully decremented
        } else {
            false // No resources available
        };

        self.mutex.unlock();
        result
    }
}
