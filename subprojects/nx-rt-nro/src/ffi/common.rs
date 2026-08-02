//! Common utilities for FFI modules.
//!
//! The kind-agnostic helpers — `GENERIC_ERROR`, `SyncUnsafeCell`, and the
//! libnx result vocabulary — are re-exported from `nx-rt-core`, their single
//! authoritative home. Each re-export is gated to the service features that
//! consume it, mirroring the per-service gating of the FFI modules themselves.
//!
//! The per-error converters that used to live here are gone: every error maps
//! itself through its own family's `ToResultCode`, so an adapter calls
//! `.to_rc()` rather than a function that took another crate's error apart.

#[cfg(feature = "service-vi")]
pub use nx_rt_core::error::{LibnxError, libnx_error};
pub use nx_rt_core::ffi::common::GENERIC_ERROR;
#[cfg(any(
    feature = "service-apm",
    feature = "service-nv",
    feature = "service-set",
    feature = "service-vi",
))]
pub use nx_rt_core::ffi::common::SyncUnsafeCell;
