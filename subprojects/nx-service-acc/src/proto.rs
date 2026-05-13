//! Account service protocol constants.

use nx_sf::ServiceName;

/// Service name for the application interface (`acc:u0`).
pub const SERVICE_NAME_APPLICATION: ServiceName = ServiceName::new_truncate("acc:u0");

/// Service name for the system interface (`acc:u1`).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("acc:u1");

/// Service name for the administrator interface (`acc:su`).
pub const SERVICE_NAME_ADMINISTRATOR: ServiceName = ServiceName::new_truncate("acc:su");

// Root service commands

/// Gets the total number of user profiles.
pub const GET_USER_COUNT: u32 = 0;

/// Lists all user IDs (HipcPointer out buffer).
pub const LIST_ALL_USERS: u32 = 2;

/// Gets the last opened user ID.
pub const GET_LAST_OPENED_USER: u32 = 4;

/// Gets an IProfile sub-object for a user.
pub const GET_PROFILE: u32 = 5;

/// Checks if user registration is permitted (sends PID).
pub const IS_USER_REGISTRATION_REQUEST_PERMITTED: u32 = 50;

/// Selects a user without applet interaction.
pub const TRY_SELECT_USER_WITHOUT_INTERACTION: u32 = 51;

/// Initializes application info (pre-6.0.0, sends PID).
pub const INITIALIZE_APPLICATION_INFO_LEGACY: u32 = 100;

/// Initializes application info (6.0.0+, sends PID).
pub const INITIALIZE_APPLICATION_INFO: u32 = 140;

// IProfile commands

/// Gets profile base and optional user data.
pub const PROFILE_GET: u32 = 0;

/// Gets profile base only.
pub const PROFILE_GET_BASE: u32 = 1;

/// Gets the profile icon image size.
pub const PROFILE_GET_IMAGE_SIZE: u32 = 10;

/// Loads the JPEG profile icon image.
pub const PROFILE_LOAD_IMAGE: u32 = 11;
