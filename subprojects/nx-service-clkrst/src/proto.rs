//! Clock/Reset (`clkrst`) protocol constants.

use nx_sf::ServiceName;

/// Service name for clkrst. Available on HOS [8.0.0+].
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("clkrst");

// IClkrstManager commands

/// Opens a [`ClkrstSession`](crate::ClkrstSession) for a given module.
pub const OPEN_SESSION: u32 = 0;

// IClkrstSession commands

/// Sets the clock rate in Hz.
pub const SET_CLOCK_RATE: u32 = 7;

/// Gets the current clock rate in Hz.
pub const GET_CLOCK_RATE: u32 = 8;

/// Gets the list of possible clock rates.
pub const GET_POSSIBLE_CLOCK_RATES: u32 = 10;
