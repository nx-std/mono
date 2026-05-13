//! Audio service protocol constants.

use nx_sf::ServiceName;

/// Service name for the `aud:a` (admin) interface.
pub const AUDA_SERVICE_NAME: ServiceName = ServiceName::new_truncate("aud:a");

/// Service name for the `aud:d` (debug) interface.
pub const AUDD_SERVICE_NAME: ServiceName = ServiceName::new_truncate("aud:d");

// IAudioSystemManagerForApplet (aud:a) commands

/// Suspends audio for a process. [11.0.0+]
pub const REQUEST_SUSPEND_AUDIO: u32 = 2;

/// Resumes audio for a process. [11.0.0+]
pub const REQUEST_RESUME_AUDIO: u32 = 3;

/// Gets the master volume for a process's audio output. [11.0.0+]
pub const GET_AUDIO_OUTPUT_PROCESS_MASTER_VOLUME: u32 = 4;

/// Sets the master volume for a process's audio output. [11.0.0+]
pub const SET_AUDIO_OUTPUT_PROCESS_MASTER_VOLUME: u32 = 5;

/// Gets the master volume for a process's audio input. [11.0.0+]
pub const GET_AUDIO_INPUT_PROCESS_MASTER_VOLUME: u32 = 6;

/// Sets the master volume for a process's audio input and output. [11.0.0+]
pub const SET_AUDIO_INPUT_PROCESS_MASTER_VOLUME: u32 = 7;

/// Gets the record volume for a process's audio output. [11.0.0+]
pub const GET_AUDIO_OUTPUT_PROCESS_RECORD_VOLUME: u32 = 8;

/// Sets the record volume for a process's audio output. [11.0.0+]
pub const SET_AUDIO_OUTPUT_PROCESS_RECORD_VOLUME: u32 = 9;

// IAudioSystemManagerForDebugger (aud:d) commands

/// Suspends audio for a process (debug). [11.0.0+]
pub const REQUEST_SUSPEND_AUDIO_FOR_DEBUG: u32 = 0;

/// Resumes audio for a process (debug). [11.0.0+]
pub const REQUEST_RESUME_AUDIO_FOR_DEBUG: u32 = 1;
