//! # OnceLock
//!
//! A thread-safe cell that can be written to exactly once and thereafter
//! read many times without additional synchronisation cost.  Internally it is
//! backed by the [`Once`] primitive from `nx-sys-sync` which guarantees that
//! the initialisation closure or `set` operation runs at most once across all
//! threads.
//!
//! The API is modelled after `std::sync::OnceLock` (Rust 1.70) but omits all
//! poisoning semantics – if the initialiser panics the entire program aborts
//! in typical `no_std` fashion.

use core::{cell::UnsafeCell, fmt, mem::MaybeUninit};

use nx_sys_sync::Once;

/// A cell which can be written to only once.
///
/// This type is a thin wrapper around an [`UnsafeCell`] containing an
/// uninitialised value together with a [`Once`] that serialises
/// initialisation.
///
/// All read-only operations (`get`, `wait`, …) are lock-free after the cell has
/// been initialised.
pub struct OnceLock<T> {
    once: Once,
    value: UnsafeCell<MaybeUninit<T>>,
}

// `T` is only accessed after initialisation has completed which implies a
// happens-before relationship with the writers, hence the usual Send/Sync
// bounds.
unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T> OnceLock<T> {
    /// Creates an uninitialised cell.
    #[inline]
    pub const fn new() -> Self {
        Self {
            once: Once::new(),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Returns a shared reference to the stored value if it has been
    /// initialised.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.is_initialised() {
            // SAFETY: The `Once` guarantees that the value has been fully
            // initialised and no mutable references exist at this point.
            Some(unsafe { (&*self.value.get()).assume_init_ref() })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the stored value if it is initialised.
    /// Requires a mutable borrow of the `OnceLock` which guarantees exclusive
    /// access.
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.is_initialised() {
            // SAFETY: Unique &mut access guaranteed by `&mut self`.
            Some(unsafe { (&mut *self.value.get()).assume_init_mut() })
        } else {
            None
        }
    }

    /// Attempts to set the contents of the cell to `value`.
    ///
    /// Returns `Err(value)` if the cell had already been initialised by this or
    /// another thread.
    pub fn set(&self, value: T) -> Result<(), T> {
        if self.is_initialised() {
            return Err(value);
        }

        // We move the value into the closure only if we become the
        // initialiser.  This avoids double-drop in case another thread wins.
        let mut value_opt = Some(value);
        let slot = self.value.get();
        let mut did_run = false;
        self.once.call_once(|| {
            // SAFETY: We are the only thread inside the closure.
            unsafe {
                (*slot).write(value_opt.take().unwrap());
            }
            did_run = true;
        });

        match did_run {
            true => Ok(()),
            false => Err(value_opt.unwrap()),
        }
    }

    /// Returns a reference to the value, initialising it with `init` if needed.
    pub fn get_or_init<F>(&self, init: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(val) = self.get() {
            return val;
        }

        let slot = self.value.get();
        let mut init_opt = Some(init);
        self.once.call_once(|| {
            let value = (init_opt.take().unwrap())();
            unsafe {
                (*slot).write(value);
            }
        });
        // Either we ran the closure or another thread did – in both cases the
        // cell is initialised now.
        unsafe { (&*slot).assume_init_ref() }
    }

    /// Same as [`get_or_init`] but the initialiser may fail.
    pub fn get_or_try_init<F, E>(&self, init: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if let Some(val) = self.get() {
            return Ok(val);
        }

        let slot = self.value.get();
        let mut init_opt = Some(init);

        // Run (or wait for) initialisation.
        self.once.call_once_try(|| {
            // Execute user initialiser.
            match (init_opt.take().unwrap())() {
                Ok(value) => {
                    // SAFETY: We are the unique initialiser.
                    unsafe {
                        (*slot).write(value);
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        })?;

        // Once we get here the value is definitely initialised.
        Ok(unsafe { (&*slot).assume_init_ref() })
    }

    /// Takes the value out of the cell, leaving it uninitialised again.
    ///
    /// Returns `None` if the cell was never initialised.
    pub fn take(&mut self) -> Option<T> {
        if self.is_initialised() {
            // SAFETY: We have unique access which guarantees that there are
            //          no outstanding `&T` references created by other threads.
            //          Nevertheless _existing_ references from the same thread
            //          would be invalidated, so callers must uphold that
            //          guarantee themselves.
            self.once = Once::new();
            Some(unsafe { (&mut *self.value.get()).assume_init_read() })
        } else {
            None
        }
    }

    #[inline]
    fn is_initialised(&self) -> bool {
        self.once.is_completed()
    }
}

impl<T> Default for OnceLock<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("OnceLock");
        match self.get() {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

impl<T: Clone> Clone for OnceLock<T> {
    fn clone(&self) -> Self {
        let cell = Self::new();
        if let Some(val) = self.get() {
            let _ = cell.set(val.clone());
        }
        cell
    }
}

impl<T> From<T> for OnceLock<T> {
    fn from(value: T) -> Self {
        let cell = Self::new();
        let _ = cell.set(value);
        cell
    }
}

impl<T: PartialEq> PartialEq for OnceLock<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}
impl<T: Eq> Eq for OnceLock<T> {}
