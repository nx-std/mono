//! System Power State Manager (`spsm`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the SPSM interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("spsm");

/// Initiates system shutdown or reboot.
pub const SHUTDOWN: u32 = 3;

/// Puts the system into an error state.
pub const PUT_ERROR_STATE: u32 = 10;
