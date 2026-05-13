//! Power state controller service protocol constants.

use nx_sf::ServiceName;

/// Service name for the power state controller service (`psc:m`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("psc:m");

// IPmControl commands
pub const GET_PM_MODULE: u32 = 0;

// IPmModule commands
pub const MODULE_INITIALIZE: u32 = 0;
pub const MODULE_GET_REQUEST: u32 = 1;
pub const MODULE_ACKNOWLEDGE_LEGACY: u32 = 2;
pub const MODULE_FINALIZE: u32 = 3;
pub const MODULE_ACKNOWLEDGE: u32 = 4;
