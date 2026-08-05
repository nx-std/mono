//! Shared pieces of the C boundary.

use core::cell::UnsafeCell;

/// Module the libnx result codes below belong to.
const MODULE_LIBNX: u32 = 345;

/// Nothing is mounted under that name (`LibnxError_NotFound`).
pub const NOT_FOUND: u32 = result(9);

/// The registry has no slot left (`LibnxError_OutOfMemory`).
///
/// libnx reports a failed `AddDevice` this way, and a caller that branches on the code should not
/// have to learn a new one.
pub const OUT_OF_MEMORY: u32 = result(2);

/// The path is not one the device can act on (`LibnxError_BadInput`).
///
/// Reported for a path rejected before any command was built, which has no server result code of
/// its own to carry back.
pub const BAD_INPUT: u32 = result(11);

/// Builds a libnx result code from its description.
const fn result(description: u32) -> u32 {
    MODULE_LIBNX | (description << 9)
}

/// An [`UnsafeCell`] that may be shared, for storage only the C boundary touches.
///
/// The C entry points that hand back a pointer need somewhere stable to point at, and that
/// storage is written by one thread during a mount and read afterwards. Nothing in the crate
/// synchronizes it, which is the same guarantee libnx offers for the structure it hands back.
#[repr(transparent)]
pub struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: the cell carries no synchronization of its own; every caller reaching through `get` is
// an `unsafe` C entry point that documents what it requires.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    /// Wraps `value`.
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    /// Returns a pointer to the value.
    pub const fn get(&self) -> *mut T {
        self.0.get()
    }
}
