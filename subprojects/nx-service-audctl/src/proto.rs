//! Audio control (`audctl`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the audio control service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("audctl");

/// Get target volume.
pub const GET_TARGET_VOLUME: u32 = 0;

/// Set target volume.
pub const SET_TARGET_VOLUME: u32 = 1;

/// Get target volume minimum.
pub const GET_TARGET_VOLUME_MIN: u32 = 2;

/// Get target volume maximum.
pub const GET_TARGET_VOLUME_MAX: u32 = 3;

/// Check if target is muted.
pub const IS_TARGET_MUTE: u32 = 4;

/// Set target mute state.
pub const SET_TARGET_MUTE: u32 = 5;

/// Check if target is connected (pre-18.0.0).
pub const IS_TARGET_CONNECTED: u32 = 6;

/// Set default audio target with fade durations.
pub const SET_DEFAULT_TARGET: u32 = 7;

/// Get default audio target.
pub const GET_DEFAULT_TARGET: u32 = 8;

/// Get audio output mode for target.
pub const GET_AUDIO_OUTPUT_MODE: u32 = 9;

/// Set audio output mode for target.
pub const SET_AUDIO_OUTPUT_MODE: u32 = 10;

/// Set force mute policy (pre-14.0.0).
pub const SET_FORCE_MUTE_POLICY: u32 = 11;

/// Get force mute policy (pre-14.0.0).
pub const GET_FORCE_MUTE_POLICY: u32 = 12;

/// Get output mode setting for target.
pub const GET_OUTPUT_MODE_SETTING: u32 = 13;

/// Set output mode setting for target.
pub const SET_OUTPUT_MODE_SETTING: u32 = 14;

/// Set output target.
pub const SET_OUTPUT_TARGET: u32 = 15;

/// Set input target force enabled.
pub const SET_INPUT_TARGET_FORCE_ENABLED: u32 = 16;

/// Set headphone output level mode (3.0.0+).
pub const SET_HEADPHONE_OUTPUT_LEVEL_MODE: u32 = 17;

/// Get headphone output level mode (3.0.0+).
pub const GET_HEADPHONE_OUTPUT_LEVEL_MODE: u32 = 18;

/// Acquire audio volume update event for play report (3.0.0–13.2.1).
pub const ACQUIRE_AUDIO_VOLUME_UPDATE_EVENT_FOR_PLAY_REPORT: u32 = 19;

/// Acquire audio output device update event for play report (3.0.0–13.2.1).
pub const ACQUIRE_AUDIO_OUTPUT_DEVICE_UPDATE_EVENT_FOR_PLAY_REPORT: u32 = 20;

/// Get audio output target for play report (3.0.0+).
pub const GET_AUDIO_OUTPUT_TARGET_FOR_PLAY_REPORT: u32 = 21;

/// Notify headphone volume warning displayed event (3.0.0+).
pub const NOTIFY_HEADPHONE_VOLUME_WARNING_DISPLAYED_EVENT: u32 = 22;

/// Set system output master volume (4.0.0+).
pub const SET_SYSTEM_OUTPUT_MASTER_VOLUME: u32 = 23;

/// Get system output master volume (4.0.0+).
pub const GET_SYSTEM_OUTPUT_MASTER_VOLUME: u32 = 24;

/// Get active output target (13.0.0+).
pub const GET_ACTIVE_OUTPUT_TARGET: u32 = 32;
