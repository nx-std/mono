//! SM Manager (`sm:m`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the SM management interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("sm:m");

/// Register a process with the Service Manager.
pub const REGISTER_PROCESS: u32 = 0;

/// Unregister a process from the Service Manager.
pub const UNREGISTER_PROCESS: u32 = 1;
