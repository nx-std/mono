//! Application Performance Management (APM) service FFI

use core::mem::MaybeUninit;

use nx_service_apm;
use nx_sf::{cmif, ffi::Service};
use nx_svc::error::ToRawResultCode;

use super::common::{GENERIC_ERROR, SyncUnsafeCell};
use crate::services::apm;

/// Static buffer for APM IManager FFI session access. Written on
/// `apm_initialize()` and zeroed on `apm_exit()`.
static APM_FFI_SERVICE: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for APM ISession FFI session access. Written on
/// `apm_initialize()` and zeroed on `apm_exit()`.
static APM_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Initializes the APM service. Returns 0 on success, error code on failure.
///
/// Corresponds to `apmInitialize()` in `apm.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_initialize() -> u32 {
    if let Err(err) = apm::init() {
        return apm_connect_error_to_rc(err);
    }

    // Populate FFI service buffers from the owned Session wrappers as
    // non-owning views (`own_handle = 0`, `object_id = 0` — libnx's
    // "override" mode): Rust's `ApmService`/`ApmSession` retain exclusive
    // ownership of the kernel handles and close them on `Drop` via
    // `apm::exit`. A `own_handle = 1` snapshot here would risk a
    // double-close if libnx ever invoked `serviceClose` on the cached
    // pointer. APM does not use pointer buffers, so size 0 is correct.
    if let Some(service) = apm::get_service() {
        let svc = Service {
            session: service.session(),
            own_handle: 0,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during initialization; no other code reads the
        // buffer concurrently.
        unsafe { APM_FFI_SERVICE.get().cast::<Service>().write(svc) };
    }
    if let Some(session) = apm::get_session() {
        let svc = Service {
            session: session.session(),
            own_handle: 0,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during initialization; no other code reads the
        // buffer concurrently.
        unsafe { APM_FFI_SESSION.get().cast::<Service>().write(svc) };
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
    apm::exit();
    // SAFETY: Called only during exit, after the service is closed.
    unsafe {
        APM_FFI_SERVICE.get().write(MaybeUninit::zeroed());
        APM_FFI_SESSION.get().write(MaybeUninit::zeroed());
    }
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

    let Some(service) = apm::get_service() else {
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

    let Some(session) = apm::get_session() else {
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

    let Some(session) = apm::get_session() else {
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
    APM_FFI_SERVICE.get().cast::<Service>()
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
    APM_FFI_SESSION.get().cast::<Service>()
}

fn apm_connect_error_to_rc(err: apm::ConnectError) -> u32 {
    match err {
        apm::ConnectError::Connect(e) => match e {
            nx_service_apm::ConnectError::GetService(e) => match e {
                nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
                nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
            },
        },
        apm::ConnectError::OpenSession(e) => match e {
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
