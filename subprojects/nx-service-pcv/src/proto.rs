//! PCV protocol constants.

use nx_sf::ServiceName;

/// Service name for the PCV service (pre-8.0.0).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pcv");

/// Set the clock rate for a module.
pub const SET_CLOCK_RATE: u32 = 2;

/// Get the clock rate for a module.
pub const GET_CLOCK_RATE: u32 = 3;

/// Get the list of possible clock rates for a module.
pub const GET_POSSIBLE_CLOCK_RATES: u32 = 5;

/// Set voltage enabled state for a power domain.
pub const SET_VOLTAGE_ENABLED: u32 = 8;

/// Get voltage enabled state for a power domain.
pub const GET_VOLTAGE_ENABLED: u32 = 9;
