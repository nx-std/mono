//! System Settings (set:sys) service FFI

use nx_sf::{cmif, service::Service, tipc};
use nx_svc::error::ToRawResultCode;

use super::common::GENERIC_ERROR;
use crate::service_registry;

/// Initializes set:sys connection. Returns 0 on success, error code on failure.
///
/// Corresponds to `setsysInitialize()` in `set.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_initialize() -> u32 {
    if let Err(err) = service_registry::setsys_init() {
        return setsys_connect_error_to_rc(err);
    }
    0
}

/// Closes set:sys connection.
///
/// Corresponds to `setsysExit()` in `set.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_exit() {
    service_registry::setsys_exit();
}

/// Gets the set:sys service session pointer.
///
/// Corresponds to `setsysGetServiceSession()` in `set.h`.
///
/// # Safety
///
/// set:sys must be initialized. The returned pointer points to the Arc-stored
/// session and remains valid as long as the service session is alive.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_get_service_session() -> *mut Service {
    let Some(setsys) = service_registry::setsys_get() else {
        return core::ptr::null_mut();
    };

    // Cast Arc<SetSysService> to *mut Service (SetSysService is repr(transparent))
    alloc::sync::Arc::as_ptr(&setsys) as *mut Service
}

/// Gets the system firmware version. Returns 0 on success, error code on failure.
///
/// Corresponds to `setsysGetFirmwareVersion()` in `set.h`.
///
/// # Safety
///
/// `out` must point to valid, writable memory for a FirmwareVersion struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_get_firmware_version(
    out: *mut nx_service_set::FirmwareVersion,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(setsys) = service_registry::setsys_get() else {
        return GENERIC_ERROR;
    };

    let fw = match setsys.get_firmware_version_cmif() {
        Ok(fw) => fw,
        Err(err) => return setsys_get_firmware_version_error_to_rc(err),
    };

    // SAFETY: Caller guarantees out points to valid memory.
    unsafe { *out = fw };
    0
}

fn setsys_connect_error_to_rc(err: service_registry::SetsysConnectError) -> u32 {
    match err {
        service_registry::SetsysConnectError::Cmif(e) => match e.0 {
            nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
        },
        service_registry::SetsysConnectError::Tipc(e) => match e.0 {
            nx_service_sm::GetServiceTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::GetServiceTipcError::ParseResponse(e) => match e {
                tipc::ParseResponseError::EmptyResponse => GENERIC_ERROR,
                tipc::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::GetServiceTipcError::MissingHandle => GENERIC_ERROR,
        },
    }
}

fn setsys_get_firmware_version_error_to_rc(
    err: nx_service_set::GetFirmwareVersionCmifError,
) -> u32 {
    match err {
        nx_service_set::GetFirmwareVersionCmifError::SendRequest(e) => e.to_rc(),
        nx_service_set::GetFirmwareVersionCmifError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}
