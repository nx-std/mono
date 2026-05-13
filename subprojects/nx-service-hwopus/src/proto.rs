//! Hardware Opus service protocol constants.

use nx_sf::ServiceName;

/// Service name for the hardware Opus manager (`hwopus`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("hwopus");

// IHardwareOpusDecoderManager commands

/// Opens a hardware Opus decoder for single-stream playback.
pub const OPEN_HARDWARE_OPUS_DECODER: u32 = 0;

/// Gets the required work buffer size for single-stream decoding.
pub const GET_WORK_BUFFER_SIZE: u32 = 1;

/// Opens a hardware Opus decoder for multi-stream playback. [3.0.0+]
pub const OPEN_HARDWARE_OPUS_DECODER_FOR_MULTI_STREAM: u32 = 2;

/// Gets the required work buffer size for multi-stream decoding. [3.0.0+]
pub const GET_WORK_BUFFER_SIZE_FOR_MULTI_STREAM: u32 = 3;

// IHardwareOpusDecoder commands (single-stream)

/// Decodes interleaved Opus data (pre-4.0.0, single-stream).
pub const DECODE_INTERLEAVED: u32 = 0;

/// Decodes interleaved Opus data with perf output (4.0.0+, single-stream).
pub const DECODE_INTERLEAVED_WITH_PERF: u32 = 4;

/// Decodes interleaved Opus data with perf and reset (6.0.0+, single-stream).
pub const DECODE_INTERLEAVED_EX: u32 = 6;

// IHardwareOpusDecoder commands (multi-stream)

/// Decodes interleaved Opus data (pre-4.0.0, multi-stream).
pub const DECODE_INTERLEAVED_MULTI_STREAM: u32 = 2;

/// Decodes interleaved Opus data with perf output (4.0.0+, multi-stream).
pub const DECODE_INTERLEAVED_WITH_PERF_MULTI_STREAM: u32 = 5;

/// Decodes interleaved Opus data with perf and reset (6.0.0+, multi-stream).
pub const DECODE_INTERLEAVED_EX_MULTI_STREAM: u32 = 7;
