//! Time service FFI

use nx_rt_core::error::ToResultCode as _;
use nx_service_time;
use nx_sf::error::ToResultCode;

use crate::ffi::common::GENERIC_ERROR;

/// Initializes the Time service.
///
/// Corresponds to `timeInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_time_initialize() -> u32 {
    match crate::services::time::init() {
        Ok(()) => 0,
        // The manager owns the mapping for its own failures; a second copy
        // here would drift from it.
        Err(err) => err.to_rc(),
    }
}

/// Anchors the realtime clock and publishes the device's timezone to the C environment.
///
/// Corresponds to `__libnx_init_time()` in libnx.
///
/// # Safety
///
/// The Time service must already be initialized; there are no arguments to uphold anything about.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_init_time() {
    // The error is dropped because there is nothing to report it to: libnx declares
    // `__libnx_init_time` as `void` and the C startup path calls it for its side effects alone.
    // A failure leaves the clock unanchored, which surfaces later as `EIO` from
    // `clock_gettime(CLOCK_REALTIME)` and `gettimeofday`.
    let _ = crate::services::time::init_wall_clock();
}

/// Exits the Time service.
///
/// Corresponds to `timeExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_time_exit() {
    crate::services::time::exit();
}

/// Gets the current time from the specified clock type.
///
/// Corresponds to `timeGetCurrentTime()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_time_get_current_time(
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
                nx_service_time::GetCurrentTimeError::SendRequest(e) => e.to_rc(),
                nx_service_time::GetCurrentTimeError::ParseResponse(e) => e.to_rc(),
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
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_time_to_calendar_time_with_my_rule(
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
                // SAFETY: both pointers were checked non-null above, and the
                // caller guarantees each addresses a writable value of its type.
                unsafe {
                    *caltime = cal;
                    *info = inf;
                }
                0
            }
            Err(err) => match err {
                nx_service_time::ToCalendarTimeError::SendRequest(e) => e.to_rc(),
                nx_service_time::ToCalendarTimeError::ParseResponse(e) => e.to_rc(),
            },
        },
        None => GENERIC_ERROR,
    }
}
