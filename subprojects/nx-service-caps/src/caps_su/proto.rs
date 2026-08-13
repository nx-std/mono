//! Screenshot upload service protocol constants.

use nx_sf::ServiceName;

/// Service name for the screenshot upload service (`caps:su`).
pub const CAPSSU_SERVICE_NAME: ServiceName = ServiceName::new_truncate("caps:su");

// IScreenShotApplicationService commands

/// Sets the shim library version. [7.0.0+]
pub const SET_SHIM_LIBRARY_VERSION: u32 = 32;

/// Saves a screenshot with attributes. [4.0.0+]
pub const SAVE_SCREEN_SHOT_EX0: u32 = 203;

/// Saves a screenshot with attributes and application data. [7.0.0+]
pub const SAVE_SCREEN_SHOT_EX1: u32 = 205;

/// Saves a screenshot with attributes and user IDs. [6.0.0+]
pub const SAVE_SCREEN_SHOT_EX2: u32 = 210;
