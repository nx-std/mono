//! Hardware Opus wire-layout types.

use static_assertions::const_assert_eq;

/// Opus packet header prepended to the opus input data.
///
/// All fields are big-endian.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct HwopusHeader {
    pub size: u32,
    pub final_range: u32,
}

const_assert_eq!(size_of::<HwopusHeader>(), 0x8);

/// Multi-stream decoder state passed to the manager as an HipcPointer buffer.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct HwopusMultistreamState {
    pub sample_rate: i32,
    pub channel_count: i32,
    pub total_stream_count: i32,
    pub stereo_stream_count: i32,
    pub channel_mapping: [u8; 256],
}

const_assert_eq!(size_of::<HwopusMultistreamState>(), 0x110);

/// Input payload for `OpenHardwareOpusDecoder` (cmd 0).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenDecoderIn {
    pub val: u64,
    pub size: u32,
}

const_assert_eq!(size_of::<OpenDecoderIn>(), 0x10);

/// Output from decode commands (pre-4.0.0).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DecodeResult {
    pub decoded_data_size: i32,
    pub decoded_sample_count: i32,
}

const_assert_eq!(size_of::<DecodeResult>(), 0x8);

/// Output from decode commands (4.0.0+) that include performance data.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DecodeResultWithPerf {
    pub decoded_data_size: i32,
    pub decoded_sample_count: i32,
    pub perf: u64,
}

const_assert_eq!(size_of::<DecodeResultWithPerf>(), 0x10);
