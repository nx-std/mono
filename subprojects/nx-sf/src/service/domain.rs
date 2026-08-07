//! Domain root and per-object handles.
//!
//! A [`Domain`] is an IPC session that has been promoted via CMIF control
//! request 0 and multiplexes multiple server-side objects over a single
//! kernel handle.
//!
//! Both levels of the domain - the session and the objects inside it - come in
//! an owning form and a borrowed one, and the split is the same at both:
//!
//! | Level  | Owns, closes on drop | Borrows, no destructor |
//! |--------|----------------------|------------------------|
//! | Domain | [`Domain`]           | [`DomainRef`]          |
//! | Object | [`DomainObject`]     | [`DomainObjectRef`]    |
//!
//! The owning forms are neither `Copy` nor `Clone`, so a second closer needs a
//! move and the move checker rejects it. The borrowed forms are `Copy`, have no
//! destructor, and carry the owner's lifetime, so they can neither close nor
//! outlive what they name.
//!
//! Everything that merely *addresses* an object takes [`DomainObjectRef`]. A
//! [`DomainObject`] is minted only where a close is genuinely owed: the server
//! emits an object id in a reply, and `DomainDispatch::send` turns each one into
//! exactly one owner.

use core::{
    marker::PhantomData,
    mem::ManuallyDrop,
};

use super::{
    control::{
        self,
        CloneObjectError,
        CopyFromDomainError,
    },
    dispatch::DomainDispatch,
    handle::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
    session::Session,
};
use crate::cmif::ObjectId;

/// Owned domain root: a session converted via CMIF control request 0.
///
/// Dropping closes the underlying session; the server cascades object close on
/// its side, so the objects opened against it need no individual close first.
#[derive(Debug)]
pub struct Domain {
    handle: OwnedSessionHandle,
    pointer_buffer_size: u16,
    object_id: ObjectId,
}

impl Domain {
    /// Wraps a handle that has already been converted to a domain.
    ///
    /// Taking an [`OwnedSessionHandle`] is what makes this a safe function: ownership is
    /// established once, where the handle is adopted, so this cannot manufacture a second
    /// closer for a session something else already owns. The caller must still ensure the
    /// server has converted the session to a domain and that `pointer_buffer_size` reflects
    /// the server's value; a wrong size mis-sizes pointer buffers rather than faulting.
    ///
    /// `object_id` is the id the server assigned the original interface when it
    /// converted the session. Requests aimed at the domain itself carry it, so a
    /// wrong value reaches the wrong object rather than faulting.
    #[inline]
    pub fn new_unchecked(
        handle: OwnedSessionHandle,
        pointer_buffer_size: u16,
        object_id: ObjectId,
    ) -> Self {
        Self {
            handle,
            pointer_buffer_size,
            object_id,
        }
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
            object_id: self.object_id,
            owner: PhantomData,
        }
    }

    /// Returns the id the server assigned the interface this domain was
    /// converted from.
    #[inline]
    pub fn object_id(&self) -> ObjectId {
        self.object_id
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

    /// Extracts a domain object into a standalone non-domain [`Session`] via
    /// CMIF control request 1.
    ///
    /// Takes the owning [`DomainObject`] rather than a [`DomainObjectRef`] or a
    /// raw [`ObjectId`], so the caller must hold the object's sole closer -
    /// preventing an id from a dropped or borrowed object from being laundered
    /// into a new [`Session`].
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
    /// [`DomainObjectRef::dispatch`] instead.
    #[inline]
    pub fn dispatch<'p>(&self, request_id: u32) -> DomainDispatch<'_, 'p> {
        self.as_borrowed().dispatch(request_id)
    }
}

/// Borrowed view onto a [`Domain`] that does not own its handle.
///
/// Dispatches like a `Domain` but has no destructor, so it cannot close the
/// session; the lifetime keeps it from outliving the owner.
#[derive(Debug, Clone, Copy)]
pub struct DomainRef<'d> {
    handle: BorrowedSessionHandle<'d>,
    pointer_buffer_size: u16,
    object_id: ObjectId,
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

    /// Addresses a named object inside the domain, taking on the obligation to
    /// close it. No IPC is issued.
    ///
    /// Crate-internal: the only legitimate source of a fresh owner is
    /// `DomainDispatch::send`, which calls this once per server-emitted
    /// [`ObjectId`].
    #[inline]
    pub(crate) fn open_object(&self, object_id: ObjectId) -> DomainObject<'d> {
        DomainObject {
            domain: *self,
            object_id,
        }
    }

    /// Starts a [`DomainDispatch`] builder aimed at the domain itself.
    ///
    /// The request carries this domain's own object id. Once a session is
    /// converted, the server expects a domain header on *every* request it
    /// receives, including the ones aimed at the original interface - sending
    /// one without is how a converted session starts answering errors.
    #[inline]
    pub fn dispatch<'p>(&self, request_id: u32) -> DomainDispatch<'d, 'p> {
        DomainDispatch::new(*self, Some(self.object_id), request_id)
    }

    /// Returns the id the server assigned the interface this domain was
    /// converted from.
    #[inline]
    pub fn object_id(&self) -> ObjectId {
        self.object_id
    }
}

