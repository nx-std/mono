//! Raw result codes for the `nx-sys-mem` error types.
//!
//! This crate has no C-FFI surface of its own, but its failures reach one: an
//! `nx-service-*` client that maps a transfer- or shared-memory region wraps
//! these errors and must report a code for them. [`ToResultCode`] lets it
//! delegate rather than take the error apart.
//!
//! The trait is therefore **not** gated on a feature - the crates that call it
//! declare no `ffi` feature of their own - but it is sealed, since only this
//! crate's errors describe this crate's failures.
//!
//! Most variants wrap a kernel error and forward whatever the SVC returned.
//! What this crate adds are the failures the kernel never saw: an address-space
//! reservation that found no room, and the page-alignment and size checks made
//! before any SVC is issued.

use nx_svc::error::ResultCode;

/// Converts an `nx-sys-mem` error into the raw result code a C caller receives.
///
/// Implemented beside each error type this crate declares.
pub trait ToResultCode: core::error::Error + _sealed::Sealed {
    /// Converts the error into a raw result code.
    fn to_rc(self) -> ResultCode;
}

pub(crate) mod _sealed {
    /// Restricts [`ToResultCode`](super::ToResultCode) to this crate's error
    /// types. Implemented immediately after every `ToResultCode` impl.
    pub trait Sealed {}
}
