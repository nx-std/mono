//! Mii image service protocol constants.

use nx_sf::ServiceName;

/// Service name for the mii image service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("miiimg");

// IMiiImageDatabaseService commands

/// Initializes the image database. [5.0.0+]
pub const INITIALIZE: u32 = 0;

/// Reloads the image database. [5.0.0+]
pub const RELOAD: u32 = 10;

/// Gets the number of mii images. [5.0.0+]
pub const GET_COUNT: u32 = 11;

/// Gets whether the image database is empty. [5.0.0+]
pub const IS_EMPTY: u32 = 12;

/// Gets whether the image database is full. [5.0.0+]
pub const IS_FULL: u32 = 13;

/// Gets the image attribute at an index. [5.0.0+]
pub const GET_ATTRIBUTE: u32 = 14;

/// Loads the image data (raw RGBA8) for an image ID. [5.0.0+]
pub const LOAD_IMAGE: u32 = 15;
