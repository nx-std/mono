//! Owned and borrowed session handles.
//!
//! A session handle is a kernel resource that must be closed exactly once. The raw
//! [`SessionHandle`] is `Copy`, so a type that owns one and closes it on drop cannot, by itself,
//! stop a second owner being made from a copy: that is a double `svcCloseHandle`, and between the
//! two closes the kernel may have reused the handle number for an unrelated object.
//!
//! The two types here separate the two roles the way `std` separates `OwnedFd` from `BorrowedFd`:
//!
//! - [`OwnedSessionHandle`] is neither `Copy` nor `Clone` and closes the session when dropped.
//!   Only a move produces a second one, and a move leaves no first.
//! - [`BorrowedSessionHandle`] is `Copy`, has no destructor, and borrows its owner, so it cannot
//!   outlive the session it names nor be turned back into something that closes.
//!
//! Everything that merely *uses* a session takes the borrowed form. Both types can yield the raw
//! handle back - [`BorrowedSessionHandle::to_handle`] for a call that needs the number, and
//! [`OwnedSessionHandle::into_handle`] to hand the obligation on - but neither yields an owner:
//! closing a session twice from there would mean calling an SVC directly rather than dropping a
//! value.

use core::{marker::PhantomData, mem::ManuallyDrop};

use nx_svc::ipc::Handle as SessionHandle;

use super::control;

/// A session handle this process owns and must close exactly once.
///
/// Dropping it closes the session. This is the only type in the crate whose destructor does so,
/// which is what makes "closed exactly once" a property of the move checker rather than of a
/// convention every holder has to keep.
#[derive(Debug)]
pub struct OwnedSessionHandle(SessionHandle);

impl OwnedSessionHandle {
    /// Adopts a session handle as the sole owner.
    ///
    /// This takes the handle value a server reply or an SVC hands back, which is the point at
    /// which a session acquires an owner: everything downstream borrows.
    ///
    /// The caller must ensure `handle` names a live session that this process owns and that
    /// nothing else will close, since this value closes it on drop. A second owner sends its
    /// close against a handle number the kernel may have reused, so an unrelated session is
    /// torn down and requests on it are answered by whatever now holds that number. That is a
    /// resource error rather than undefined behaviour, which is why this is a safe function.
    #[inline]
    pub const fn from_handle_unchecked(handle: SessionHandle) -> Self {
        Self(handle)
    }

    /// Borrows the handle for the duration of `&self`.
    #[inline]
    pub const fn as_borrowed(&self) -> BorrowedSessionHandle<'_> {
        BorrowedSessionHandle {
            handle: self.0,
            owner: PhantomData,
        }
    }

    /// Gives up ownership, returning the handle without closing it.
    ///
    /// The caller becomes responsible for closing it exactly once.
    #[inline]
    pub fn into_handle(self) -> SessionHandle {
        // The handle is being handed on rather than released, so the destructor that would
        // close it is suppressed.
        let this = ManuallyDrop::new(self);
        this.0
    }
}

impl Drop for OwnedSessionHandle {
    fn drop(&mut self) {
        control::close_session(self.as_borrowed());
    }
}

/// A session handle borrowed from its owner.
///
/// Copyable and destructor-free: closing is the owner's job, and the lifetime keeps a borrow from
/// outliving the session it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowedSessionHandle<'h> {
    handle: SessionHandle,
    owner: PhantomData<&'h OwnedSessionHandle>,
}

impl BorrowedSessionHandle<'_> {
    /// Borrows a handle whose owner is not an [`OwnedSessionHandle`].
    ///
    /// The caller must ensure the session outlives `'h`. This exists for sessions owned outside
    /// this crate, such as one libnx opened and still closes itself; borrowing a session that is
    /// closed while the borrow is live leaves it naming a handle number the kernel may have
    /// reused, and requests sent through it reach whatever now holds that number.
    #[inline]
    pub const fn from_handle_unchecked(handle: SessionHandle) -> Self {
        Self {
            handle,
            owner: PhantomData,
        }
    }

    /// Returns the session handle for an SVC or IPC call.
    #[inline]
    pub const fn to_handle(&self) -> SessionHandle {
        self.handle
    }
}
