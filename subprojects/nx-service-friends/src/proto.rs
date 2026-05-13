//! Friends service protocol constants.

use nx_sf::ServiceName;

/// Service name for `friend:u` (user).
pub const SERVICE_NAME_USER: ServiceName = ServiceName::new_truncate("friend:u");

/// Service name for `friend:v` (viewer).
pub const SERVICE_NAME_VIEWER: ServiceName = ServiceName::new_truncate("friend:v");

/// Service name for `friend:m` (manager).
pub const SERVICE_NAME_MANAGER: ServiceName = ServiceName::new_truncate("friend:m");

/// Service name for `friend:s` (system).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("friend:s");

/// Service name for `friend:a` (administrator).
pub const SERVICE_NAME_ADMIN: ServiceName = ServiceName::new_truncate("friend:a");

// IServiceCreator commands

/// Creates IFriendService sub-object (domain object).
pub const CREATE_FRIEND_SERVICE: u32 = 0;

// IFriendService commands

/// Gets the user setting for an account.
pub const GET_USER_SETTING: u32 = 20800;
