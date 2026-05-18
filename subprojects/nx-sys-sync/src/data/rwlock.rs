//! Data-protecting read/write lock: the allocation-free
//! [`crate::rwlock::RwLock`] bundled with the data it guards, handing out
//! scoped access through RAII read and write guards.
//!
//! See the [`data`](super) module docs for the wider rationale.

use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::rwlock::RwLock as RawRwLock;

/// A reader-writer lock useful for protecting shared data.
///
/// Allows any number of readers or a single writer at a time. Readers obtain a
/// shared [`RwLockReadGuard`]; the writer obtains an exclusive
/// [`RwLockWriteGuard`].
pub struct RwLock<T: ?Sized> {
    inner: RawRwLock,
    data: UnsafeCell<T>,
}

// SAFETY: the `inner` lock serialises writers and excludes them from readers,
// so sharing the `RwLock` across threads is sound whenever the protected data
// may itself move (`Send`) and be observed concurrently by readers (`Sync`).
unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// Creates a new read/write lock in an unlocked state ready for use.
    #[inline]
    pub const fn new(data: T) -> RwLock<T> {
        RwLock {
            inner: RawRwLock::new(),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Acquires shared read access, blocking the current thread until it is
    /// able to do so.
    ///
    /// An RAII guard is returned; the read lock is released when the guard is
    /// dropped.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read_lock();
        // SAFETY: the read lock was just acquired by the current thread.
        unsafe { RwLockReadGuard::new(self) }
    }

    /// Acquires exclusive write access, blocking the current thread until it is
    /// able to do so.
    ///
    /// An RAII guard is returned; the write lock is released when the guard is
    /// dropped.
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.inner.write_lock();
        // SAFETY: the write lock was just acquired by the current thread.
        unsafe { RwLockWriteGuard::new(self) }
    }
}

/// RAII guard granting scoped shared access to the data behind a [`RwLock`].
#[must_use = "if unused the RwLock will immediately unlock"]
#[clippy::has_significant_drop]
pub struct RwLockReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
    _marker: PhantomData<*const ()>,
}

// SAFETY: the guard only hands out shared references to `T`, so it is `Sync`
// exactly when shared references to `T` are.
unsafe impl<T: ?Sized + Sync> Sync for RwLockReadGuard<'_, T> {}

impl<'rwlock, T: ?Sized> RwLockReadGuard<'rwlock, T> {
    /// Wraps an already read-locked `lock` in a guard.
    ///
    /// # Safety
    ///
    /// The current thread must hold a read lock on `lock`.
    unsafe fn new(lock: &'rwlock RwLock<T>) -> RwLockReadGuard<'rwlock, T> {
        RwLockReadGuard {
            lock,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the guard's existence proves a read lock is held, so no
        // writer can be mutating the data concurrently.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.lock.inner.read_unlock();
    }
}

/// RAII guard granting scoped exclusive access to the data behind a [`RwLock`].
#[must_use = "if unused the RwLock will immediately unlock"]
#[clippy::has_significant_drop]
pub struct RwLockWriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
    _marker: PhantomData<*const ()>,
}

// SAFETY: the guard hands out references to `T`, so it is `Sync` exactly when
// shared references to `T` are.
unsafe impl<T: ?Sized + Sync> Sync for RwLockWriteGuard<'_, T> {}

impl<'rwlock, T: ?Sized> RwLockWriteGuard<'rwlock, T> {
    /// Wraps an already write-locked `lock` in a guard.
    ///
    /// # Safety
    ///
    /// The current thread must hold the write lock on `lock` and no other guard
    /// for it may exist.
    unsafe fn new(lock: &'rwlock RwLock<T>) -> RwLockWriteGuard<'rwlock, T> {
        RwLockWriteGuard {
            lock,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the guard's existence proves the write lock is held.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the guard's existence proves the write lock is held and the
        // `&mut self` borrow makes this the unique reference to the data.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.lock.inner.write_unlock();
    }
}
