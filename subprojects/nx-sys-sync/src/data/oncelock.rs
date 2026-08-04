//! Write-once cell: a value paired with the allocation-free [`Once`] that
//! serialises its single initialisation.
//!
//! See the [`data`](super) module docs for the wider rationale.

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
};

use crate::once::Once;

/// A cell which can be written to exactly once and thereafter read many times
/// without further synchronisation cost.
///
/// The cell is initialised at most once across all threads; reads after
/// initialisation are lock-free.
pub struct OnceLock<T> {
    once: Once,
    value: UnsafeCell<MaybeUninit<T>>,
}

// SAFETY: `value` is only accessed after `once` reports completion, which
// establishes a happens-before edge with the initialising thread, so the usual
// `Send`/`Sync` bounds suffice.
unsafe impl<T: Send + Sync> Sync for OnceLock<T> {}
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

    /// Returns a shared reference to the stored value, or `None` if the cell
    /// has not been initialised.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.is_initialised() {
            // SAFETY: `is_initialised` reporting `true` proves the value was
            // fully written before any thread could observe completion.
            Some(unsafe { (*self.value.get()).assume_init_ref() })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the stored value, or `None` if the cell
    /// has not been initialised.
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.is_initialised() {
            // SAFETY: the value is initialised (see above) and `&mut self`
            // makes this the unique reference to it.
            Some(unsafe { (*self.value.get()).assume_init_mut() })
        } else {
            None
        }
    }

    /// Blocks the current thread until the cell is initialised, then returns a
    /// shared reference to the stored value.
    ///
    /// Blocks indefinitely if the cell is never initialised.
    #[inline]
    pub fn wait(&self) -> &T {
        self.once.wait();
        // SAFETY: `once.wait()` only returns once initialisation has completed,
        // so the value was fully written before this thread observed it.
        unsafe { (*self.value.get()).assume_init_ref() }
    }

    /// Sets the contents of the cell to `value`.
    ///
    /// Returns `Err(value)` if the cell had already been initialised by this or
    /// another thread.
    pub fn set(&self, value: T) -> Result<(), T> {
        if self.is_initialised() {
            return Err(value);
        }

        // Hand the value to the closure; only the winning initialiser takes it,
        // so a losing thread gets its value back without a double drop.
        let mut value_opt = Some(value);
        let slot = self.value.get();
        let mut did_run = false;
        self.once.call_once(|| {
            // SAFETY: `call_once` runs this closure on a single thread with
            // exclusive access to `slot`.
            unsafe { (*slot).write(value_opt.take().unwrap()) };
            did_run = true;
        });

        match did_run {
            true => Ok(()),
            false => Err(value_opt.unwrap()),
        }
    }

    /// Returns a reference to the stored value, initialising it with `init` if
    /// the cell is still empty.
    pub fn get_or_init<F>(&self, init: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(value) = self.get() {
            return value;
        }

        let slot = self.value.get();
        let mut init_opt = Some(init);
        self.once.call_once(|| {
            let value = (init_opt.take().unwrap())();
            // SAFETY: `call_once` runs this closure on a single thread with
            // exclusive access to `slot`.
            unsafe { (*slot).write(value) };
        });
        // The cell is initialised now, whether by this thread or another.
        // SAFETY: `call_once` returned, so `slot` holds an initialised value.
        unsafe { (*slot).assume_init_ref() }
    }

    /// Like [`get_or_init`](Self::get_or_init), but the initialiser may fail.
    ///
    /// On `Err` the cell stays uninitialised, so a later call can retry.
    pub fn get_or_try_init<F, E>(&self, init: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if let Some(value) = self.get() {
            return Ok(value);
        }

        let slot = self.value.get();
        let mut init_opt = Some(init);
        self.once
            .call_once_try(|| match (init_opt.take().unwrap())() {
                Ok(value) => {
                    // SAFETY: `call_once_try` runs this closure on a single thread
                    // with exclusive access to `slot`.
                    unsafe { (*slot).write(value) };
                    Ok(())
                }
                Err(e) => Err(e),
            })?;

        // SAFETY: `call_once_try` returned `Ok`, so `slot` is initialised.
        Ok(unsafe { (*slot).assume_init_ref() })
    }

    /// Takes the value out of the cell, moving it back to the uninitialised
    /// state. Returns `None` if the cell has not been initialised.
    pub fn take(&mut self) -> Option<T> {
        if self.is_initialised() {
            // Reset the `Once` first so the cell reads as uninitialised; this
            // also stops `Drop` from reading the value a second time.
            self.once = Once::new();
            // SAFETY: the cell was initialised and `&mut self` is unique
            // access, so moving the value out is sound.
            Some(unsafe { (*self.value.get()).assume_init_read() })
        } else {
            None
        }
    }

    /// Consumes the cell, returning the stored value. Returns `None` if the
    /// cell has not been initialised.
    pub fn into_inner(mut self) -> Option<T> {
        self.take()
    }

    /// Returns `true` once the cell has been initialised.
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

impl<T> Drop for OnceLock<T> {
    /// Drops the contained value if the cell was initialised.
    #[inline]
    fn drop(&mut self) {
        if self.is_initialised() {
            // SAFETY: the cell is initialised, and `&mut self` makes this the
            // unique reference to it, so dropping the value in place is sound.
            unsafe { (*self.value.get()).assume_init_drop() };
        }
    }
}
