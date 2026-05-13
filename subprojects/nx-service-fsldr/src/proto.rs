//! Filesystem loader service protocol constants.

use nx_sf::ServiceName;

/// Service name for the filesystem-proxy-for-loader interface (`fsp-ldr`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("fsp-ldr");

/// Opens a code filesystem for a given title.
pub const OPEN_CODE_FILE_SYSTEM: u32 = 0;

/// Checks whether a program (by PID) is archived.
pub const IS_ARCHIVED_PROGRAM: u32 = 1;

/// Sets the current process (sends PID to the service).
pub const SET_CURRENT_PROCESS: u32 = 2;
