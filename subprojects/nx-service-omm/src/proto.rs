//! Operation Mode Manager (`omm`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the operation mode manager service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("omm");

/// Get the current operation mode. Output: `u8` (OperationMode).
pub const GET_OPERATION_MODE: u32 = 0;

/// Set the operation mode policy (3.0.0+). Input: `u8` (OperationModePolicy).
pub const SET_OPERATION_MODE_POLICY: u32 = 10;

/// Get the default display resolution (3.0.0+). Output: `{s32, s32}`.
pub const GET_DEFAULT_DISPLAY_RESOLUTION: u32 = 11;
