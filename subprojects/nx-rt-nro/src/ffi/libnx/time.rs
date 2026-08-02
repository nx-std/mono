//! Time service FFI

use nx_service_time;

use crate::ffi::common::{GENERIC_ERROR, parse_resp_bytes_error_to_rc, send_error_to_rc};

/// Initializes the Time service.
///
/// Corresponds to `timeInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_time_initialize() -> u32 {
    match crate::services::time::init() {
        Ok(()) => 0,
        Err(err) => {
            // Convert error to result code
            let crate::services::time::ConnectError(conn_err) = err;
            match conn_err {
                nx_service_time::ConnectError::GetService(sm_err) => match sm_err {
                    nx_service_sm::GetServiceCmifError::SendRequest(e) => send_error_to_rc(e),
                    _ => GENERIC_ERROR,
                },
                nx_service_time::ConnectError::GetUserSystemClock(clock_err) => match clock_err {
                    nx_service_time::GetSystemClockError::SendRequest(e) => send_error_to_rc(e),
                    nx_service_time::GetSystemClockError::ParseResponse(e) => {
                        parse_resp_bytes_error_to_rc(e)
                    }
                    nx_service_time::GetSystemClockError::MissingHandle => GENERIC_ERROR,
                },
                nx_service_time::ConnectError::GetSteadyClock(steady_err) => match steady_err {
                    nx_service_time::GetSteadyClockError::SendRequest(e) => send_error_to_rc(e),
                    nx_service_time::GetSteadyClockError::ParseResponse(e) => {
                        parse_resp_bytes_error_to_rc(e)
                    }
                    nx_service_time::GetSteadyClockError::MissingHandle => GENERIC_ERROR,
                },
                nx_service_time::ConnectError::GetTimeZoneService(tz_err) => match tz_err {
                    nx_service_time::GetTimeZoneServiceError::SendRequest(e) => send_error_to_rc(e),
                    nx_service_time::GetTimeZoneServiceError::ParseResponse(e) => {
                        parse_resp_bytes_error_to_rc(e)
                    }
                    nx_service_time::GetTimeZoneServiceError::MissingHandle => GENERIC_ERROR,
                },
            }
        }
    }
}

/// Exits the Time service.
///
/// Corresponds to `timeExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_time_exit() {
    crate::services::time::exit();
}

/// Gets the current time from the specified clock type.
///
/// Corresponds to `timeGetCurrentTime()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_time_get_current_time(
    clock_type: u32,
    timestamp: *mut u64,
) -> u32 {
    if timestamp.is_null() {
        return GENERIC_ERROR;
    }

    let time_type = match clock_type {
        0 => nx_service_time::TimeType::UserSystemClock,
        1 => nx_service_time::TimeType::NetworkSystemClock,
        2 => nx_service_time::TimeType::LocalSystemClock,
        _ => return GENERIC_ERROR,
    };

    match crate::services::time::get_service() {
        Some(service) => match service.get_current_time(time_type) {
            Ok(time) => {
                unsafe { *timestamp = time };
                0
            }
            Err(err) => match err {
                nx_service_time::GetCurrentTimeError::SendRequest(e) => send_error_to_rc(e),
                nx_service_time::GetCurrentTimeError::ParseResponse(e) => {
                    parse_resp_bytes_error_to_rc(e)
                }
                nx_service_time::GetCurrentTimeError::NetworkClockUnavailable => GENERIC_ERROR,
                nx_service_time::GetCurrentTimeError::LocalClockNotSupported => GENERIC_ERROR,
                nx_service_time::GetCurrentTimeError::SourceIdMismatch => GENERIC_ERROR,
            },
        },
        None => GENERIC_ERROR,
    }
}

/// Converts a POSIX timestamp to calendar time using the device's timezone.
///
/// Corresponds to `timeToCalendarTimeWithMyRule()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_time_to_calendar_time_with_my_rule(
    timestamp: u64,
    caltime: *mut nx_service_time::TimeCalendarTime,
    info: *mut nx_service_time::TimeCalendarAdditionalInfo,
) -> u32 {
    if caltime.is_null() || info.is_null() {
        return GENERIC_ERROR;
    }

    match crate::services::time::get_service() {
        Some(service) => match service.to_calendar_time_with_my_rule(timestamp) {
            Ok((cal, inf)) => {
                unsafe {
                    *caltime = cal;
                    *info = inf;
                }
                0
            }
            Err(err) => match err {
                nx_service_time::ToCalendarTimeError::SendRequest(e) => send_error_to_rc(e),
                nx_service_time::ToCalendarTimeError::ParseResponse(e) => {
                    parse_resp_bytes_error_to_rc(e)
                }
            },
        },
        None => GENERIC_ERROR,
    }
}
