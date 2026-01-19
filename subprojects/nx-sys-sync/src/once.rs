//! # Once
//!
//! A synchronization primitive which can be used to run a one‐time global
//! initialization. Unlike the standard library version this implementation is
//! **non-poisoning** – if the initialization routine panics the `Once` simply
//! stays in the [`INCOMPLETE`] state and subsequent calls will endlessly spin
//! trying to acquire it. This mirrors the behaviour of a panic in `no_std`
//! environments where unwinding is typically disabled and the entire program
//! aborts anyway.
//!
//! The API is intentionally kept very close to the one used inside the Rust
//! standard library's *platform layer* (see
//! <https://doc.rust-lang.org/src/std/sys/sync/once/>). The main differences
//! are:
//!
//! * The API has **no poisoning support** whatsoever – there is no error state
//!   and the initializer closure is either executed successfully or, in the
//!   rare case of a panic, leaves the program in an undefined state (typically
//!   abort).
//! * No `OnceState`, no `poison` API surface and no `ignore_poisoning`
//!   parameters.
//!
//! Implementation details:
//!
//! • Internally the type is composed of an `AtomicUsize` that tracks the state
//!   (INCOMPLETE → RUNNING → COMPLETE) plus a [`Mutex`] ⁄ [`Condvar`] pair that
//!   lets all non-initialising threads sleep inside the kernel instead of
//!   spinning on the CPU.
//! • Only the thread that transitions the state to `RUNNING` executes the
//!   initialiser closure.  Others block on the condition variable until the
//!   state becomes `COMPLETE` and the initialiser performs `wake_all()`.
//! • Memory ordering follows the standard-library contract: the `COMPLETE`
//!   store uses `Release` semantics and the fast-path load uses `Acquire`
//!   guaranteeing that all writes performed by the initialiser are visible to
//!   threads that observe the `COMPLETE` state.
//!
//! ## Memory ordering
//!
//! All writes performed inside the initialization closure become visible to
//! other threads once the `Once` transitions into the `COMPLETE` state because
//! the store uses `Release` semantics and readers use `Acquire`.

use core::sync::atomic::{
    AtomicUsize,
    Ordering::{Acquire, Relaxed, Release},
};

use super::{Condvar, Mutex};

/// No initialization has run yet, and no thread is currently using the Once.
const INCOMPLETE: usize = 0;
/// Some thread is currently attempting to run initialization. It may succeed,
/// so all future threads need to wait for it to finish.
const RUNNING: usize = 1;
/// Initialization has completed and all future calls should finish immediately.
const COMPLETE: usize = 2;

/// The public representation of a `Once`.
///
/// A `Once` may be placed in static storage and safely used from multiple
/// threads concurrently.
#[repr(C)]
pub struct Once {
    state: AtomicUsize,
    mutex: Mutex,
    cvar: Condvar,
}

impl Once {
    /// Creates a new `Once` in the [`INCOMPLETE`] state.
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(INCOMPLETE),
            mutex: Mutex::new(),
            cvar: Condvar::new(),
        }
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}

impl Once {
    /// Returns `true` if the initialization has already run to completion.
    #[inline]
    pub fn is_completed(&self) -> bool {
        self.state.load(Acquire) == COMPLETE
    }

    /// Blocks the current thread until the `Once` has finished initialising.
    ///
    /// Internally this acquires the internal mutex and puts the thread to sleep
    /// on a condition variable, so it does **not** burn CPU time while waiting.
    #[inline]
    pub fn wait(&self) {
        // Fast path: completed.
        if self.is_completed() {
            return;
        }

        // Slow path – block on the condition variable until `COMPLETE`.
        self.mutex.lock();
        while self.state.load(Relaxed) != COMPLETE {
            // Ignore potential error codes; we only care about waking up.
            let _ = self.cvar.wait(&self.mutex);
        }
        self.mutex.unlock();
    }

    /// Executes the given closure exactly **once**. Subsequent calls block
    /// until the first invocation completes (or has completed already).
    #[inline]
    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        // Fast-path: already initialised.
        if self.is_completed() {
            return;
        }

        // Slow path – coordinate through mutex + condvar.
        self.mutex.lock();

        match self.state.load(Relaxed) {
            INCOMPLETE => {
                // Become the initializer.
                self.state.store(RUNNING, Relaxed);
                self.mutex.unlock();

                // Run user initialization code outside the critical section.
                f();

                // Mark as complete and wake waiters.
                self.mutex.lock();
                self.state.store(COMPLETE, Release);
                self.cvar.wake_all();
                self.mutex.unlock();
            }
            RUNNING => {
                // Somebody else is running – wait until they are done.
                while self.state.load(Relaxed) != COMPLETE {
                    let _ = self.cvar.wait(&self.mutex);
                }
                self.mutex.unlock();
            }
            COMPLETE => {
                // Became complete while we were locking.
                self.mutex.unlock();
            }
            _ => unreachable!(),
        }
    }

    /// Executes the given fallible closure exactly **once**. If the
    /// closure returns `Err(e)` the internal state is reset to
    /// [`INCOMPLETE`] so that another attempt can be made at a later
    /// time.
    ///
    /// All other semantics are identical to [`call_once`]. Only the thread
    /// that actually executes the initializer receives the error. Waiting
    /// threads will simply observe the `INCOMPLETE` state and may try
    /// again.
    #[inline]
    pub fn call_once_try<F, E>(&self, f: F) -> Result<(), E>
    where
        F: FnOnce() -> Result<(), E>,
    {
        // Fast-path: already initialised.
        if self.is_completed() {
            return Ok(());
        }

        // We have to keep the closure in an `Option` because in the event we
        // end up waiting for another thread we need to be able to retry the
        // call once the state goes back to `INCOMPLETE`.
        //
        // NOTE: Using a loop here keeps the implementation concise without
        // the need for extra helper functions.
        let mut init_opt = Some(f);

        loop {
            self.mutex.lock();

            match self.state.load(Relaxed) {
                INCOMPLETE => {
                    // Become the initializer.
                    self.state.store(RUNNING, Relaxed);
                    self.mutex.unlock();

                    // Run user initialisation code outside the critical section.
                    let result = (init_opt.take().unwrap())();

                    // Update state and wake waiters.
                    self.mutex.lock();
                    match result {
                        Ok(()) => {
                            // Success – publish the completion.
                            self.state.store(COMPLETE, Release);
                            self.cvar.wake_all();
                            self.mutex.unlock();
                            return Ok(());
                        }
                        Err(e) => {
                            // Failure – roll back to INCOMPLETE so that the
                            // next caller can try again.
                            self.state.store(INCOMPLETE, Relaxed);
                            self.cvar.wake_all();
                            self.mutex.unlock();
                            return Err(e);
                        }
                    }
                }
                RUNNING => {
                    // Somebody else is running – wait until they finish.
                    while self.state.load(Relaxed) == RUNNING {
                        let _ = self.cvar.wait(&self.mutex);
                    }

                    // Re-check the state: if it is COMPLETE we are done,
                    // otherwise it is INCOMPLETE and we must loop and try
                    // again.
                    if self.state.load(Relaxed) == COMPLETE {
                        self.mutex.unlock();
                        return Ok(());
                    }

                    // State is INCOMPLETE – another round.
                    self.mutex.unlock();
                }
                COMPLETE => {
                    self.mutex.unlock();
                    return Ok(());
                }
                _ => unreachable!(),
            }
        }
    }
}

// SAFETY: `Once` can be safely shared between threads because all interior
// mutability is guarded by atomic operations.
unsafe impl Send for Once {}
unsafe impl Sync for Once {}