/// Owned view onto a single object inside a [`Domain`]: dropping it sends a
/// per-object close request on the parent session.
///
/// Neither `Copy` nor `Clone`, so it is the sole closer for its id. The `'d`
/// lifetime ties the object to its parent domain, making use-after-close a
/// compile error.
///
/// Take [`DomainObjectRef`] in anything that merely addresses the object; this
/// type is for the one holder that owes the close.
#[derive(Debug)]
pub struct DomainObject<'d> {
    domain: DomainRef<'d>,
    object_id: ObjectId,
}

impl<'d> DomainObject<'d> {
    /// Wraps a raw object id, taking on the obligation to close it. Returns
    /// `None` if the raw value is the sentinel zero (no object).
    ///
    /// Prefer [`DomainObjectRef::from_raw_unchecked`], which closes nothing and
    /// so carries no such obligation. This form exists for the `Drop` impl of a
    /// wrapper that stored an id and now owes the server a close.
    ///
    /// The caller must ensure `raw_object_id` names a live server-side object
    /// inside `domain` that no other live `DomainObject` already addresses. A
    /// second owner sends its close against an id the server may have reused, so
    /// an unrelated object is torn down; that is a resource error rather than
    /// undefined behaviour, which is why this is a safe function.
    #[inline]
    pub fn from_raw_unchecked(domain: DomainRef<'d>, raw_object_id: u32) -> Option<Self> {
        ObjectId::new(raw_object_id).map(|object_id| domain.open_object(object_id))
    }

    /// Returns the object id this view addresses.
    #[inline]
    pub fn object_id(&self) -> ObjectId {
        self.object_id
    }

    /// Returns the parent domain.
    #[inline]
    pub fn domain(&self) -> DomainRef<'d> {
        self.domain
    }

    /// Borrows the object as a non-closing view.
    #[inline]
    pub fn as_borrowed(&self) -> DomainObjectRef<'_> {
        DomainObjectRef {
            domain: self.domain,
            object_id: self.object_id,
            owner: PhantomData,
        }
    }

    /// Gives up the close obligation, returning the raw object id.
    ///
    /// The caller becomes responsible for closing the object exactly once, which
    /// at this level means minting one [`DomainObject`] for the id and dropping
    /// it.
    #[inline]
    pub fn into_raw_object_id(self) -> u32 {
        // The close is being handed on rather than sent, so the destructor that
        // would send it is suppressed.
        let this = ManuallyDrop::new(self);
        this.object_id.to_raw()
    }

    /// Starts a [`DomainDispatch`] builder addressing this object.
    #[inline]
    pub fn dispatch<'p>(&self, request_id: u32) -> DomainDispatch<'_, 'p> {
        self.as_borrowed().dispatch(request_id)
    }
}

impl Drop for DomainObject<'_> {
    fn drop(&mut self) {
        control::close_object(self.domain.handle(), self.object_id);
    }
}

/// Borrowed view onto a single object inside a [`Domain`].
///
/// `Copy` and destructor-free: closing is the owner's job, and the lifetime
/// keeps the view from outliving either the object or the domain it is
/// addressed through.
///
/// This is the type every function that merely dispatches against an object
/// should take.
#[derive(Debug, Clone, Copy)]
pub struct DomainObjectRef<'d> {
    domain: DomainRef<'d>,
    object_id: ObjectId,
    owner: PhantomData<&'d DomainObject<'d>>,
}

impl<'d> DomainObjectRef<'d> {
    /// Addresses a server-side object by its raw id, without checking that the
    /// id names one. Returns `None` if the raw value is the sentinel zero (no
    /// object).
    ///
    /// The caller must ensure `raw_object_id` was issued by the server for an
    /// object inside `domain` that is still open. Nothing here can check that,
    /// since only the server knows which ids are live. A stale or fabricated id
    /// is answered with an error by the request it reaches rather than faulting,
    /// and this view closes nothing, so no close can land on an object the id
    /// was reused for - which is why this is a safe function.
    #[inline]
    pub fn from_raw_unchecked(domain: DomainRef<'d>, raw_object_id: u32) -> Option<Self> {
        ObjectId::new(raw_object_id).map(|object_id| Self {
            domain,
            object_id,
            owner: PhantomData,
        })
    }

    /// Returns the object id this view addresses.
    #[inline]
    pub fn object_id(&self) -> ObjectId {
        self.object_id
    }

    /// Returns the parent domain.
    #[inline]
    pub fn domain(&self) -> DomainRef<'d> {
        self.domain
    }

    /// Starts a [`DomainDispatch`] builder addressing this object.
    #[inline]
    pub fn dispatch<'p>(&self, request_id: u32) -> DomainDispatch<'d, 'p> {
        DomainDispatch::new(self.domain, Some(self.object_id), request_id)
    }
}
