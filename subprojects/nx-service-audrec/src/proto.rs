//! Audio recorder service protocol constants.

use nx_sf::ServiceName;

/// Service name for the audio recorder user interface (`audrec:u`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("audrec:u");

// Root service commands (IFinalOutputRecorderManager)

/// Opens a final output recorder sub-object.
pub const OPEN_FINAL_OUTPUT_RECORDER: u32 = 0;

// Recorder sub-object commands (IFinalOutputRecorder)

/// Starts recording.
pub const RECORDER_START: u32 = 1;

/// Stops recording.
pub const RECORDER_STOP: u32 = 2;

/// Registers the buffer event (copy handle output).
pub const RECORDER_REGISTER_BUFFER_EVENT: u32 = 4;

/// Appends a final output recorder buffer (legacy, map-alias). [1.0.0-2.x.x]
pub const RECORDER_APPEND_BUFFER_LEGACY: u32 = 3;

/// Gets released final output recorder buffers (legacy, map-alias). [1.0.0-2.x.x]
pub const RECORDER_GET_RELEASED_BUFFERS_LEGACY: u32 = 5;

/// Appends a final output recorder buffer (auto-select). [3.0.0+]
pub const RECORDER_APPEND_BUFFER: u32 = 8;

/// Gets released final output recorder buffers (auto-select). [3.0.0+]
pub const RECORDER_GET_RELEASED_BUFFERS: u32 = 9;
