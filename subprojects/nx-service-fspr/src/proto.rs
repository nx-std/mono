//! FilesystemProxy-ProgramRegistry (`fsp-pr`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the fsp-pr interface.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("fsp-pr");

/// Registers a program's filesystem access controls.
pub const REGISTER_PROGRAM: u32 = 0;

/// Unregisters a program's filesystem access controls.
pub const UNREGISTER_PROGRAM: u32 = 1;

/// Sets the current process on the fsp-pr session.
pub const SET_CURRENT_PROCESS: u32 = 2;

/// Enables or disables program verification (removed in `[10.0.0+]`).
pub const SET_ENABLED_PROGRAM_VERIFICATION: u32 = 256;
