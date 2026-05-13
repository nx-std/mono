//! Owned, non-domain IPC session.
//!
//! A [`Session`] holds a kernel IPC handle and the server's pointer-buffer
//! size. Dropping a `Session` sends a CMIF close request and closes the
//! kernel handle; close-time errors are deliberately swallowed (see
//! [`control::close_session`]).
//!
//! Use [`Session::convert_to_domain`] to promote the session to a
//! [`Domain`](super::Domain), which unlocks domain object multiplexing.

use core::mem::ManuallyDrop;

use nx_svc::ipc::Handle as SessionHandle;

use super::{
    control::{self, CloneObjectError, CloneObjectExError, ConvertToDomainError},
    dispatch::Dispatch,
    domain::Domain,
};

/// Owned IPC session — not a domain.
///
/// Dropping closes the session; see the module docs for error-handling
/// semantics.
#[derive(Debug)]
pub struct Session {
    handle: SessionHandle,
    pointer_buffer_size: u16,
}

impl Session {
    /// Wraps an IPC session handle, querying the server's pointer-buffer
    /// size. If the query fails the size defaults to 0; the session remains
    /// usable for non-pointer-buffer requests.
    pub fn new(handle: SessionHandle) -> Self {
        let pointer_buffer_size = control::query_pointer_buffer_size(handle).unwrap_or(0);
        Self {
            handle,
            pointer_buffer_size,
        }
    }

    /// Wraps a raw handle without querying pointer-buffer size.
    ///
    /// Use this when the service is known not to use pointer buffers or
    /// when the size is already known, to skip the extra IPC roundtrip
    /// [`Session::new`] performs.
    #[inline]
    pub fn from_handle(handle: SessionHandle, pointer_buffer_size: u16) -> Self {
        Self {
            handle,
            pointer_buffer_size,
        }
    }

    /// Transfers the kernel handle out of the `Session` without closing it.
    /// Used at the FFI boundary to hand the handle back to C.
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

    /// Clones the session via CMIF control request 2.
    pub fn try_clone(&self) -> Result<Session, CloneObjectError> {
        let new_handle = control::clone_current_object(self.handle)?;
        Ok(Self {
            handle: new_handle,
            pointer_buffer_size: self.pointer_buffer_size,
        })
    }

    /// Clones the session with a session-manager tag via CMIF control
    /// request 4.
    pub fn try_clone_ex(&self, tag: u32) -> Result<Session, CloneObjectExError> {
        let new_handle = control::clone_current_object_ex(self.handle, tag)?;
        Ok(Self {
            handle: new_handle,
            pointer_buffer_size: self.pointer_buffer_size,
        })
    }

    /// Converts the session to a [`Domain`] via CMIF control request 0.
    ///
    /// On failure the original `Session` is returned alongside the error so
    /// the caller can drop it normally instead of leaking the handle.
    pub fn convert_to_domain(self) -> Result<Domain, (Session, ConvertToDomainError)> {
        match control::convert_current_object_to_domain(self.handle) {
            Ok(_object_id) => {
                let this = ManuallyDrop::new(self);
                // SAFETY: We extracted the handle from `this`; `ManuallyDrop`
                // suppresses the original `Session`'s Drop so the handle is
                // not closed.
                Ok(unsafe { Domain::from_handle_unchecked(this.handle, this.pointer_buffer_size) })
            }
            Err(err) => Err((self, err)),
        }
    }

    /// Starts a [`Dispatch`] builder for `request_id`.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> Dispatch<'_> {
        Dispatch::new(self.handle, self.pointer_buffer_size, None, request_id)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        control::close_session(self.handle);
    }
}
