//! Non-owning view over an IPC session managed elsewhere.
//!
//! `OverrideService` is used by code paths that take over a libnx-owned
//! session (for example, a Rust replacement for a libnx service init
//! routine). It carries the same data as a [`Session`](super::Session) but
//! does **not** close the handle on drop — ownership remains with whoever
//! provided it.

use nx_svc::ipc::Handle as SessionHandle;

use super::{
    dispatch::Dispatch,
    handle::BorrowedSessionHandle,
};

/// Non-owning service view; drop does not close the handle.
#[derive(Debug, Clone, Copy)]
pub struct OverrideService {
    handle: BorrowedSessionHandle<'static>,
    pointer_buffer_size: u16,
}

impl OverrideService {
    /// Wraps a handle managed by an external owner, without checking that the owner outlives
    /// this view.
    ///
    /// The `'static` borrow this mints is an assertion, not a fact the compiler checked, which
    /// is what the name admits. The caller must ensure the external owner keeps the session
    /// open for as long as this view is used; nothing here closes it, and a session closed
    /// underneath leaves the view naming a handle number the kernel may have reused, so
    /// requests sent through it reach whatever now holds that number.
    #[inline]
    pub const fn new_unchecked(handle: SessionHandle, pointer_buffer_size: u16) -> Self {
        Self {
            // SAFETY: The external owner named in this constructor's precondition is what
            // keeps the session alive; no borrow the compiler could check exists here,
            // because the owner is on the C side of the boundary.
            handle: BorrowedSessionHandle::from_handle_unchecked(handle),
            pointer_buffer_size,
        }
    }

    /// Returns the underlying session handle.
    #[inline]
    pub fn handle(&self) -> BorrowedSessionHandle<'static> {
        self.handle
    }

    /// Returns the server's pointer-buffer size.
    #[inline]
    pub fn pointer_buffer_size(&self) -> u16 {
        self.pointer_buffer_size
    }

    /// Starts a [`Dispatch`] builder for `request_id`.
    #[inline]
    pub fn dispatch<'p>(&self, request_id: u32) -> Dispatch<'_, 'p> {
        Dispatch::new(self.handle, self.pointer_buffer_size, None, request_id)
    }
}
