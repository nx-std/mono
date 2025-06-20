//! # Semaphore
//!
//! A semaphore is a synchronization primitive that maintains a count of available resources.
//! It allows threads to wait for and release resources in a thread-safe manner. The semaphore's
//! internal counter represents the number of available resources.
//!
//! Semaphores are useful in scenarios where you need to:
//! - Control access to a pool of resources
//! - Implement producer-consumer patterns
//! - Coordinate multiple threads accessing shared resources
//! - Limit concurrent access to certain operations
//!
//! When a permit is acquired, it is represented by a [`SemaphorePermit`] that automatically
//! releases the permit back to the semaphore when dropped.

use core::fmt;

use nx_sys_sync::sys::switch as sys;

/// A counting semaphore synchronization primitive.
///
/// The semaphore maintains an internal counter of available resources. Threads can
/// wait for resources (decrementing the counter) or signal when they're done
/// (incrementing the counter).
///
/// # Resource Management
/// - Each permit acquisition decrements the counter
/// - Each permit release increments the counter
/// - When no permits are available, threads will block until a permit is released
pub struct Semaphore {
    /// The low-level semaphore
    inner: sys::Semaphore,
}

impl Semaphore {
    /// Creates a new semaphore with the initial number of permits.
    ///
    /// The `permits` parameter sets the initial number of available resources that
    /// can be acquired concurrently.
    pub const fn new(permits: usize) -> Self {
        Self {
            inner: sys::Semaphore::new(permits as u64),
        }
    }

    /// Acquires a permit from the semaphore.
    ///
    /// This returns a [`SemaphorePermit`] representing the acquired permit.
    ///
    /// # Blocking Behavior
    /// This method will block the current thread until a permit becomes available.
    /// The permit is automatically released when dropped.
    pub fn acquire(&self) -> SemaphorePermit<'_> {
        self.inner.wait();
        SemaphorePermit {
            sem: self,
            permits: 1,
        }
    }

    /// Tries to acquire a permit from the semaphore without blocking.
    ///
    /// # Returns
    /// - `Ok(SemaphorePermit)` if a permit was successfully acquired
    /// - `Err(TryAcquireError::NoPermits)` if no permits are currently available
    ///
    /// Unlike [`acquire`], this method will not block if no permits are available.
    pub fn try_acquire(&self) -> Result<SemaphorePermit<'_>, TryAcquireError> {
        if self.inner.try_wait() {
            Ok(SemaphorePermit {
                sem: self,
                permits: 1,
            })
        } else {
            Err(TryAcquireError::NoPermits)
        }
    }
}

/// A permit from the semaphore that borrows the semaphore reference.
///
/// This type is created by the [`acquire`] and [`try_acquire`] methods.
/// When dropped, the permit is automatically released back to the semaphore.
///
/// [`acquire`]: Semaphore::acquire
/// [`try_acquire`]: Semaphore::try_acquire
#[must_use]
#[clippy::has_significant_drop]
pub struct SemaphorePermit<'a> {
    sem: &'a Semaphore,
    permits: u32,
}
impl<'a> SemaphorePermit<'a> {
    /// Returns the number of permits held by `self`.
    pub fn num_permits(&self) -> usize {
        self.permits as usize
    }
}

impl Drop for SemaphorePermit<'_> {
    fn drop(&mut self) {
        // Release the permits back to the semaphore, and wake up any waiting threads
        self.sem.inner.signal();
    }
}

/// Error returned when trying to acquire a permit fails.
///
/// This error is returned by [`try_acquire`] when there are no permits available.
///
/// [`try_acquire`]: Semaphore::try_acquire
#[derive(Debug, PartialEq, Eq)]
pub enum TryAcquireError {
    /// The semaphore has no available permits at this time.
    NoPermits,
}

impl TryAcquireError {
    /// Returns `true` if the error was caused by calling `try_acquire` on a
    /// semaphore with no available permits.
    #[allow(dead_code)] // may be used later!
    pub(crate) fn is_no_permits(&self) -> bool {
        matches!(self, TryAcquireError::NoPermits)
    }
}

impl fmt::Display for TryAcquireError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryAcquireError::NoPermits => write!(fmt, "no permits available"),
        }
    }
}

impl core::error::Error for TryAcquireError {}
