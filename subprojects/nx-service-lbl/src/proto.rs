//! Backlight (`lbl`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the backlight service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("lbl");

/// Save current backlight settings.
pub const SAVE_CURRENT_SETTING: u32 = 0;

/// Load current backlight settings.
pub const LOAD_CURRENT_SETTING: u32 = 1;

/// Set current brightness setting (0.0–1.0).
pub const SET_CURRENT_BRIGHTNESS_SETTING: u32 = 2;

/// Get current brightness setting.
pub const GET_CURRENT_BRIGHTNESS_SETTING: u32 = 3;

/// Apply current brightness setting to backlight hardware.
pub const APPLY_CURRENT_BRIGHTNESS_SETTING_TO_BACKLIGHT: u32 = 4;

/// Get brightness setting applied to backlight hardware.
pub const GET_BRIGHTNESS_SETTING_APPLIED_TO_BACKLIGHT: u32 = 5;

/// Switch backlight on with fade time (nanoseconds).
pub const SWITCH_BACKLIGHT_ON: u32 = 6;

/// Switch backlight off with fade time (nanoseconds).
pub const SWITCH_BACKLIGHT_OFF: u32 = 7;

/// Get backlight switch status.
pub const GET_BACKLIGHT_SWITCH_STATUS: u32 = 8;

/// Enable display dimming.
pub const ENABLE_DIMMING: u32 = 9;

/// Disable display dimming.
pub const DISABLE_DIMMING: u32 = 10;

/// Check if dimming is enabled.
pub const IS_DIMMING_ENABLED: u32 = 11;

/// Enable auto brightness control.
pub const ENABLE_AUTO_BRIGHTNESS_CONTROL: u32 = 12;

/// Disable auto brightness control.
pub const DISABLE_AUTO_BRIGHTNESS_CONTROL: u32 = 13;

/// Check if auto brightness control is enabled.
pub const IS_AUTO_BRIGHTNESS_CONTROL_ENABLED: u32 = 14;

/// Set ambient light sensor value.
pub const SET_AMBIENT_LIGHT_SENSOR_VALUE: u32 = 15;

/// Get ambient light sensor value.
pub const GET_AMBIENT_LIGHT_SENSOR_VALUE: u32 = 16;

/// Check if ambient light sensor is available (3.0.0+).
pub const IS_AMBIENT_LIGHT_SENSOR_AVAILABLE: u32 = 23;

/// Set current brightness setting for VR mode (3.0.0+).
pub const SET_CURRENT_BRIGHTNESS_SETTING_FOR_VR_MODE: u32 = 24;

/// Get current brightness setting for VR mode (3.0.0+).
pub const GET_CURRENT_BRIGHTNESS_SETTING_FOR_VR_MODE: u32 = 25;

/// Enable VR mode (3.0.0+).
pub const ENABLE_VR_MODE: u32 = 26;

/// Disable VR mode (3.0.0+).
pub const DISABLE_VR_MODE: u32 = 27;

/// Check if VR mode is enabled (3.0.0+).
pub const IS_VR_MODE_ENABLED: u32 = 28;
