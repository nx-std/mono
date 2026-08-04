//! Data-protecting mutex: the allocation-free [`crate::mutex::Mutex`] bundled
//! with the data it guards, handing out scoped access through an RAII
//! [`MutexGuard`].
//!
//! See the [`data`](super) module docs for the wider rationale.

use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{
        Deref,
        DerefMut,
    },
};

use crate::mutex::Mutex as RawMutex;

/// A mutual exclusion primitive useful for protecting shared data.
pub struct Mutex<T: ?Sized> {
    inner: RawMutex,
    data: UnsafeCell<T>,
}

// SAFETY: the `inner` lock serialises access to `data`, so sharing the `Mutex`
// across threads is sound whenever the protected data may itself move/be shared.
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub const fn new(data: T) -> Mutex<T> {
        Mutex {
            inner: RawMutex::new(),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires the mutex, blocking the current thread until it is able to do
    /// so.
    ///
    /// An RAII guard is returned; the mutex is unlocked when the guard is
    /// dropped.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.inner.lock();
        // SAFETY: the lock was just acquired by the current thread.
        unsafe { MutexGuard::new(self) }
    }

    /// Forcibly unlocks the mutex, regardless of whether a [`MutexGuard`] is
    /// currently in scope.
    ///
    /// Pairs with [`MutexGuard::leak`] to keep the lock held across an FFI
    /// boundary, where the guard's `Drop` cannot run.
    ///
    /// # Safety
    ///
    /// * The current thread must currently own the lock.
    /// * No live [`MutexGuard`] for this mutex may exist (unless it was
    ///   intentionally leaked).
    #[inline]
    pub unsafe fn force_unlock(&self) {
        self.inner.unlock();
    }

    /// Returns a raw pointer to the data protected by the mutex.
    ///
    /// Useful after the guard has been deliberately leaked and FFI code still
    /// needs to reach the data; dereferencing it is unsafe because the compiler
    /// cannot prove the absence of data races.
    #[inline]
    pub fn data_ptr(&self) -> *mut T {
        self.data.get()
    }

    /// Returns `true` if the mutex is currently held by the calling thread.
    #[inline]
    pub fn is_locked_by_current_thread(&self) -> bool {
        self.inner.is_locked_by_current_thread()
    }
}

/// RAII guard granting scoped access to the data behind a [`Mutex`].
#[must_use = "if unused the Mutex will immediately unlock"]
#[clippy::has_significant_drop]
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a Mutex<T>,
    _marker: PhantomData<*const ()>,
}

// SAFETY: the guard only hands out references to `T`, so it is `Sync` exactly
// when shared references to `T` are.
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<'mutex, T: ?Sized> MutexGuard<'mutex, T> {
    /// Wraps an already-locked `lock` in a guard.
    ///
    /// # Safety
    ///
    /// The current thread must own `lock` and no other guard for it may exist.
    unsafe fn new(lock: &'mutex Mutex<T>) -> MutexGuard<'mutex, T> {
        MutexGuard {
            lock,
            _marker: PhantomData,
        }
    }

    /// Leaks the guard, returning a mutable reference to the protected data
    /// **without** unlocking the mutex.
    ///
    /// The caller becomes responsible for eventually releasing the mutex with
    /// [`Mutex::force_unlock`]. Used to keep a lock held across a C FFI
    /// boundary, where `Drop` cannot run.
    #[inline]
    pub fn leak(self) -> &'mutex mut T {
        let ptr = self.lock.data.get();
        // Skip the guard's `Drop`, leaving the mutex locked.
        core::mem::forget(self);
        // SAFETY: the guard owned exclusive access to the data; forgetting it
        // keeps the mutex locked, so this reference stays unique.
        unsafe { &mut *ptr }
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the guard's existence proves the mutex is locked.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the guard's existence proves the mutex is locked and the
        // `&mut self` borrow makes this the unique reference to the data.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.lock.inner.unlock();
    }
}
