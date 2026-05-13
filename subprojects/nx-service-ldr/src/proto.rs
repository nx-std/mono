//! Loader (`ldr`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the loader shell service (`ldr:shel`).
pub const SHELL_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ldr:shel");

/// Service name for the loader debug/monitor service (`ldr:dmnt`).
pub const DMNT_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ldr:dmnt");

/// Service name for the loader PM service (`ldr:pm`).
pub const PM_SERVICE_NAME: ServiceName = ServiceName::new_truncate("ldr:pm");

// Shell / Dmnt shared commands
pub const SET_PROGRAM_ARGUMENTS: u32 = 0;
pub const FLUSH_ARGUMENTS: u32 = 1;

// Dmnt-only command
pub const DMNT_GET_PROCESS_MODULE_INFO: u32 = 2;

// Pm commands
pub const PM_CREATE_PROCESS: u32 = 0;
pub const PM_GET_PROGRAM_INFO: u32 = 1;
pub const PM_PIN_PROGRAM: u32 = 2;
pub const PM_UNPIN_PROGRAM: u32 = 3;
pub const PM_SET_ENABLED_PROGRAM_VERIFICATION: u32 = 4;
