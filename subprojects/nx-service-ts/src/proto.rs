//! Temperature measurement service protocol constants.

use nx_sf::ServiceName;

/// Service name for the temperature measurement service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("ts");

// ITemperatureMeasurement commands

/// Gets the temperature range for a location. [1.0.0-16.1.0]
pub const GET_TEMPERATURE_RANGE: u32 = 0;

/// Gets the temperature for a location. [1.0.0-16.1.0]
pub const GET_TEMPERATURE: u32 = 1;

/// Gets the temperature in millicelsius for a location. [1.0.0-13.2.1]
pub const GET_TEMPERATURE_MILLI_C: u32 = 3;

/// Opens a session for a specific device code. [8.0.0+]
pub const OPEN_SESSION: u32 = 4;

// ITsSession commands

/// Gets the temperature as a float. [10.0.0+]
pub const SESSION_GET_TEMPERATURE: u32 = 4;
