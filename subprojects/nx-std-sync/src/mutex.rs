//! # Mutex

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
};

use nx_sys_sync as sys;

pub use crate::result::{TryLockError, TryLockResult};

/// A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available. The
/// mutex can be created via a [`new`] constructor. Each mutex has a type parameter
/// which represents the data that it is protecting. The data can only be accessed
/// through the RAII guards returned from [`lock`] and [`try_lock`], which
/// guarantees that the data is only ever accessed when the mutex is locked.
pub struct Mutex<T: ?Sized> {
    inner: sys::Mutex,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub const fn new(data: T) -> Mutex<T> {
        Mutex {
            inner: sys::Mutex::new(),
            data: UnsafeCell::new(data),
        }
    }

    /// Creates a new `Mutex` from an existing raw [`nx_sys_sync::Mutex`].
    ///
    /// This is primarily intended for interoperability with foreign code that
    /// exposes a raw libnx mutex.  The returned high-level `Mutex` takes
    /// ownership of the provided raw mutex; after calling this function you
    /// must not manipulate `inner` directly.
    ///
    /// # Safety
    ///
    /// * `inner` must be a valid, properly initialised `nx_sys_sync::Mutex`.
    /// * No other code may continue to access `inner` for the lifetime of the
    ///   created `Mutex`.
    /// * The calling thread must ensure that there are no outstanding locks on
    ///   `inner` that could violate Rust's aliasing rules.
    #[inline]
    pub const unsafe fn from_raw(inner: sys::Mutex, data: T) -> Mutex<T>
    where
        T: Sized,
    {
        Mutex {
            inner,
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires a mutex, blocking the current thread until it is able to do so.
    ///
    /// This function will block the local thread until it is available to acquire
    /// the mutex. Upon returning, the thread is the only thread with the lock
    /// held. An RAII guard is returned to allow scoped unlock of the lock. When
    /// the guard goes out of scope, the mutex will be unlocked.
    ///
    /// The exact behavior on locking a mutex in the thread which already holds
    /// the lock is left unspecified. However, this function will not return on
    /// the second call (it might panic or deadlock, for example).
    pub fn lock(&self) -> MutexGuard<'_, T> {
        unsafe {
            self.inner.lock();
            MutexGuard::new(self)
        }
    }

    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then [`Err`] is returned.
    /// Otherwise, an RAII guard is returned. The lock will be unlocked when the
    /// guard is dropped.
    ///
    /// This function does not block.
    ///
    /// # Errors
    ///
    /// If the mutex could not be acquired because it is already locked, then
    /// this call will return the [`WouldBlock`] error.
    ///
    /// [`WouldBlock`]: TryLockError::WouldBlock
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        unsafe {
            if self.inner.try_lock() {
                Ok(MutexGuard::new(self))
            } else {
                Err(TryLockError::WouldBlock)
            }
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.data.into_inner()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `Mutex` mutably, no actual locking needs to
    /// take place -- the mutable borrow statically guarantees no locks exist.
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// Creates a new `MutexGuard` **without** verifying that the lock is currently held.
    ///
    /// This is useful when the lock has already been acquired through some other
    /// mechanism (for example, after calling [`try_lock`] and keeping the lock
    /// across an FFI boundary) and you need to reconstruct a guard so that the
    /// mutex is unlocked automatically on drop.
    ///
    /// # Safety
    ///
    /// * The current thread must logically own the lock.
    /// * There must be no other active `MutexGuard` instances for this mutex
    ///   (unless they were previously leaked with `core::mem::forget`).
    /// * Violating these requirements results in **undefined behaviour**.
    #[inline]
    pub unsafe fn make_guard_unchecked(&self) -> MutexGuard<'_, T> {
        // SAFETY: Caller guarantees that the mutex is already locked and that
        // no other guard exists.
        unsafe { MutexGuard::new(self) }
    }

    /// Forcibly unlocks the mutex, regardless of whether a `MutexGuard` is
    /// currently in scope.
    ///
    /// This can be combined with `core::mem::forget` to keep the lock for an
    /// arbitrary duration without holding a guard value, which is sometimes
    /// required when interfacing with foreign code.
    ///
    /// # Safety
    ///
    /// * The current thread must currently own the lock.
    /// * No `MutexGuard` instances for this mutex may exist (unless they have
    ///   been intentionally leaked with `core::mem::forget`).
    /// * Unlocking a mutex that is not locked or is locked by another thread
    ///   is **undefined behaviour**.
    #[inline]
    pub unsafe fn force_unlock(&self) {
        self.inner.unlock();
    }

    /// Returns a raw pointer to the underlying data protected by the mutex.
    ///
    /// This can be handy when the guard has been purposely leaked and you still
    /// want to access the data (e.g. from FFI code).  Dereferencing the pointer
    /// is inherently unsafe because the compiler cannot guarantee the absence
    /// of data races.
    #[inline]
    pub fn data_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T> From<T> for Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    /// This is equivalent to [`Mutex::new`].
    fn from(t: T) -> Self {
        Mutex::new(t)
    }
}

impl<T: ?Sized + Default> Default for Mutex<T> {
    /// Creates a `Mutex<T>`, with the `Default` value for T.
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Mutex");
        match self.try_lock() {
            Ok(guard) => {
                d.field("data", &&*guard);
            }
            Err(TryLockError::WouldBlock) => {
                d.field("data", &format_args!("<locked>"));
            }
        }
        d.finish_non_exhaustive()
    }
}

#[must_use = "if unused the Mutex will immediately unlock"]
#[clippy::has_significant_drop]
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a Mutex<T>,
    _marker: core::marker::PhantomData<*const ()>,
}

unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<'mutex, T: ?Sized> MutexGuard<'mutex, T> {
    unsafe fn new(lock: &'mutex Mutex<T>) -> MutexGuard<'mutex, T> {
        MutexGuard {
            lock,
            _marker: Default::default(),
        }
    }

    /// Leaks the mutex guard and returns a mutable reference to the data it
    /// protects **without** unlocking the mutex.
    ///
    /// After calling this function the caller is responsible for eventually
    /// unlocking the mutex manually (for example with
    /// [`Mutex::force_unlock`](super::mutex::Mutex::force_unlock)) once the
    /// mutable reference is no longer used. Failing to do so will leave the
    /// mutex permanently locked and likely dead-lock future lock attempts.
    ///
    /// The behaviour mimics [`lock_api::MutexGuard::leak`] from the *lock_api*
    /// crate.
    #[inline]
    pub fn leak(self) -> &'mutex mut T {
        // SAFETY: `this` provides exclusive access to the data and we
        // intentionally skip the guard's `Drop` implementation, leaving the
        // mutex locked.
        let ptr = self.lock.data.get();
        core::mem::forget(self);
        unsafe { &mut *ptr }
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.lock.inner.unlock();
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

pub(crate) fn guard_lock<'a, T: ?Sized>(guard: &MutexGuard<'a, T>) -> &'a sys::Mutex {
    &guard.lock.inner
}
