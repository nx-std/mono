//! # Once
//!
//! A synchronization primitive which can be used to run a one-time global
//! initialization. Unlike the standard library version this implementation is
//! **non-poisoning**: it has no poisoned state. Because the workspace builds
//! with `panic = "abort"`, a panic inside the initializer aborts the whole
//! process, so a half-completed `Once` is never observable by another thread.
//!
//! The API is intentionally kept very close to the one used inside the Rust
//! standard library's *platform layer* (see
//! <https://doc.rust-lang.org/src/std/sys/sync/once/>). The main differences
//! are:
//!
//! * The API has **no poisoning support** whatsoever – the initializer closure
//!   either runs to completion or panics, and a panic aborts the process.
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

use core::{
    convert::Infallible,
    sync::atomic::{
        AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
    },
};

use super::{Condvar, Mutex};

/// The progress of a [`Once`]'s initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum OnceState {
    /// No initialization has run yet, and no thread is currently using the Once.
    Incomplete = 0,
    /// Some thread is currently attempting to run initialization. It may succeed,
    /// so all future threads need to wait for it to finish.
    Running = 1,
    /// Initialization has completed and all future calls should finish immediately.
    Complete = 2,
}

impl TryFrom<usize> for OnceState {
    type Error = UnknownOnceStateError;

    /// Decodes the word held in the atomic cell.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownOnceStateError`] if the word is not a state discriminant.
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Incomplete),
            1 => Ok(Self::Running),
            2 => Ok(Self::Complete),
            unknown => Err(UnknownOnceStateError(unknown)),
        }
    }
}

/// An error indicating that a word names no [`Once`] initialization state.
#[derive(Debug, thiserror::Error)]
#[error("Unknown Once state {0}")]
struct UnknownOnceStateError(usize);

/// Reads the state out of `state`.
///
/// The cell is private to [`Once`] and every write stores an `OnceState` discriminant, so the
/// decode cannot fail. `Running` is the safe reading if it ever did: a thread that observed it
/// waits for the initializer rather than becoming one or treating the value as published.
#[inline]
fn load_state(state: &AtomicUsize, ordering: core::sync::atomic::Ordering) -> OnceState {
    OnceState::try_from(state.load(ordering)).unwrap_or(OnceState::Running)
}

/// The public representation of a `Once`.
///
/// A `Once` may be placed in static storage and safely used from multiple
/// threads concurrently.
pub struct Once {
    state: AtomicUsize,
    mutex: Mutex,
    cvar: Condvar,
}

impl Once {
    /// Creates a new `Once` in the [`OnceState::Incomplete`] state.
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(OnceState::Incomplete as usize),
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
        load_state(&self.state, Acquire) == OnceState::Complete
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
        while load_state(&self.state, Relaxed) != OnceState::Complete {
            // Ignore potential error codes; we only care about waking up.
            let _ = self.cvar.wait(&self.mutex);
        }
        self.mutex.unlock();
    }

    /// Executes the given closure exactly **once**. Subsequent calls block
    /// until the first invocation completes (or has completed already).
    ///
    /// Calling `call_once` (or [`call_once_try`](Self::call_once_try))
    /// reentrantly from within `f` deadlocks the calling thread.
    #[inline]
    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        // The infallible variant is the fallible one whose initializer never
        // errors. Sharing the implementation keeps a single state machine, so
        // the two entry points cannot drift apart in their waiter handling.
        let _ = self.call_once_try(|| {
            f();
            Ok::<(), Infallible>(())
        });
    }

    /// Executes the given fallible closure exactly **once**. If the
    /// closure returns `Err(e)` the internal state is reset to
    /// [`OnceState::Incomplete`] so that another attempt can be made at a
    /// later time.
    ///
    /// All other semantics are identical to [`call_once`]. Only the thread
    /// that actually executes the initializer receives the error. Waiting
    /// threads will simply observe the `Incomplete` state and may try
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

            // Every state transition happens under `self.mutex`, so these
            // `Relaxed` loads are ordered by the mutex acquire/release. The
            // `Acquire` load in `is_completed` covers the lock-free fast path.
            match load_state(&self.state, Relaxed) {
                OnceState::Incomplete => {
                    // Become the initializer.
                    self.state.store(OnceState::Running as usize, Relaxed);
                    self.mutex.unlock();

                    // Run user initialisation code outside the critical section.
                    let result = (init_opt.take().unwrap())();

                    // Update state and wake waiters.
                    self.mutex.lock();
                    match result {
                        Ok(()) => {
                            // Success – publish the completion.
                            self.state.store(OnceState::Complete as usize, Release);
                            self.cvar.wake_all();
                            self.mutex.unlock();
                            return Ok(());
                        }
                        Err(e) => {
                            // Failure – roll back to INCOMPLETE so that the
                            // next caller can try again.
                            self.state.store(OnceState::Incomplete as usize, Relaxed);
                            self.cvar.wake_all();
                            self.mutex.unlock();
                            return Err(e);
                        }
                    }
                }
                OnceState::Running => {
                    // Somebody else is running – wait until they finish.
                    while load_state(&self.state, Relaxed) == OnceState::Running {
                        let _ = self.cvar.wait(&self.mutex);
                    }

                    // Re-check the state: if it is Complete we are done,
                    // otherwise it is Incomplete and we must loop and try
                    // again.
                    if load_state(&self.state, Relaxed) == OnceState::Complete {
                        self.mutex.unlock();
                        return Ok(());
                    }

                    // State is Incomplete – another round.
                    self.mutex.unlock();
                }
                OnceState::Complete => {
                    self.mutex.unlock();
                    return Ok(());
                }
            }
        }
    }
}
