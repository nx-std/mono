//! Common FFI helpers shared across the `nx-rt-*` runtime crates.
//!
//! `nx-rt-core` is the single authoritative home for the kind-agnostic FFI
//! helpers: the generic error code, the `SyncUnsafeCell` static-storage
//! wrapper, the CMIF/TIPC parse-error → result-code converters, and the
//! `nx-sf` dispatch/domain error converters. The per-output-kind entry crates
//! (`nx-rt-nro`, …) re-export these from here rather than re-defining them, so
//! the nx-sf parse-error → libnx result-code mapping lives in exactly one
//! place.

use core::cell::UnsafeCell;

use nx_sf::{cmif, tipc};

/// Generic error code for FFI when no specific result code is available.
pub const GENERIC_ERROR: u32 = 0xFFFF;

/// Wrapper to make UnsafeCell Sync for static storage.
#[repr(transparent)]
pub struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: this `unsafe impl` asserts `Sync` for *every* `T`, which is sound
// only under a usage contract the type itself does not enforce. Every
// instantiation across the `nx-rt-*` runtime is a `static` cache touched
// solely during single-threaded runtime init/exit: each cell is written
// exactly once by its owner's `*_initialize` hook (or zeroed by the matching
// `*_exit`) and read only after that publication, with no concurrent access.
// libnx guarantees a given service's `*_initialize`/`*_exit` runs once,
// single-threaded, before/after any FFI access to its cache — so no two
// accesses race. Callers placing a `SyncUnsafeCell` in `static` storage must
// uphold this no-races contract; it is an internal runtime utility, not a
// general-purpose cell.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// Converts a CMIF [`cmif::ParseError`] to a raw result code.
pub fn parse_resp_error_to_rc(err: cmif::ParseError) -> u32 {
    match err {
        cmif::ParseError::ServiceError(code) => code,
        cmif::ParseError::InvalidMagic
        | cmif::ParseError::Hipc(_)
        | cmif::ParseError::TruncatedOutHeader
        | cmif::ParseError::TruncatedDomainHeader
        | cmif::ParseError::TruncatedPayload
        | cmif::ParseError::TruncatedDomainObjects => GENERIC_ERROR,
    }
}

/// Converts a TIPC [`tipc::ParseResponseError`] to a raw result code.
pub fn parse_tipc_resp_error_to_rc(err: tipc::ParseResponseError) -> u32 {
    match err {
        tipc::ParseResponseError::ServiceError(code) => code,
        tipc::ParseResponseError::Hipc(_)
        | tipc::ParseResponseError::TruncatedResult
        | tipc::ParseResponseError::TruncatedPayload => GENERIC_ERROR,
    }
}

/// Converts a CMIF [`cmif::ParseError`] to a raw result code.
pub fn parse_resp_bytes_error_to_rc(err: cmif::ParseError) -> u32 {
    match err {
        cmif::ParseError::ServiceError(code) => code,
        cmif::ParseError::InvalidMagic
        | cmif::ParseError::Hipc(_)
        | cmif::ParseError::TruncatedOutHeader
        | cmif::ParseError::TruncatedDomainHeader
        | cmif::ParseError::TruncatedPayload
        | cmif::ParseError::TruncatedDomainObjects => GENERIC_ERROR,
    }
}

/// Converts an `nx-sf` [`DispatchError`](nx_sf::service::DispatchError) to a
/// raw result code.
pub fn dispatch_error_to_rc(err: nx_sf::service::DispatchError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        nx_sf::service::DispatchError::Layout(_) => GENERIC_ERROR,
        nx_sf::service::DispatchError::SendRequest(e) => e.to_rc(),
        nx_sf::service::DispatchError::ParseResponse(e) => parse_resp_bytes_error_to_rc(e),
    }
}

/// Converts an `nx-sf` [`ConvertToDomainError`](nx_sf::service::ConvertToDomainError)
/// to a raw result code.
pub fn convert_to_domain_error_to_rc(err: nx_sf::service::ConvertToDomainError) -> u32 {
    match err {
        nx_sf::service::ConvertToDomainError::SendRequest(e) => send_error_to_rc(e),
        nx_sf::service::ConvertToDomainError::ParseResponse(e) => parse_resp_error_to_rc(e),
    }
}

/// Converts an `nx-sf` request [`SendError`](cmif::SendError) to a raw
/// result code. Covers both CMIF and TIPC requests - the two aliases name
/// the same HIPC-level type.
pub fn send_error_to_rc(err: cmif::SendError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        cmif::SendError::Layout(_) => GENERIC_ERROR,
        cmif::SendError::SendRequest(e) => e.to_rc(),
    }
}
