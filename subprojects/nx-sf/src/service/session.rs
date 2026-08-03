//! Owned, non-domain IPC session.
//!
//! A [`Session`] holds a kernel IPC handle and the server's pointer-buffer
//! size. Dropping a `Session` sends a CMIF close request and closes the
//! kernel handle; close-time errors are deliberately swallowed (see
//! [`control::close_session`]).
//!
//! Use [`Session::convert_to_domain`] to promote the session to a
//! [`Domain`](super::Domain), which unlocks domain object multiplexing.

use super::{
    control::{self, CloneObjectError, CloneObjectExError, ConvertToDomainError},
    dispatch::Dispatch,
    domain::Domain,
    handle::{BorrowedSessionHandle, OwnedSessionHandle},
};

/// Owned IPC session — not a domain.
///
/// Dropping closes the session; see the module docs for error-handling
/// semantics.
#[derive(Debug)]
pub struct Session {
    handle: OwnedSessionHandle,
    pointer_buffer_size: u16,
}

impl Session {
    /// Wraps an IPC session handle, querying the server's pointer-buffer
    /// size. If the query fails the size defaults to 0; the session remains
    /// usable for non-pointer-buffer requests.
    pub fn open(handle: OwnedSessionHandle) -> Self {
        let pointer_buffer_size =
            control::query_pointer_buffer_size(handle.as_borrowed()).unwrap_or(0);
        Self {
            handle,
            pointer_buffer_size,
        }
    }

    /// Wraps an owned handle without querying pointer-buffer size.
    ///
    /// Use this when the service is known not to use pointer buffers or
    /// when the size is already known, to skip the extra IPC roundtrip
    /// [`Session::open`] performs.
    #[inline]
    pub fn new(handle: OwnedSessionHandle, pointer_buffer_size: u16) -> Self {
        Self {
            handle,
            pointer_buffer_size,
        }
    }

    /// Transfers the kernel handle out of the `Session` without closing it.
    /// Used at the FFI boundary to hand the handle back to C.
    #[inline]
    pub fn into_handle(self) -> OwnedSessionHandle {
        let this = core::mem::ManuallyDrop::new(self);
        // SAFETY: `this` is never dropped and never read again, so the handle is moved out
        // exactly once.
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

    /// Clones the session via CMIF control request 2.
    pub fn try_clone(&self) -> Result<Session, CloneObjectError> {
        let new_handle = control::clone_current_object(self.handle.as_borrowed())?;
        Ok(Self {
            handle: new_handle,
            pointer_buffer_size: self.pointer_buffer_size,
        })
    }

    /// Clones the session with a session-manager tag via CMIF control
    /// request 4.
    pub fn try_clone_ex(&self, tag: u32) -> Result<Session, CloneObjectExError> {
        let new_handle = control::clone_current_object_ex(self.handle.as_borrowed(), tag)?;
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
        match control::convert_current_object_to_domain(self.handle.as_borrowed()) {
            Ok(_object_id) => {
                let pointer_buffer_size = self.pointer_buffer_size;
                // SAFETY: The control request above returned success, which is the server
                // confirming it converted this session to a domain; `pointer_buffer_size` is
                // carried over from the `Session`, where the server itself reported it.
                Ok(Domain::new_unchecked(
                    self.into_handle(),
                    pointer_buffer_size,
                ))
            }
            Err(err) => Err((self, err)),
        }
    }

    /// Starts a [`Dispatch`] builder for `request_id`.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> Dispatch<'_> {
        Dispatch::new(
            self.handle.as_borrowed(),
            self.pointer_buffer_size,
            None,
            request_id,
        )
    }
}
