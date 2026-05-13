//! Non-owning view over an IPC session managed elsewhere.
//!
//! `OverrideService` is used by code paths that take over a libnx-owned
//! session (for example, a Rust replacement for a libnx service init
//! routine). It carries the same data as a [`Session`](super::Session) but
//! does **not** close the handle on drop — ownership remains with whoever
//! provided it.

use nx_svc::ipc::Handle as SessionHandle;

use super::dispatch::Dispatch;

/// Non-owning service view; drop does not close the handle.
#[derive(Debug, Clone, Copy)]
pub struct OverrideService {
    handle: SessionHandle,
    pointer_buffer_size: u16,
}

impl OverrideService {
    /// Wraps a handle managed by an external owner.
    #[inline]
    pub const fn new(handle: SessionHandle, pointer_buffer_size: u16) -> Self {
        Self {
            handle,
            pointer_buffer_size,
        }
    }

    /// Returns the underlying session handle.
    #[inline]
    pub fn handle(&self) -> SessionHandle {
        self.handle
    }

    /// Returns the server's pointer-buffer size.
    #[inline]
    pub fn pointer_buffer_size(&self) -> u16 {
        self.pointer_buffer_size
    }

    /// Starts a [`Dispatch`] builder for `request_id`.
    #[inline]
    pub fn dispatch(&self, request_id: u32) -> Dispatch<'_> {
        Dispatch::new(self.handle, self.pointer_buffer_size, None, request_id)
    }
}
