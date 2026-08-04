//! The owner slot every lock in this crate uses to name the thread holding it.
//!
//! Each primitive stores the owning thread's kernel handle so it can tell "locked by me" from
//! "locked by someone else": [`Mutex`](crate::Mutex) packs it into its atomic word alongside the
//! waiters bit, [`ReentrantMutex`](crate::ReentrantMutex) keeps it beside a recursion counter, and
//! [`RwLock`](crate::RwLock) keeps it for the writer. All three compare it against the calling
//! thread, and all three use [`ThreadTag::NONE`] to mean unowned.
//!
//! The handle is a bare `u32` on the wire, which is also what a count, a waiter total and a raw
//! `ResultCode` are. `ThreadTag` keeps the one that identifies a thread apart from the rest, and
//! gives the "who am I" read a single home rather than a copy in each primitive.

use nx_svc::raw::{
    Handle,
    INVALID_HANDLE,
};

/// The kernel handle of the thread owning a synchronization primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ThreadTag(Handle);

impl ThreadTag {
    /// The tag stored while no thread owns the primitive.
    ///
    /// This is the kernel's invalid handle, which no live thread is ever assigned.
    pub const NONE: Self = Self(INVALID_HANDLE);

    /// Returns the calling thread's tag.
    ///
    /// The handle is read from the thread's own TLS footer, so this is a load from a
    /// register-relative address rather than a kernel call.
    #[inline(always)]
    pub fn current() -> Self {
        Self(nx_sys_thread_tls::get_current_thread_handle().to_raw())
    }

    /// Wraps a raw kernel handle without checking that it names a thread.
    ///
    /// The caller must ensure `raw` is a thread handle or [`INVALID_HANDLE`]. A handle naming
    /// some other kernel object compares unequal to every thread, so a lock tagged with one can
    /// never be unlocked by its owner.
    #[inline]
    pub const fn from_raw_unchecked(raw: Handle) -> Self {
        Self(raw)
    }

    /// Returns the raw kernel handle.
    #[inline]
    pub const fn to_raw(self) -> Handle {
        self.0
    }

    /// Returns `true` if no thread owns the primitive.
    #[inline]
    pub const fn is_none(self) -> bool {
        self.0 == INVALID_HANDLE
    }
}
