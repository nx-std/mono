//! Audio output service protocol constants.

use nx_sf::ServiceName;

/// Service name for the audio output user interface (`audout:u`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("audout:u");

/// Service name for the audio output admin interface (`audout:a`).
/// Removed in \[11.0.0\] (replaced by `aud:a`).
pub const AUDOUTA_SERVICE_NAME: ServiceName = ServiceName::new_truncate("audout:a");

/// Service name for the audio output debug interface (`audout:d`).
/// Removed in \[11.0.0\] (replaced by `aud:d`).
pub const AUDOUTD_SERVICE_NAME: ServiceName = ServiceName::new_truncate("audout:d");

/// Device name buffer length (0x100 bytes per device).
pub const DEVICE_NAME_LENGTH: usize = 0x100;

// Root service commands (IAudioOutManager)

/// Lists available audio output devices (legacy, map-alias). \[1.0.0-2.x.x\]
pub const LIST_AUDIO_OUTS_LEGACY: u32 = 0;

/// Opens an audio output device (legacy, map-alias). \[1.0.0-2.x.x\]
pub const OPEN_AUDIO_OUT_LEGACY: u32 = 1;

/// Lists available audio output devices (auto-select). \[3.0.0+\]
pub const LIST_AUDIO_OUTS: u32 = 2;

/// Opens an audio output device (auto-select). \[3.0.0+\]
pub const OPEN_AUDIO_OUT: u32 = 3;

// Audio-out sub-object commands (IAudioOut)

/// Gets the current audio output state.
pub const AUDIO_OUT_GET_STATE: u32 = 0;

/// Starts audio output playback.
pub const AUDIO_OUT_START: u32 = 1;

/// Stops audio output playback.
pub const AUDIO_OUT_STOP: u32 = 2;

/// Appends an audio output buffer (legacy, map-alias). \[1.0.0-2.x.x\]
pub const AUDIO_OUT_APPEND_BUFFER_LEGACY: u32 = 3;

/// Registers the buffer event (copy handle output).
pub const AUDIO_OUT_REGISTER_BUFFER_EVENT: u32 = 4;

/// Gets released audio output buffers (legacy, map-alias). \[1.0.0-2.x.x\]
pub const AUDIO_OUT_GET_RELEASED_BUFFER_LEGACY: u32 = 5;

/// Checks whether a buffer is contained in the audio output.
pub const AUDIO_OUT_CONTAINS_BUFFER: u32 = 6;

/// Appends an audio output buffer (auto-select). \[3.0.0+\]
pub const AUDIO_OUT_APPEND_BUFFER: u32 = 7;

/// Gets released audio output buffers (auto-select). \[3.0.0+\]
pub const AUDIO_OUT_GET_RELEASED_BUFFER: u32 = 8;

/// Gets the number of queued audio output buffers. \[4.0.0+\]
pub const AUDIO_OUT_GET_BUFFER_COUNT: u32 = 9;

/// Gets the total number of played samples. \[4.0.0+\]
pub const AUDIO_OUT_GET_PLAYED_SAMPLE_COUNT: u32 = 10;

/// Flushes all queued audio output buffers. \[4.0.0+\]
pub const AUDIO_OUT_FLUSH_BUFFERS: u32 = 11;

/// Sets the audio output volume. \[6.0.0+\]
pub const AUDIO_OUT_SET_VOLUME: u32 = 12;

/// Gets the audio output volume. \[6.0.0+\]
pub const AUDIO_OUT_GET_VOLUME: u32 = 13;

// audout:a commands (pre-11.0.0)

/// Suspends audio output for a process. \[4.0.0+\]
pub const AUDOUTA_REQUEST_SUSPEND: u32 = 0;

/// Resumes audio output for a process. \[4.0.0+\]
pub const AUDOUTA_REQUEST_RESUME: u32 = 1;

/// Gets the master volume for a process.
pub const AUDOUTA_GET_PROCESS_MASTER_VOLUME: u32 = 2;

/// Sets the master volume for a process.
pub const AUDOUTA_SET_PROCESS_MASTER_VOLUME: u32 = 3;

/// Gets the record volume for a process. \[4.0.0+\]
pub const AUDOUTA_GET_PROCESS_RECORD_VOLUME: u32 = 4;

/// Sets the record volume for a process. \[4.0.0+\]
pub const AUDOUTA_SET_PROCESS_RECORD_VOLUME: u32 = 5;

// audout:d commands (pre-11.0.0)

/// Suspends audio output for a process (debug).
pub const AUDOUTD_REQUEST_SUSPEND_FOR_DEBUG: u32 = 0;

/// Resumes audio output for a process (debug).
pub const AUDOUTD_REQUEST_RESUME_FOR_DEBUG: u32 = 1;
