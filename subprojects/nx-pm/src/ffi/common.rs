//! Shared helpers for the pm FFI surface.

use core::{cell::UnsafeCell, mem::size_of};

use nx_sf::cmif;
use nx_svc::error::ToResultCode;
use static_assertions::const_assert_eq;

/// Generic fallback used when no specific result code is available.
pub(super) const GENERIC_ERROR: u32 = 0xFFFF;

/// `MAKERESULT(Module_Libnx, LibnxError_IncompatSysVer)` — returned by libnx
/// gating checks when the running firmware predates a command. Mirrors
/// `nx-rt`'s SM FFI for the same condition.
pub(super) const RC_INCOMPAT_SYSVER: u32 = 0x8A564;

/// Encodes a major/minor/micro firmware version the same way libnx's
/// `MAKEHOSVERSION` macro does.
#[inline]
pub(super) const fn make_hos_version(major: u8, minor: u8, micro: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (micro as u32)
}

unsafe extern "C" {
    /// Provided by libnx (or its `nx-rt` override) at link time.
    pub(super) safe fn hosversionGet() -> u32;
    /// Provided by libnx (or its `nx-rt` override) at link time.
    pub(super) safe fn hosversionIsAtmosphere() -> bool;
}

/// Returns `true` when the running firmware is at least `major.minor.micro`.
#[inline]
pub(super) fn hosversion_at_least(major: u8, minor: u8, micro: u8) -> bool {
    hosversionGet() >= make_hos_version(major, minor, micro)
}

/// Returns `true` when the firmware predates `major.minor.micro`.
#[inline]
pub(super) fn hosversion_before(major: u8, minor: u8, micro: u8) -> bool {
    !hosversion_at_least(major, minor, micro)
}

/// libnx `Event` layout, used by `pmdmntHookTo*` and `pmshellGetProcessEventHandle`.
///
/// Matches `Event` in `libnx/include/switch/kernel/event.h`:
/// ```c
/// typedef struct {
///     Handle revent;     // u32
///     Handle wevent;     // u32
///     bool   autoclear;  // u8
/// } Event;
/// ```
#[repr(C)]
pub(super) struct LibnxEvent {
    pub revent: u32,
    pub wevent: u32,
    pub autoclear: bool,
}
const_assert_eq!(size_of::<LibnxEvent>(), 12);

/// `UnsafeCell` wrapper that asserts `Sync` for static storage. Access is
/// synchronised externally (by the per-service `RwLock` in `state`).
#[repr(transparent)]
pub(super) struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: synchronisation is provided by the per-service singleton lock.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub(super) const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub(super) fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// Converts a CMIF dispatch failure to its raw libnx result code.
pub(super) fn dispatch_error_to_rc(err: nx_sf::service::DispatchError) -> u32 {
    match err {
        nx_sf::service::DispatchError::Layout(_) => GENERIC_ERROR,
        nx_sf::service::DispatchError::SendRequest(e) => e.to_rc(),
        nx_sf::service::DispatchError::ParseResponse(e) => parse_resp_bytes_error_to_rc(e),
    }
}

/// Converts an SM `GetService` failure to its raw libnx result code.
pub(super) fn sm_get_service_error_to_rc(err: nx_service_sm::GetServiceCmifError) -> u32 {
    match err {
        nx_service_sm::GetServiceCmifError::SendRequest(e) => send_error_to_rc(e),
        nx_service_sm::GetServiceCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
        nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
    }
}

/// Converts an SM connect failure to its raw libnx result code.
pub(super) fn sm_connect_error_to_rc(err: nx_service_sm::ConnectError) -> u32 {
    match err {
        nx_service_sm::ConnectError::Connect(e) => e.to_rc(),
        nx_service_sm::ConnectError::RegisterClient(e) => match e {
            nx_service_sm::RegisterClientCmifError::SendRequest(e) => send_error_to_rc(e),
            nx_service_sm::RegisterClientCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
        },
    }
}

/// Converts a request send failure to a raw result code.
fn send_error_to_rc(err: cmif::SendError) -> u32 {
    match err {
        cmif::SendError::Layout(_) => GENERIC_ERROR,
        cmif::SendError::SendRequest(e) => e.to_rc(),
    }
}

/// Translates a `pm:*` service connect error to a raw result code.
pub(super) fn connect_error_to_rc(err: nx_service_sm::GetServiceCmifError) -> u32 {
    sm_get_service_error_to_rc(err)
}

/// Converts a CMIF [`cmif::ParseError`] to a raw libnx result code.
fn parse_resp_error_to_rc(err: cmif::ParseError) -> u32 {
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

/// Converts a CMIF [`cmif::ParseError`] to a raw libnx result code.
fn parse_resp_bytes_error_to_rc(err: cmif::ParseError) -> u32 {
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
