//! Audio input service protocol constants.

use nx_sf::ServiceName;

/// Service name for the audio input user interface (`audin:u`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("audin:u");

/// Device name buffer length (0x100 bytes per device).
pub const DEVICE_NAME_LENGTH: usize = 0x100;

// Root service commands (IAudioInManager)

/// Lists available audio input devices (legacy, map-alias). \[1.0.0-2.x.x\]
pub const LIST_AUDIO_INS_LEGACY: u32 = 0;

/// Opens an audio input device (legacy, map-alias). \[1.0.0-2.x.x\]
pub const OPEN_AUDIO_IN_LEGACY: u32 = 1;

/// Lists available audio input devices (auto-select). \[3.0.0+\]
pub const LIST_AUDIO_INS: u32 = 2;

/// Opens an audio input device (auto-select). \[3.0.0+\]
pub const OPEN_AUDIO_IN: u32 = 3;

// Audio-in sub-object commands (IAudioIn)

/// Gets the current audio input state.
pub const AUDIO_IN_GET_STATE: u32 = 0;

/// Starts audio input capture.
pub const AUDIO_IN_START: u32 = 1;

/// Stops audio input capture.
pub const AUDIO_IN_STOP: u32 = 2;

/// Appends an audio input buffer (legacy, map-alias). \[1.0.0-2.x.x\]
pub const AUDIO_IN_APPEND_BUFFER_LEGACY: u32 = 3;

/// Registers the buffer event (copy handle output).
pub const AUDIO_IN_REGISTER_BUFFER_EVENT: u32 = 4;

/// Gets released audio input buffers (legacy, map-alias). \[1.0.0-2.x.x\]
pub const AUDIO_IN_GET_RELEASED_BUFFER_LEGACY: u32 = 5;

/// Checks whether a buffer is contained in the audio input.
pub const AUDIO_IN_CONTAINS_BUFFER: u32 = 6;

/// Appends an audio input buffer (auto-select). \[3.0.0+\]
pub const AUDIO_IN_APPEND_BUFFER: u32 = 8;

/// Gets released audio input buffers (auto-select). \[3.0.0+\]
pub const AUDIO_IN_GET_RELEASED_BUFFER: u32 = 9;
