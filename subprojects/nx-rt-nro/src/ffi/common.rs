//! Common utilities for FFI modules.
//!
//! The kind-agnostic helpers — `GENERIC_ERROR`, `SyncUnsafeCell`, and the
//! CMIF/TIPC parse-error → result-code converters — are re-exported from
//! [`nx_rt_core::ffi::common`], their single authoritative home. Each
//! re-export is gated to the service features that consume it, mirroring the
//! per-service gating of the FFI modules themselves. Only the NRO-specific
//! `LibnxError` / `libnx_error` helpers are defined here.

pub use nx_rt_core::ffi::common::GENERIC_ERROR;
#[cfg(any(
    feature = "service-apm",
    feature = "service-nv",
    feature = "service-set",
    feature = "service-vi",
))]
pub use nx_rt_core::ffi::common::SyncUnsafeCell;
#[cfg(any(
    feature = "service-apm",
    feature = "service-nv",
    feature = "service-set",
    feature = "service-time",
    feature = "service-vi",
))]
pub use nx_rt_core::ffi::common::parse_resp_bytes_error_to_rc;
#[cfg(any(
    feature = "service-apm",
    feature = "service-hid",
    feature = "service-nv",
    feature = "service-set",
    feature = "service-vi",
))]
pub use nx_rt_core::ffi::common::parse_resp_error_to_rc;
#[cfg(feature = "service-set")]
pub use nx_rt_core::ffi::common::parse_tipc_resp_error_to_rc;
#[cfg(any(
    feature = "service-apm",
    feature = "service-hid",
    feature = "service-nv",
    feature = "service-set",
    feature = "service-time",
    feature = "service-vi",
))]
pub use nx_rt_core::ffi::common::send_error_to_rc;

/// libnx error enumeration for MAKERESULT(Module_Libnx, error).
///
/// Values mirror the sequential enum in libnx `include/switch/result.h`
/// (`LibnxError_BadReloc = 1`, ...) so the result codes produced by
/// [`libnx_error`] match what libnx callers expect.
#[cfg(feature = "service-vi")]
#[repr(u32)]
pub enum LibnxError {
    NotInitialized = 8,
    BadInput = 11,
    IncompatSysVer = 37,
}

/// Constructs a libnx result code.
#[cfg(feature = "service-vi")]
pub const fn libnx_error(err: LibnxError) -> u32 {
    const MODULE_LIBNX: u32 = 345;
    (MODULE_LIBNX & 0x1FF) | ((err as u32 & 0x1FFF) << 9)
}
