//! Parental controls service protocol constants.

use nx_sf::ServiceName;

/// Primary service name for the parental controls service (`pctl:a`).
pub const SERVICE_NAME_A: ServiceName = ServiceName::new_truncate("pctl:a");

/// Fallback service name (`pctl:s`).
pub const SERVICE_NAME_S: ServiceName = ServiceName::new_truncate("pctl:s");

/// Fallback service name (`pctl:r`).
pub const SERVICE_NAME_R: ServiceName = ServiceName::new_truncate("pctl:r");

/// Lowest-privilege service name (`pctl`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pctl");

// IParentalControlServiceFactory commands

/// Creates IParentalControlService (pre-4.0.0 wire format).
pub const CREATE_SERVICE_LEGACY: u32 = 0;

/// Creates IParentalControlService (4.0.0+ wire format).
pub const CREATE_SERVICE: u32 = 1;

// IParentalControlService commands

/// Confirms launch-application permission (called during post-init on 4.0.0+).
pub const CONFIRM_LAUNCH_APPLICATION_PERMISSION: u32 = 1;

/// Checks whether parental controls restrictions are temporarily unlocked.
pub const IS_RESTRICTION_TEMPORARY_UNLOCKED: u32 = 1006;

/// Confirms stereo vision (VR mode) permission. [4.0.0+]
pub const CONFIRM_STEREO_VISION_PERMISSION: u32 = 1013;

/// Checks whether parental controls are enabled.
pub const IS_RESTRICTION_ENABLED: u32 = 1031;

/// Gets the current safety level.
pub const GET_SAFETY_LEVEL: u32 = 1032;

/// Gets the current restriction settings.
pub const GET_CURRENT_SETTINGS: u32 = 1035;

/// Gets the count of free-communication applications.
pub const GET_FREE_COMMUNICATION_APPLICATION_LIST_COUNT: u32 = 1039;

/// Resets the stereo vision permission confirmation. [5.0.0+]
pub const RESET_CONFIRMED_STEREO_VISION_PERMISSION: u32 = 1064;

/// Checks whether stereo vision is permitted. [5.0.0+]
pub const IS_STEREO_VISION_PERMITTED: u32 = 1065;

/// Checks whether pairing is active.
pub const IS_PAIRING_ACTIVE: u32 = 1403;

/// Gets the synchronization event.
pub const GET_SYNCHRONIZATION_EVENT: u32 = 1432;

/// Gets the play-timer suspension-request event.
pub const GET_PLAY_TIMER_EVENT_TO_REQUEST_SUSPENSION: u32 = 1457;

/// Checks whether the play-timer alarm is disabled. [4.0.0+]
pub const IS_PLAY_TIMER_ALARM_DISABLED: u32 = 1458;

/// Gets the unlinked event.
pub const GET_UNLINKED_EVENT: u32 = 1473;
