//! Raw result codes for the `nx-sys-thread` error family.
//!
//! Every error this crate declares reaches a C caller as a bare `u32`, through
//! the libnx and libsysbase adapters in [`crate::ffi`]. [`ToResultCode`]
//! puts that mapping next to the error type that owns it, so an error states
//! how it renders exactly once rather than once per adapter that returns it.
//!
//! # Why the trait sits at the crate root
//!
//! The trait is only ever used by the FFI adapters, so it is gated on the `ffi`
//! feature - but it does not live *under* [`crate::ffi`]. The adapters already
//! reach into the core modules for their types, so a trait declared inside
//! `ffi` would force [`thread`](crate::thread) and [`tsd`](crate::tsd) to reach
//! back up into `ffi` to implement it, closing a sibling cycle
//! (`rust-mods-graph` §2). A third module both sides import from - this one -
//! is the fix that rule names.
//!
//! # Why not `nx-svc`'s trait
//!
//! These impls previously sat on [`nx_svc::error::ToResultCode`]. That trait
//! answers "which kernel result code describes this failure", and its
//! implementors are the kernel's own errors. A thread failure is not a kernel
//! failure: `CreateError::StackTooSmall` and `TsdAllocError`
//! are this crate's own preconditions, rejected before any SVC is issued. They
//! borrow a [`KernelError`](nx_svc::error::KernelError) code because it is the
//! closest thing a C caller can decode - a choice this crate makes, not a fact
//! about the kernel.
//!
//! Keeping the two traits apart lets `nx-svc` seal its own, so nothing outside
//! that crate can put a non-kernel code where a caller expects a kernel one.
//!
//! # Using both traits at once
//!
//! A mapping that forwards a code the kernel already produced needs both traits
//! in scope. Import each as `_` and let the receiver select: no type implements
//! both, so `err.to_rc()` resolves unambiguously whichever family `err` belongs
//! to.

use nx_svc::error::ResultCode;

/// Converts an `nx-sys-thread` error into the raw result code a C caller
/// receives.
///
/// Implemented beside each error type this crate declares, immediately after
/// the type itself.
///
/// The trait is sealed, for the same reason `nx-svc` seals its own: an error
/// declared elsewhere is not a thread failure, and the adapters in
/// [`crate::ffi`] return these codes on this crate's behalf. A foreign error
/// declares its own family's trait instead.
pub trait ToResultCode: core::error::Error + _sealed::Sealed {
    /// Converts the error into a raw result code.
    fn to_rc(self) -> ResultCode;
}

pub(crate) mod _sealed {
    /// Restricts [`ToResultCode`](super::ToResultCode) to this crate's error
    /// types. Implemented immediately after every `ToResultCode` impl.
    pub trait Sealed {}
}
