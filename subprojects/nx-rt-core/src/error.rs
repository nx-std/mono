//! Raw result codes for the `nx-rt-*` runtime error family.
//!
//! The runtime crates sit between the `nx-service-*` clients and the C
//! callers that libnx's headers describe. Every failure they surface leaves as
//! a bare `u32`, so each runtime error declares its own mapping through
//! [`ToResultCode`] rather than having each adapter decode it by hand.
//!
//! # Scope of the family
//!
//! `nx-rt-core` plus the per-output-kind entry crates (`nx-rt-nro`,
//! `nx-rt-nso`, `nx-rt-kip`, `nx-rt-module`), which implement this trait for
//! the runtime errors they own. It is therefore unsealed, like `nx-sf`'s: the
//! entry crates are the rest of this family, not foreign implementors, and
//! they share the [`LibnxError`] vocabulary declared here.
//!
//! # What this family adds
//!
//! A runtime error usually wraps a service-client error, and those already map
//! themselves through [`nx_sf::error::ToResultCode`] - so most impls here are
//! one delegating arm per variant. What the runtime owns, and the layers below
//! it cannot express, is *when* to reach for the **libnx result vocabulary**:
//! codes in `Module_Libnx` that describe a policy this layer enforces rather
//! than a failure any server or the kernel reported. [`libnx_error`] builds
//! those, re-exported here from `nx-sf`.
//!
//! # Using several traits at once
//!
//! An impl that forwards a code produced further down needs that family's trait
//! in scope too. Import the foreign ones as `_` and let the receiver select: no
//! type implements two of them, so `err.to_rc()` resolves unambiguously.

/// The result-code type and libnx vocabulary, re-exported from the Service
/// Framework family.
///
/// The runtime sits directly on top of `nx-sf` and reports the same codes, so
/// the two must not drift. `nx-sf` is the lowest crate in the libnx-replacement
/// stack that needs them, which is why they are declared there and not here.
pub use nx_sf::error::{
    GENERIC_ERROR,
    LibnxError,
    ResultCode,
    libnx_error,
};

/// Converts an `nx-rt-*` runtime error into the raw result code a C caller
/// receives.
///
/// Implemented beside each runtime error type, immediately after the type
/// itself.
pub trait ToResultCode: core::error::Error {
    /// Converts the error into a raw result code.
    fn to_rc(self) -> ResultCode;
}
