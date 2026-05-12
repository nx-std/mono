//! IAudioDevice protocol constants.

use nx_sf::ServiceName;

/// Service name for the audio renderer manager (`audren:u`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("audren:u");

// IAudioRendererManager commands

/// Opens an IAudioDevice for the given applet resource user ID.
pub const GET_AUDIO_DEVICE_SERVICE: u32 = 2;

// IAudioDevice commands (pre-3.0.0)

/// Lists audio device names (pre-3.0.0, mapped buffers).
pub const LIST_AUDIO_DEVICE_NAME_OLD: u32 = 0;

/// Sets the output volume for a named device (pre-3.0.0, mapped buffers).
pub const SET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD: u32 = 1;

/// Gets the output volume for a named device (pre-3.0.0, mapped buffers).
pub const GET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD: u32 = 2;

/// Gets the active audio device name (pre-3.0.0, mapped buffers).
pub const GET_ACTIVE_AUDIO_DEVICE_NAME_OLD: u32 = 3;

// IAudioDevice commands (3.0.0+)

/// Lists audio device names (3.0.0+, auto-select buffers).
pub const LIST_AUDIO_DEVICE_NAME: u32 = 6;

/// Sets the output volume for a named device (3.0.0+, auto-select buffers).
pub const SET_AUDIO_DEVICE_OUTPUT_VOLUME: u32 = 7;

/// Gets the output volume for a named device (3.0.0+, auto-select buffers).
pub const GET_AUDIO_DEVICE_OUTPUT_VOLUME: u32 = 8;

/// Gets the active audio device name (3.0.0+, auto-select buffers).
pub const GET_ACTIVE_AUDIO_DEVICE_NAME: u32 = 10;
