//! I2C service protocol constants.

use nx_sf::ServiceName;

/// Service name for the I2C bus service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("i2c");

// II2cManager commands

/// Opens a session for a specific I2C device.
pub const OPEN_SESSION: u32 = 1;

// II2cSession commands

/// Sends data to the I2C device with automatic buffer selection.
pub const SEND_AUTO: u32 = 10;

/// Receives data from the I2C device with automatic buffer selection.
pub const RECEIVE_AUTO: u32 = 11;

/// Executes a command list on the I2C device.
pub const EXECUTE_COMMAND_LIST: u32 = 12;
