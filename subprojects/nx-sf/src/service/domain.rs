//! Domain root and per-object handles.
//!
//! A [`Domain`] is an IPC session that has been promoted via CMIF control
//! request 0 and multiplexes multiple server-side objects over a single
//! kernel handle. Each named object inside the domain is accessed through
//! a [`DomainObject`], which borrows the parent `Domain` so the borrow
//! checker prevents using the object after the domain has been dropped.
//!
//! Dropping a `Domain` closes the underlying session (the server cascades
//! object close on its side). Dropping a `DomainObject` sends a per-object
//! close request on the parent session without touching the handle.

use core::{marker::PhantomData, mem::ManuallyDrop};

use super::{
    control::{self, CloneObjectError, CopyFromDomainError},
    dispatch::DomainDispatch,
    handle::{BorrowedSessionHandle, OwnedSessionHandle},
    session::Session,
};
use crate::cmif::ObjectId;

/// Owned domain root: a session converted via CMIF control request 0.
#[derive(Debug)]
pub struct Domain {
    handle: OwnedSessionHandle,
    pointer_buffer_size: u16,
}

impl Domain {
    /// Wraps a handle that has already been converted to a domain.
    ///
    /// Taking an [`OwnedSessionHandle`] is what makes this a safe function: ownership is
    /// established once, where the handle is adopted, so this cannot manufacture a second
    /// closer for a session something else already owns. The caller must still ensure the
    /// server has converted the session to a domain and that `pointer_buffer_size` reflects
    /// the server's value; a wrong size mis-sizes pointer buffers rather than faulting.
    #[inline]
    pub fn new_unchecked(handle: OwnedSessionHandle, pointer_buffer_size: u16) -> Self {
        Self {
            handle,
            pointer_buffer_size,
        }
    }

    /// Builds a `Domain`-typed alias of this domain that must not be dropped.
    ///
    /// This is the one hole left in the ownership split [`OwnedSessionHandle`] otherwise
    /// closes: the alias holds a second owner of the same handle, and safe code can reach the
    /// double close through [`ManuallyDrop::into_inner`]. It exists only for callers that need
    /// a field of type `Domain` and cannot hold a borrow, which today is `nx-service-applet`'s
    /// self-referential `Proxy<R>`. Prefer [`as_borrowed`](Self::as_borrowed), which cannot be
    /// dropped wrongly at all.
    // TODO: Remove once `nx-service-applet` stores object ids and builds its sub-interface
    //       views on demand, at which point every caller can hold a `DomainRef` instead.
    #[inline]
    pub fn alias(&self) -> ManuallyDrop<Self> {
        ManuallyDrop::new(Self {
            // SAFETY: The `ManuallyDrop` suppresses the alias's destructor, so the owner this
            // borrows from remains the only closer for as long as the caller upholds the
            // must-not-be-dropped contract above.
            handle: OwnedSessionHandle::from_handle_unchecked(
                self.handle.as_borrowed().to_handle(),
            ),
            pointer_buffer_size: self.pointer_buffer_size,
        })
    }

