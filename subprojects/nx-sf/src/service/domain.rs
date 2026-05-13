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

use core::mem::ManuallyDrop;

use nx_svc::ipc::Handle as SessionHandle;

use super::{
    control::{self, CloneObjectError, CopyFromDomainError},
    dispatch::Dispatch,
    session::Session,
};
use crate::cmif::ObjectId;

/// Owned domain root: a session converted via CMIF control request 0.
#[derive(Debug)]
pub struct Domain {
    handle: SessionHandle,
    pointer_buffer_size: u16,
}

impl Domain {
    /// Wraps a handle that has already been converted to a domain.
    ///
    /// # Safety
    ///
    /// The caller must own `handle`, the server must have converted it to a
    /// domain, and `pointer_buffer_size` must reflect the server's value.
    #[inline]
    pub unsafe fn from_handle_unchecked(handle: SessionHandle, pointer_buffer_size: u16) -> Self {
        Self {
            handle,
            pointer_buffer_size,
        }
    }

    /// Transfers the kernel handle out of the `Domain` without closing it.
    /// Used at the FFI boundary.
    #[inline]
    pub fn into_handle(self) -> SessionHandle {
        let this = ManuallyDrop::new(self);
        this.handle
    }

    /// Returns the underlying session handle without transferring ownership.
    #[inline]
    pub fn handle(&self) -> SessionHandle {
        self.handle
    }

    /// Returns the server's pointer-buffer size.
    #[inline]
    pub fn pointer_buffer_size(&self) -> u16 {
        self.pointer_buffer_size
    }

    /// Borrows the domain to address a named object inside it. No IPC is
    /// issued — typical use is wrapping an [`ObjectId`] returned in a prior
    /// dispatch response.
    #[inline]
    pub fn open_object(&self, object_id: ObjectId) -> DomainObject<'_> {
        DomainObject {
            domain: self,
            object_id,
        }
    }

    /// Like [`open_object`](Self::open_object) but takes the raw object id
    /// returned by the server in a dispatch response. Returns `None` if the
    /// raw value is the sentinel zero (no object).
    #[inline]
    pub fn open_object_raw(&self, raw_object_id: u32) -> Option<DomainObject<'_>> {
        ObjectId::new(raw_object_id).map(|object_id| self.open_object(object_id))
    }

    /// Extracts a domain object into a standalone non-domain [`Session`] via
    /// CMIF control request 1.
    pub fn copy_object_to_session(
        &self,
        object_id: ObjectId,
    ) -> Result<Session, CopyFromDomainError> {
        let new_handle = control::copy_from_current_domain(self.handle, object_id)?;
        Ok(Session::from_handle(new_handle, self.pointer_buffer_size))
    }

    /// Clones the underlying session via CMIF control request 2. The clone
    /// is a fresh non-domain [`Session`], matching libnx semantics.
    pub fn try_clone(&self) -> Result<Session, CloneObjectError> {
        let new_handle = control::clone_current_object(self.handle)?;
        Ok(Session::from_handle(new_handle, self.pointer_buffer_size))
    }

    /// Starts a [`Dispatch`] builder addressing the domain root itself.
    /// Domain-object requests should go through
    /// [`DomainObject::dispatch`] instead.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> Dispatch<'_> {
        Dispatch::new(self.handle, self.pointer_buffer_size, None, request_id)
    }
}

impl Drop for Domain {
    fn drop(&mut self) {
        control::close_session(self.handle);
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

    /// Starts a [`Dispatch`] builder addressing this domain object.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> Dispatch<'_> {
        Dispatch::new(
            self.domain.handle,
            self.domain.pointer_buffer_size,
            Some(self.object_id),
            request_id,
        )
    }
}

impl Drop for DomainObject<'_> {
    fn drop(&mut self) {
        control::close_object(self.domain.handle, self.object_id);
    }
}
