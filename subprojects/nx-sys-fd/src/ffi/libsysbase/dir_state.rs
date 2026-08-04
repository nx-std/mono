//! The open directory walk, stored in the state C allocates behind each iterator.
//!
//! A directory has no descriptor number, so none of the identity machinery the per-descriptor
//! operations rely on applies to it. What it does have is the iterator itself: `__diropen`
//! allocates one per open directory, along with however many bytes of private state the device
//! asked for, and hands the same pointer to every later call.
//!
//! Asking for room to hold the walk is therefore all the identity a directory shim needs, and it is
//! exactly how a C device does it. No table, no cap on how many directories may be open at once,
//! and nothing to clean up if a caller leaks an iterator that this crate never allocated.

use alloc::boxed::Box;

use super::ctypes::DirIter;
use crate::device::Dir;

/// What the state behind an iterator holds.
///
/// An `Option` rather than a bare box so that closing can empty the slot: a second close then finds
/// nothing and reports a bad iterator, instead of reconstructing a second box over an allocation
/// that is already gone. The niche in the box pointer means the `None` costs no extra space.
type State = Option<Box<dyn Dir>>;

/// Bytes of per-iterator state the shim table asks C to allocate.
pub const SIZE: usize = size_of::<State>();

/// Stores `dir` in the state behind `iter`.
///
/// # Safety
///
/// `iter` must be non-null with a `dir_struct` addressing [`SIZE`] writable bytes that hold no walk
/// yet. The bytes need not be initialized: this overwrites them without reading first.
pub unsafe fn store(iter: *mut DirIter, dir: Box<dyn Dir>) {
    // SAFETY: the caller guarantees the state is writable and large enough, and the C caller
    // allocates it at pointer alignment. `write` does not drop what was there, which is what makes
    // it correct over uninitialized bytes.
    unsafe { slot(iter).write(Some(dir)) };
}

/// Borrows the walk stored behind `iter`.
///
/// # Safety
///
/// `iter` must be null or an iterator [`store`] wrote to. The borrow lasts as long as the returned
/// reference, and the C callers operate one iterator at a time, so no second reference to the same
/// walk exists.
// The `'static` on the trait object is the one the stored `Box<dyn Dir>` already carries. Behind a
// `&mut` a trait object is invariant, so it cannot be shortened to the borrow's own lifetime and
// has to be named.
pub unsafe fn borrow<'a>(iter: *mut DirIter) -> Option<&'a mut (dyn Dir + 'static)> {
    if iter.is_null() {
        return None;
    }

    // SAFETY: the caller guarantees the state was written by `store`, so it holds a live `State`,
    // and that no other reference to it exists for the life of this borrow.
    let state: &'a mut State = unsafe { &mut *slot(iter) };
    state.as_deref_mut()
}

/// Removes the walk stored behind `iter` and returns ownership of it.
///
/// # Safety
///
/// `iter` must be null or an iterator [`store`] wrote to.
pub unsafe fn take(iter: *mut DirIter) -> Option<Box<dyn Dir>> {
    if iter.is_null() {
        return None;
    }

    // SAFETY: the caller guarantees the state was written by `store`, so it holds a live `State`.
    unsafe { (*slot(iter)).take() }
}

/// Returns where the walk is stored behind `iter`.
///
/// # Safety
///
/// `iter` must be non-null with a `dir_struct` addressing [`SIZE`] bytes.
unsafe fn slot(iter: *mut DirIter) -> *mut State {
    // SAFETY: the caller guarantees `iter` is non-null and live.
    unsafe { (*iter).dir_struct.cast::<State>() }
}
