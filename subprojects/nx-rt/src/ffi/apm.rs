//! Application Performance Management (APM) service FFI

use nx_service_apm;
use nx_sf::{cmif, service::Service};
use nx_svc::error::ToRawResultCode;

use super::common::GENERIC_ERROR;
use crate::apm_manager;

/// Initializes the APM service. Returns 0 on success, error code on failure.
///
/// Corresponds to `apmInitialize()` in `apm.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_initialize() -> u32 {
    if let Err(err) = apm_manager::init() {
        return apm_connect_error_to_rc(err);
    }
    0
}

/// Closes the APM service connection.
///
/// Corresponds to `apmExit()` in `apm.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_exit() {
    apm_manager::exit();
}

/// Gets the current performance mode.
///
/// Corresponds to `apmGetPerformanceMode()` in `apm.h`.
///
/// # Safety
///
/// Caller guarantees out points to valid i32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_performance_mode(out: *mut i32) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = apm_manager::get_service() else {
        return GENERIC_ERROR;
    };

    match service.get_performance_mode() {
        Ok(mode) => {
            // SAFETY: Caller guarantees out points to valid memory.
            unsafe { *out = mode as i32 };
            0
        }
        Err(err) => apm_get_performance_mode_error_to_rc(err),
    }
}

/// Sets the performance configuration for a mode.
///
/// Corresponds to `apmSetPerformanceConfiguration()` in `apm.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_set_performance_configuration(mode: i32, config: u32) -> u32 {
    let Some(perf_mode) = nx_service_apm::PerformanceMode::from_raw(mode) else {
        return GENERIC_ERROR;
    };

    let Some(session) = apm_manager::get_session() else {
        return GENERIC_ERROR;
    };

    match session.set_performance_configuration(perf_mode, config) {
        Ok(()) => 0,
        Err(err) => apm_set_performance_configuration_error_to_rc(err),
    }
}

/// Gets the performance configuration for a mode.
///
/// Corresponds to `apmGetPerformanceConfiguration()` in `apm.h`.
///
/// # Safety
///
/// Caller guarantees out points to valid u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_performance_configuration(
    mode: i32,
    out: *mut u32,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(perf_mode) = nx_service_apm::PerformanceMode::from_raw(mode) else {
        return GENERIC_ERROR;
    };

    let Some(session) = apm_manager::get_session() else {
        return GENERIC_ERROR;
    };

    match session.get_performance_configuration(perf_mode) {
        Ok(config) => {
            // SAFETY: Caller guarantees out points to valid memory.
            unsafe { *out = config };
            0
        }
        Err(err) => apm_get_performance_configuration_error_to_rc(err),
    }
}

/// Gets the APM service session for C interop.
///
/// Corresponds to `apmGetServiceSession()` in `apm.h`.
///
/// # Safety
///
/// Returns a pointer to the service session or null if not initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_service_session() -> *mut Service {
    let Some(service) = apm_manager::get_service() else {
        return core::ptr::null_mut();
    };

    // SAFETY: Session handle is valid while service is initialized.
    // Lifetime safety guaranteed by singleton management.
    (&*service as *const _ as *mut Service)
        .cast_const()
        .cast_mut()
}

/// Gets the APM ISession for C interop.
///
/// Corresponds to `apmGetServiceSession_Session()` in `apm.h`.
///
/// # Safety
///
/// Returns a pointer to the session or null if not initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_service_session_session() -> *mut Service {
    let Some(session) = apm_manager::get_session() else {
        return core::ptr::null_mut();
    };

    // SAFETY: Session handle is valid while APM is initialized.
    // Lifetime safety guaranteed by singleton management.
    (&*session as *const _ as *mut Service)
        .cast_const()
        .cast_mut()
}

fn apm_connect_error_to_rc(err: apm_manager::ConnectError) -> u32 {
    match err {
        apm_manager::ConnectError::Connect(e) => match e {
            nx_service_apm::ConnectError::GetService(e) => match e {
                nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
                nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
            },
        },
        apm_manager::ConnectError::OpenSession(e) => match e {
            nx_service_apm::OpenSessionError::SendRequest(e) => e.to_rc(),
            nx_service_apm::OpenSessionError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_apm::OpenSessionError::MissingHandle => GENERIC_ERROR,
        },
    }
}

fn apm_get_performance_mode_error_to_rc(err: nx_service_apm::GetPerformanceModeError) -> u32 {
    match err {
        nx_service_apm::GetPerformanceModeError::SendRequest(e) => e.to_rc(),
        nx_service_apm::GetPerformanceModeError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_apm::GetPerformanceModeError::InvalidResponse => GENERIC_ERROR,
        nx_service_apm::GetPerformanceModeError::InvalidMode(_) => GENERIC_ERROR,
    }
}

fn apm_set_performance_configuration_error_to_rc(
    err: nx_service_apm::SetPerformanceConfigurationError,
) -> u32 {
    match err {
        nx_service_apm::SetPerformanceConfigurationError::SendRequest(e) => e.to_rc(),
        nx_service_apm::SetPerformanceConfigurationError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn apm_get_performance_configuration_error_to_rc(
    err: nx_service_apm::GetPerformanceConfigurationError,
) -> u32 {
    match err {
        nx_service_apm::GetPerformanceConfigurationError::SendRequest(e) => e.to_rc(),
        nx_service_apm::GetPerformanceConfigurationError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_apm::GetPerformanceConfigurationError::InvalidResponse => GENERIC_ERROR,
    }
}
