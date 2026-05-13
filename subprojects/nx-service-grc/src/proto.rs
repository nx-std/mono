//! GRC game recording service protocol constants.

use nx_sf::ServiceName;

/// Service name for the game recording debug service (`grc:d`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("grc:d");

// ---------------------------------------------------------------------------
// grc:d commands
// ---------------------------------------------------------------------------

/// Begins streaming. Must not be called more than once per sysmodule instance.
pub const BEGIN: u32 = 1;

/// Retrieves stream data from the continuous recorder.
pub const TRANSFER: u32 = 2;

// ---------------------------------------------------------------------------
// IGameMovieTrimmer commands
// ---------------------------------------------------------------------------

/// Begins trimming a game movie.
pub const TRIMMER_BEGIN_TRIM: u32 = 1;

/// Ends trimming and retrieves the output movie ID.
pub const TRIMMER_END_TRIM: u32 = 2;

/// Gets the "not trimming" event (copy handle, autoclear=false).
pub const TRIMMER_GET_NOT_TRIMMING_EVENT: u32 = 10;

/// Sets the thumbnail RGBA image for the trimmed movie.
pub const TRIMMER_SET_THUMBNAIL_RGBA: u32 = 20;

// ---------------------------------------------------------------------------
// IMovieMaker commands
// ---------------------------------------------------------------------------

/// Creates a video proxy sub-object (IHOSBinderDriver).
pub const MAKER_CREATE_VIDEO_PROXY: u32 = 2;

/// Sets the album shim library version. \[7.0.0+\]
pub const MAKER_SET_ALBUM_SHIM_LIBRARY_VERSION: u32 = 9;

/// Opens an offscreen layer. Returns a binder ID.
pub const MAKER_OPEN_OFFSCREEN_LAYER: u32 = 10;

/// Closes an offscreen layer.
pub const MAKER_CLOSE_OFFSCREEN_LAYER: u32 = 11;

/// Aborts offscreen recording.
pub const MAKER_ABORT_OFFSCREEN_RECORDING: u32 = 21;

/// Requests offscreen recording finish ready.
pub const MAKER_REQUEST_OFFSCREEN_RECORDING_FINISH_READY: u32 = 22;

/// Starts offscreen recording with the given parameters.
pub const MAKER_START_OFFSCREEN_RECORDING: u32 = 24;

/// Completes offscreen recording finish (pre-7.0.0).
pub const MAKER_COMPLETE_OFFSCREEN_RECORDING_FINISH_EX0: u32 = 25;

/// Completes offscreen recording finish (7.0.0+). Returns application album entry.
pub const MAKER_COMPLETE_OFFSCREEN_RECORDING_FINISH_EX1: u32 = 26;

/// Gets the offscreen layer error.
pub const MAKER_GET_OFFSCREEN_LAYER_ERROR: u32 = 30;

/// Encodes offscreen layer audio sample data.
pub const MAKER_ENCODE_OFFSCREEN_LAYER_AUDIO_SAMPLE: u32 = 41;

/// Gets the offscreen layer recording finish ready event (copy handle).
pub const MAKER_GET_OFFSCREEN_LAYER_RECORDING_FINISH_READY_EVENT: u32 = 50;

/// Gets the offscreen layer audio encode ready event (copy handle).
pub const MAKER_GET_OFFSCREEN_LAYER_AUDIO_ENCODE_READY_EVENT: u32 = 52;
