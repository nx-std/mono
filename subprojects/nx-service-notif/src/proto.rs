//! Notification service protocol constants.

use nx_sf::ServiceName;

/// Application notification service name (`notif:a`).
pub const SERVICE_NAME_A: ServiceName = ServiceName::new_truncate("notif:a");

/// System notification service name (`notif:s`).
pub const SERVICE_NAME_S: ServiceName = ServiceName::new_truncate("notif:s");

// INotificationService commands

/// Registers an alarm setting (returns alarm_setting_id).
pub const REGISTER_ALARM_SETTING: u32 = 500;

/// Updates an existing alarm setting.
pub const UPDATE_ALARM_SETTING: u32 = 510;

/// Lists all registered alarm settings.
pub const LIST_ALARM_SETTINGS: u32 = 520;

/// Loads the application parameter for a given alarm setting.
pub const LOAD_APPLICATION_PARAMETER: u32 = 530;

/// Deletes an alarm setting by ID.
pub const DELETE_ALARM_SETTING: u32 = 540;

/// Initializes the Application variant (sends PID). Called during connect for `notif:a`.
pub const INITIALIZE: u32 = 1000;
