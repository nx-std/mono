//! Fan service protocol constants.

use nx_sf::ServiceName;

/// Service name for the fan interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("fan");

// IFanManager commands

/// Opens an `IController` session for a given device.
pub const OPEN_CONTROLLER: u32 = 0;

// IController commands

/// Sets the fan rotation speed level.
pub const SET_ROTATION_SPEED_LEVEL: u32 = 0;

/// Gets the current fan rotation speed level.
pub const GET_ROTATION_SPEED_LEVEL: u32 = 2;
