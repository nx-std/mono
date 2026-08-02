//! Raw result codes for the Service Framework error family.
//!
//! Every error this crate declares eventually reaches a C caller as a bare
//! `u32`: through [`crate::ffi`], or through one of the `nx-rt-*` runtime
//! crates that wrap an `nx-service-*` client. Before this module existed each
//! of those boundaries carried its own free function per error type
//! (`parse_resp_error_to_rc`, `dispatch_error_to_rc`, ...) and its own copy of
//! the fallback constant, so one mapping was written once per consumer and
//! drifted between them.
//!
//! [`ToResultCode`] moves the mapping next to the error type that owns it: an
//! error declares how it renders exactly once, and every boundary is a
//! `.to_rc()` call.
//!
//! # Scope of the family
//!
//! The family is `nx-sf` plus the `nx-service-*` IPC clients built on it. Those
//! crates expose a pure Rust API and declare no `ffi` feature of their own
//! (`rust-ffi` §6), so this trait is **neither gated nor sealed**: a service
//! client implements it for its own command errors, and the runtime crate that
//! wraps that client calls `.to_rc()` at the boundary.
//!
//! # Why not `nx-svc`'s trait
//!
//! [`nx_svc::error::ToResultCode`] answers "which kernel result code describes
//! this failure", and it is sealed to the kernel's own errors. Most failures
//! here are not kernel failures at all: a response whose CMIF magic is wrong, a
//! request that will not fit the IPC buffer, a server that replied without the
//! move handle it promised. The kernel never saw them and assigned them no
//! code, so they collapse to [`GENERIC_ERROR`] - a fallback this family owns
//! and the kernel family has no use for.
//!
//! # Using both traits at once
//!
//! A mapping that forwards a code the kernel produced needs both traits in
//! scope. Import the foreign one as `_` and let the receiver select: no type
//! implements both, so `err.to_rc()` resolves unambiguously whichever family
//! `err` belongs to.

/// The raw result-code type, re-exported from `nx-svc`.
///
/// [`ToResultCode`] returns one, so a crate implementing the trait needs the
/// name; re-exporting it here means an `nx-service-*` client does not take a
/// dependency on `nx-svc` for a type alias alone.
pub use nx_svc::error::ResultCode;

/// Result code reported for a failure that carries no service result code of
/// its own.
///
/// A request that does not fit the IPC buffer, a truncated reply, or a missing
/// handle are all local protocol failures: no server ever assigned them a code,
/// so there is nothing to forward.
///
/// Most of them are conditions libnx does not detect at all - `hipcParseResponse`
/// cannot fail, and `cmifParseResponse` checks only the header magic - so there
/// is no libnx value to match. `LibnxError_ShouldNotHappen` is the code libnx
/// reserves for exactly that: a state its own parsers assume away.
pub const GENERIC_ERROR: ResultCode = libnx_error(LibnxError::ShouldNotHappen);

/// libnx error descriptions, for `MAKERESULT(Module_Libnx, error)`.
///
/// Values are the ordinals of the sequential enum in libnx
/// `include/switch/result.h` (`LibnxError_BadReloc = 1`, ...). Only the
/// descriptions this workspace emits are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LibnxError {
    /// The subsystem was used before its `*_initialize` hook ran.
    NotInitialized = 8,
    /// A caller-supplied argument failed validation at the boundary.
    BadInput = 11,
    /// The service-override table is full.
    TooManyOverrides = 16,
    /// The running system version does not implement the requested command.
    IncompatSysVer = 37,
    /// A CMIF reply did not carry the `SFCO` magic.
    InvalidCmifOutHeader = 47,
    /// A condition libnx's own parsers assume cannot arise.
    ShouldNotHappen = 48,
}

/// Builds a result code in libnx's own module.
///
/// Mirrors libnx's `MAKERESULT(Module_Libnx, description)`: the description
/// occupies bits 9..22 and the module the low 9 bits.
pub const fn libnx_error(err: LibnxError) -> ResultCode {
    const MODULE_LIBNX: u32 = 345;
    (MODULE_LIBNX & 0x1FF) | ((err as u32 & 0x1FFF) << 9)
}

/// Converts a Service Framework error into the raw result code a C caller
/// receives.
///
/// Implemented beside each error type, immediately after the type itself.
pub trait ToResultCode: core::error::Error {
    /// Converts the error into a raw result code.
    fn to_rc(self) -> ResultCode;
}
