//! Audio renderer service protocol constants.

use nx_sf::ServiceName;

/// Service name for the audio renderer manager (`audren:u`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("audren:u");

// ---------------------------------------------------------------------------
// IAudioRendererManager commands
// ---------------------------------------------------------------------------

/// Opens an audio renderer (returns IAudioRenderer sub-object). \[1.0.0+\]
pub const OPEN_AUDIO_RENDERER: u32 = 0;

/// Gets the required work buffer size for the given parameters. \[1.0.0+\]
pub const GET_WORK_BUFFER_SIZE: u32 = 1;

// ---------------------------------------------------------------------------
// IAudioRenderer commands
// ---------------------------------------------------------------------------

/// Gets the current renderer state. \[1.0.0+\]
pub const RENDERER_GET_STATE: u32 = 3;

/// Requests update of the audio renderer (map-alias, legacy). \[1.0.0-2.x.x\]
pub const RENDERER_REQUEST_UPDATE_LEGACY: u32 = 4;

/// Starts the audio renderer. \[1.0.0+\]
pub const RENDERER_START: u32 = 5;

/// Stops the audio renderer. \[1.0.0+\]
pub const RENDERER_STOP: u32 = 6;

/// Queries the system event (copy handle, autoclear). \[1.0.0+\]
pub const RENDERER_QUERY_SYSTEM_EVENT: u32 = 7;

/// Sets the rendering time limit as a percentage. \[1.0.0+\]
pub const RENDERER_SET_RENDERING_TIME_LIMIT: u32 = 8;

/// Requests update of the audio renderer (auto-select). \[3.0.0+\]
pub const RENDERER_REQUEST_UPDATE: u32 = 10;
