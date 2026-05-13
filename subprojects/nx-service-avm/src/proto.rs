//! AV module service protocol constants.

use nx_sf::ServiceName;

/// Service name for the AV module service (`avm`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("avm");

// IVersionListController commands

/// Gets the highest available version for a title pair. [6.0.0+]
pub const GET_HIGHEST_AVAILABLE_VERSION: u32 = 100;

/// Gets the highest required version for a title pair. [6.0.0+]
pub const GET_HIGHEST_REQUIRED_VERSION: u32 = 101;

/// Gets a single version list entry by application ID. [6.0.0+]
pub const GET_VERSION_LIST_ENTRY: u32 = 102;

/// Gets a version list importer sub-object. [6.0.0+]
pub const GET_VERSION_LIST_IMPORTER: u32 = 103;

/// Gets the launch-required version for an application. [6.0.0+]
pub const GET_LAUNCH_REQUIRED_VERSION: u32 = 200;

/// Upgrades the launch-required version for an application. [6.0.0+]
pub const UPGRADE_LAUNCH_REQUIRED_VERSION: u32 = 202;

/// Pushes the launch version for an application. [6.0.0+]
pub const PUSH_LAUNCH_VERSION: u32 = 1000;

/// Lists all version list entries into a buffer. [6.0.0+]
pub const LIST_VERSION_LIST: u32 = 1001;

/// Lists all required-version entries into a buffer. [6.0.0+]
pub const LIST_REQUIRED_VERSION: u32 = 1002;

// IVersionListImporter commands

/// Sets the timestamp on the importer. [6.0.0+]
pub const IMPORTER_SET_TIMESTAMP: u32 = 0;

/// Sets the version list data on the importer. [6.0.0+]
pub const IMPORTER_SET_DATA: u32 = 1;

/// Flushes the importer, committing the data. [6.0.0+]
pub const IMPORTER_FLUSH: u32 = 2;
