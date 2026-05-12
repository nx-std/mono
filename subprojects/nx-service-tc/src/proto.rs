//! Temperature control service protocol constants.

use nx_sf::ServiceName;

/// Service name for the temperature control interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("tc");

/// Enables fan control.
pub const ENABLE_FAN_CONTROL: u32 = 6;

/// Disables fan control.
pub const DISABLE_FAN_CONTROL: u32 = 7;

/// Queries whether fan control is enabled.
pub const IS_FAN_CONTROL_ENABLED: u32 = 8;

/// Gets the skin temperature in milli-degrees Celsius.
pub const GET_SKIN_TEMPERATURE_MILLI_C: u32 = 9;
