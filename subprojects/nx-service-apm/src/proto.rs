//! APM protocol constants and types.

use nx_sf::ServiceName;

/// Service name for APM.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("apm");

/// IManager command: OpenSession.
pub const CMD_OPEN_SESSION: u32 = 0;

/// IManager command: GetPerformanceMode.
pub const CMD_GET_PERFORMANCE_MODE: u32 = 1;

/// ISession command: SetPerformanceConfiguration.
pub const CMD_SET_PERFORMANCE_CONFIGURATION: u32 = 0;

/// ISession command: GetPerformanceConfiguration.
pub const CMD_GET_PERFORMANCE_CONFIGURATION: u32 = 1;

/// Performance mode (Normal vs Boost).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PerformanceMode {
    /// Invalid performance mode.
    Invalid = -1,
    /// Normal performance mode.
    Normal = 0,
    /// Boost performance mode (higher clocks).
    Boost = 1,
}

impl PerformanceMode {
    /// Converts a raw i32 value to a PerformanceMode.
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            -1 => Some(Self::Invalid),
            0 => Some(Self::Normal),
            1 => Some(Self::Boost),
            _ => None,
        }
    }

    /// Returns true if this is a valid mode (not Invalid).
    pub fn is_valid(self) -> bool {
        !matches!(self, Self::Invalid)
    }
}
