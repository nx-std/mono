//! Network Install Manager (`nim`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the NIM interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("nim");

/// Destroys a system update task.
pub const DESTROY_SYSTEM_UPDATE_TASK: u32 = 1;

/// Lists all system update tasks.
pub const LIST_SYSTEM_UPDATE_TASK: u32 = 2;
