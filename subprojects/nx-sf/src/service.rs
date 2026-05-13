//! Idiomatic Rust API for Horizon OS IPC services.
//!
//! Each operational mode is its own type:
//!
//! - [`Session`] — owns a non-domain session handle. Dropping closes it.
//! - [`Domain`] — owns a session that has been converted to a domain root.
//!   Dropping closes the session and the server cascades object-close on its
//!   side.
//! - [`DomainObject`] — borrows a [`Domain`] and names a single object inside
//!   it. Dropping sends a per-object close request without closing the
//!   underlying handle. The borrow makes use-after-close a compile error.
//! - [`OverrideService`] — a non-owning view used by FFI / libnx-takeover
//!   paths; its drop is a no-op.
//!
//! Stateless CMIF control-request helpers live in [`control`]. The dispatch
//! builder lives in [`dispatch`] and is reached via the typed wrappers'
//! `dispatch(...)` methods.

pub(crate) mod control;
mod dispatch;
mod domain;
mod override_service;
mod session;

pub use self::{
    control::{
        CloneObjectError, CloneObjectExError, ConvertToDomainError, CopyFromDomainError,
        QueryPointerBufferSizeError, clone_current_object, clone_current_object_ex,
        convert_current_object_to_domain, copy_from_current_domain, query_pointer_buffer_size,
    },
    dispatch::{
        Buffer, BufferAttr, Dispatch, DispatchError, DispatchResult, DomainDispatch,
        DomainDispatchResult, MAX_BUFFERS, MAX_IN_HANDLES, MAX_IN_OBJECTS, MAX_OUT_OBJECTS,
        OutHandleAttr,
    },
    domain::{Domain, DomainObject},
    override_service::OverrideService,
    session::Session,
};
