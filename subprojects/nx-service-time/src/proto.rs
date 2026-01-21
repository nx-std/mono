//! Time service protocol constants and types.

use nx_sf::ServiceName;

/// Service name for time:u (User time service).
pub const SERVICE_NAME_USER: ServiceName = ServiceName::new_truncate("time:u");

/// Service name for time:a (Menu/Applet time service).
pub const SERVICE_NAME_MENU: ServiceName = ServiceName::new_truncate("time:a");

/// Service name for time:s (System time service).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("time:s");

/// Service name for time:r (Repair time service, 9.0.0+).
pub const SERVICE_NAME_REPAIR: ServiceName = ServiceName::new_truncate("time:r");

/// Service name for time:su (SystemUser time service, 9.0.0+).
pub const SERVICE_NAME_SYSTEM_USER: ServiceName = ServiceName::new_truncate("time:su");

/// IStaticService (time:*) command IDs
pub mod static_service_cmds {
    /// Get standard user system clock (ISystemClock).
    pub const GET_STANDARD_USER_SYSTEM_CLOCK: u32 = 0;

    /// Get standard network system clock (ISystemClock).
    pub const GET_STANDARD_NETWORK_SYSTEM_CLOCK: u32 = 1;

    /// Get standard steady clock (ISteadyClock).
    pub const GET_STANDARD_STEADY_CLOCK: u32 = 2;

    /// Get time zone service (ITimeZoneService).
    pub const GET_TIME_ZONE_SERVICE: u32 = 3;

    /// Get standard local system clock (ISystemClock).
    #[expect(dead_code)]
    pub const GET_STANDARD_LOCAL_SYSTEM_CLOCK: u32 = 4;

    /// [6.0.0+] Get shared memory native handle.
    pub const GET_SHARED_MEMORY_NATIVE_HANDLE: u32 = 20;
}

/// ISystemClock command IDs
pub mod system_clock_cmds {
    /// Get current time (POSIX timestamp).
    pub const GET_CURRENT_TIME: u32 = 0;

    /// Set current time (POSIX timestamp).
    #[expect(dead_code)]
    pub const SET_CURRENT_TIME: u32 = 1;
}

/// ISteadyClock command IDs
pub mod steady_clock_cmds {
    /// Get current time point.
    #[expect(dead_code)]
    pub const GET_CURRENT_TIME_POINT: u32 = 0;

    /// [3.0.0+] Get standard steady clock internal offset.
    #[expect(dead_code)]
    pub const GET_INTERNAL_OFFSET: u32 = 200;
}

/// ITimeZoneService command IDs
pub mod timezone_service_cmds {
    /// Get device location name.
    #[expect(dead_code)]
    pub const GET_DEVICE_LOCATION_NAME: u32 = 0;

    /// Set device location name.
    #[expect(dead_code)]
    pub const SET_DEVICE_LOCATION_NAME: u32 = 1;

    /// Get total location name count.
    #[expect(dead_code)]
    pub const GET_TOTAL_LOCATION_NAME_COUNT: u32 = 2;

    /// To calendar time with my rule.
    pub const TO_CALENDAR_TIME_WITH_MY_RULE: u32 = 101;

    /// To POSIX time (convert calendar time to POSIX timestamp).
    #[expect(dead_code)]
    pub const TO_POSIX_TIME: u32 = 201;

    /// To POSIX time with my rule.
    #[expect(dead_code)]
    pub const TO_POSIX_TIME_WITH_MY_RULE: u32 = 202;
}
