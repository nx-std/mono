//! Mii service protocol constants.

use nx_sf::ServiceName;

/// Service name for the Mii system interface (`mii:e`).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("mii:e");

/// Service name for the Mii user interface (`mii:u`).
pub const SERVICE_NAME_USER: ServiceName = ServiceName::new_truncate("mii:u");

// Root service commands (IDatabaseService / IStaticService)

/// Opens a Mii database sub-object.
pub const OPEN_DATABASE: u32 = 0;

// MiiDatabase commands (IDatabaseService)

/// Checks if the database has been updated.
pub const DB_IS_UPDATED: u32 = 0;

/// Checks if the database is full.
pub const DB_IS_FULL: u32 = 1;

/// Gets the number of Miis matching a source flag.
pub const DB_GET_COUNT: u32 = 2;

/// Gets Mii character info entries matching a source flag.
pub const DB_GET1: u32 = 4;

/// Builds a random Mii character info.
pub const DB_BUILD_RANDOM: u32 = 6;
