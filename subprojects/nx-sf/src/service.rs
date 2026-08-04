//! Idiomatic Rust API for Horizon OS IPC services.
//!
//! Each operational mode is its own type, and each resource a type may close
//! comes in an owning form and a borrowed one:
//!
//! - [`Session`] - owns a non-domain session handle. Dropping closes it.
//! - [`Domain`] / [`DomainRef`] - a session converted to a domain root.
//!   Dropping the owner closes the session and the server cascades object-close
//!   on its side; the ref closes nothing.
//! - [`DomainObject`] / [`DomainObjectRef`] - a single object inside a domain.
//!   Dropping the owner sends a per-object close request without touching the
//!   kernel handle; the ref closes nothing. Both carry the domain's lifetime,
//!   which makes use-after-close a compile error.
//! - [`OverrideService`] - a non-owning view used by FFI / libnx-takeover
//!   paths; its drop is a no-op.
//!
//! Take the borrowed form in anything that merely uses a resource. The owning
//! forms are neither `Copy` nor `Clone`, so a second closer needs a move and the
//! move checker rejects it.
//!
//! Stateless CMIF control-request helpers live in [`control`]. The dispatch
//! builder lives in [`dispatch`] and is reached via the typed wrappers'
//! `dispatch(...)` methods.

pub(crate) mod control;
mod dispatch;
mod domain;
pub(crate) mod handle;
mod override_service;
mod session;

pub use self::{
    control::{
        CloneObjectError,
        CloneObjectExError,
        ConvertToDomainError,
        CopyFromDomainError,
        QueryPointerBufferSizeError,
        clone_current_object,
        clone_current_object_ex,
        convert_current_object_to_domain,
        copy_from_current_domain,
        query_pointer_buffer_size,
    },
    dispatch::{
        BufferAttr,
        Dispatch,
        DispatchError,
        DispatchResult,
        DomainDispatch,
        DomainDispatchResult,
        MAX_BUFFERS,
        MAX_IN_HANDLES,
        MAX_IN_OBJECTS,
        MAX_OUT_OBJECTS,
        OutHandleAttr,
    },
    domain::{
        Domain,
        DomainObject,
        DomainObjectRef,
        DomainRef,
    },
    handle::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
    override_service::OverrideService,
    session::Session,
};