    /// Borrows the domain as a non-owning view.
    ///
    /// The view dispatches exactly as the domain does but has no destructor, so it cannot close
    /// the session; the lifetime keeps it from outliving the owner.
    #[inline]
    pub fn as_borrowed(&self) -> DomainRef<'_> {
        DomainRef {
            handle: self.handle.as_borrowed(),
            pointer_buffer_size: self.pointer_buffer_size,
            owner: PhantomData,
        }
    }

    /// Transfers the kernel handle out of the `Domain` without closing it.
    /// Used at the FFI boundary.
    #[inline]
    pub fn into_handle(self) -> OwnedSessionHandle {
        let this = ManuallyDrop::new(self);
        // SAFETY: `this` is never dropped and never read again, so the handle moves out once.
        unsafe { core::ptr::read(&this.handle) }
    }

    /// Returns the underlying session handle without transferring ownership.
    #[inline]
    pub fn handle(&self) -> BorrowedSessionHandle<'_> {
        self.handle.as_borrowed()
    }

    /// Returns the server's pointer-buffer size.
    #[inline]
    pub fn pointer_buffer_size(&self) -> u16 {
        self.pointer_buffer_size
    }

    /// Borrows the domain to address a named object inside it. No IPC is
    /// issued.
    ///
    /// Crate-internal: the only legitimate source of fresh
    /// [`DomainObject`]s is [`DomainDispatch::send`], which calls this once
    /// per server-emitted [`ObjectId`]. External callers that need a
    /// [`DomainObject`] from a raw id must use the `unsafe`
    /// [`open_object_raw`](Self::open_object_raw) escape hatch.
    #[inline]
    pub(crate) fn open_object(&self, object_id: ObjectId) -> DomainObject<'_> {
        DomainObject {
            domain: self,
            object_id,
        }
    }

    /// Wraps a raw object id into a [`DomainObject`]. Returns `None` if the
    /// raw value is the sentinel zero (no object).
    ///
    /// # Safety
    ///
    /// The caller must ensure that `raw_object_id` corresponds to a
    /// server-side object inside this [`Domain`] that **no other live
    /// `DomainObject<'_>` already references**. Constructing a second
    /// [`DomainObject`] for the same id would double-close on Drop. Passing
    /// the id of a previously-dropped `DomainObject` is also unsound — the
    /// server may have reused the id for a different object.
    ///
    /// The safe alternative is to obtain [`DomainObject`]s from
    /// [`DomainDispatch::send`], which guarantees each server-emitted
    /// [`ObjectId`] becomes exactly one [`DomainObject`].
    #[inline]
    pub unsafe fn open_object_raw(&self, raw_object_id: u32) -> Option<DomainObject<'_>> {
        ObjectId::new(raw_object_id).map(|object_id| self.open_object(object_id))
    }

    /// Extracts a domain object into a standalone non-domain [`Session`] via
    /// CMIF control request 1.
    ///
    /// Takes `&DomainObject` (rather than a raw [`ObjectId`]) so the caller
    /// must hold a live, unique handle to the object — preventing a raw id
    /// from a dropped or aliased `DomainObject` from being laundered into a
    /// new [`Session`].
    pub fn copy_object_to_session(
        &self,
        object: &DomainObject<'_>,
    ) -> Result<Session, CopyFromDomainError> {
        let new_handle =
            control::copy_from_current_domain(self.handle.as_borrowed(), object.object_id)?;
        Ok(Session::new(new_handle, self.pointer_buffer_size))
    }

    /// Clones the underlying session via CMIF control request 2. The clone
    /// is a fresh non-domain [`Session`], matching libnx semantics.
    pub fn try_clone(&self) -> Result<Session, CloneObjectError> {
        let new_handle = control::clone_current_object(self.handle.as_borrowed())?;
        Ok(Session::new(new_handle, self.pointer_buffer_size))
    }

    /// Starts a [`DomainDispatch`] builder addressing the domain root
    /// itself. Domain-object requests should go through
    /// [`DomainObject::dispatch`] instead.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> DomainDispatch<'_> {
        DomainDispatch::new(
            self,
            self.handle.as_borrowed(),
            self.pointer_buffer_size,
            None,
            request_id,
        )
    }
}

/// Borrowed view onto a [`Domain`] that does not own its handle.
///
/// Dispatches like a `Domain` but has no destructor, so aliasing one costs neither an `unsafe`
/// block nor a `ManuallyDrop` to suppress a close that would otherwise happen twice.
#[derive(Debug, Clone, Copy)]
pub struct DomainRef<'d> {
    handle: BorrowedSessionHandle<'d>,
    pointer_buffer_size: u16,
    owner: PhantomData<&'d Domain>,
}

impl<'d> DomainRef<'d> {
    /// Returns the borrowed session handle.
    #[inline]
    pub fn handle(&self) -> BorrowedSessionHandle<'d> {
        self.handle
    }

    /// Returns the server's pointer-buffer size.
    #[inline]
    pub fn pointer_buffer_size(&self) -> u16 {
        self.pointer_buffer_size
    }
}

/// Borrowed view onto a single object inside a [`Domain`].
///
/// The `'d` lifetime ties the object to its parent domain so use-after-close
/// is a compile error. Dropping the object sends a per-object close request
/// on the parent session.
#[derive(Debug)]
pub struct DomainObject<'d> {
    domain: &'d Domain,
    object_id: ObjectId,
}

impl<'d> DomainObject<'d> {
    /// Returns the object id this view addresses.
    #[inline]
    pub fn object_id(&self) -> ObjectId {
        self.object_id
    }

    /// Returns the parent domain.
    #[inline]
    pub fn domain(&self) -> &'d Domain {
        self.domain
    }

    /// Starts a [`DomainDispatch`] builder addressing this domain object.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> DomainDispatch<'d> {
        DomainDispatch::new(
            self.domain,
            self.domain.handle.as_borrowed(),
            self.domain.pointer_buffer_size,
            Some(self.object_id),
            request_id,
        )
    }
}

impl Drop for DomainObject<'_> {
    fn drop(&mut self) {
        control::close_object(self.domain.handle.as_borrowed(), self.object_id);
    }
}
