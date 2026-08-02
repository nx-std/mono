//! Common FFI helpers shared across the `nx-rt-*` runtime crates.
//!
//! `nx-rt-core` is the single authoritative home for the kind-agnostic FFI
//! helpers: the generic error code and the `SyncUnsafeCell` static-storage
//! wrapper. The per-output-kind entry crates (`nx-rt-nro`, …) re-export these
//! from here rather than re-defining them.
//!
//! The per-error converters that used to live here are gone: every error now
//! maps itself through its own family's `ToResultCode`, so an adapter calls
//! `.to_rc()` instead of a function that knew how to take another crate's
//! error apart.

use core::cell::UnsafeCell;

/// Generic error code for FFI when no specific result code is available.
///
/// Re-exported so the adapters keep a single import site; the value is the
/// Service Framework family's fallback.
pub use crate::error::GENERIC_ERROR;

/// Wrapper to make UnsafeCell Sync for static storage.
#[repr(transparent)]
pub struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: this `unsafe impl` asserts `Sync` for *every* `T`, which is sound
// only under a usage contract the type itself does not enforce. Every
// instantiation across the `nx-rt-*` runtime is a `static` cache touched
// solely during single-threaded runtime init/exit: each cell is written
// exactly once by its owner's `*_initialize` hook (or zeroed by the matching
// `*_exit`) and read only after that publication, with no concurrent access.
// libnx guarantees a given service's `*_initialize`/`*_exit` runs once,
// single-threaded, before/after any FFI access to its cache — so no two
// accesses race. Callers placing a `SyncUnsafeCell` in `static` storage must
// uphold this no-races contract; it is an internal runtime utility, not a
// general-purpose cell.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub fn get(&self) -> *mut T {
        self.0.get()
    }
}
