//! Time service data types.

/// Time service type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TimeServiceType {
    /// User time service (time:u).
    User = 0,
    /// Menu/Applet time service (time:a).
    Menu = 1,
    /// System time service (time:s).
    System = 2,
    /// Repair time service (time:r) - Only available with 9.0.0+.
    Repair = 3,
    /// SystemUser time service (time:su) - Only available with 9.0.0+.
    SystemUser = 4,
}

/// Time clock type for selecting which system clock to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum TimeType {
    /// User system clock.
    #[default]
    UserSystemClock = 0,
    /// Network system clock.
    NetworkSystemClock = 1,
    /// Local system clock.
    LocalSystemClock = 2,
}

/// Calendar time representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct TimeCalendarTime {
    /// Year
    pub year: u16,
    /// Month (1-12)
    pub month: u8,
    /// Day (1-31)
    pub day: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
    /// Padding
    pub pad: u8,
}

/// Additional calendar information.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TimeCalendarAdditionalInfo {
    /// 0-based day-of-week (0 = Sunday).
    pub wday: u32,
    /// 0-based day-of-year.
    pub yday: u32,
    /// Timezone name string.
    pub timezone_name: [u8; 8],
    /// DST flag (0 = no DST, 1 = DST).
    pub dst: u32,
    /// Seconds relative to UTC for this timezone.
    pub offset: i32,
}

/// Steady clock time point.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TimeSteadyClockTimePoint {
    /// Monotonic count in seconds.
    pub time_point: i64,
    /// An ID representing the clock source (UUID).
    pub source_id: [u8; 16],
}

/// Standard steady clock time point type (used in shared memory).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TimeStandardSteadyClockTimePointType {
    /// Base time in nanoseconds.
    pub base_time: i64,
    /// An ID representing the clock source (UUID).
    pub source_id: [u8; 16],
}

/// System clock context.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TimeSystemClockContext {
    /// Offset from steady clock.
    pub offset: i64,
    /// Steady clock timestamp.
    pub timestamp: TimeSteadyClockTimePoint,
}
