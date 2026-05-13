//! Capture MTP service protocol constants.

use nx_sf::ServiceName;

/// Service name for the capture MTP service (`capmtp`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("capmtp");

// Root service commands

/// Opens a session sub-object. [11.0.0+]
pub const OPEN_SESSION: u32 = 0;

// Session sub-object commands

/// Opens the MTP session with transfer memory and device name. [11.0.0+]
pub const SESSION_OPEN: u32 = 0;

/// Closes the MTP session. [11.0.0+]
pub const SESSION_CLOSE: u32 = 1;

/// Starts the MTP command handler. [11.0.0+]
pub const SESSION_START_COMMAND_HANDLER: u32 = 2;

/// Stops the MTP command handler. [11.0.0+]
pub const SESSION_STOP_COMMAND_HANDLER: u32 = 3;

/// Checks whether the command handler is running. [11.0.0+]
pub const SESSION_IS_RUNNING: u32 = 4;

/// Gets the connection event handle. [11.0.0+]
pub const SESSION_GET_CONNECTION_EVENT: u32 = 5;

/// Checks whether a USB device is connected. [11.0.0+]
pub const SESSION_IS_CONNECTED: u32 = 6;

/// Gets the scan-error event handle. [11.0.0+]
pub const SESSION_GET_SCAN_ERROR_EVENT: u32 = 7;

/// Gets the scan-error result code. [11.0.0+]
pub const SESSION_GET_SCAN_ERROR: u32 = 8;
