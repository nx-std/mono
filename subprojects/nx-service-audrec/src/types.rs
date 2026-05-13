//! Audio recorder wire-layout types.

use static_assertions::const_assert_eq;

/// Buffer metadata for a final output recorder buffer.
///
/// Describes a sample buffer's layout and timing information.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct FinalOutputRecorderBuffer {
    /// Timestamp (in nanoseconds) when this buffer was released.
    pub released_ns: u64,
    /// Client-side pointer to the next buffer (linked list).
    pub next_buffer_ptr: u64,
    /// Client-side pointer to the sample data.
    pub sample_buffer_ptr: u64,
    /// Total capacity of the sample buffer in bytes.
    pub sample_buffer_capacity: u64,
    /// Size of the recorded data in bytes.
    pub data_size: u64,
    /// Offset into the sample buffer where data begins.
    pub data_offset: u64,
}

const_assert_eq!(size_of::<FinalOutputRecorderBuffer>(), 0x30);

/// Input parameters for opening a final output recorder.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct FinalOutputRecorderParameter {
    /// Sample rate in Hz (e.g. 48000).
    pub sample_rate: u32,
    /// Number of audio channels (e.g. 2 for stereo).
    pub channel_count: u32,
}

const_assert_eq!(size_of::<FinalOutputRecorderParameter>(), 0x08);

/// Output parameters returned when opening a final output recorder.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct FinalOutputRecorderParameterInternal {
    /// Actual sample rate in Hz.
    pub sample_rate: u32,
    /// Actual number of audio channels.
    pub channel_count: u32,
    /// Sample format (PCM encoding).
    pub sample_format: u32,
    /// Recorder state.
    pub state: u32,
}

const_assert_eq!(size_of::<FinalOutputRecorderParameterInternal>(), 0x10);

/// Wire-layout input for `OpenFinalOutputRecorder`:
/// `{ FinalOutputRecorderParameter param, u64 aruid }`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenRecorderIn {
    pub param: FinalOutputRecorderParameter,
    pub aruid: u64,
}

const_assert_eq!(size_of::<OpenRecorderIn>(), 0x10);

/// Wire-layout output for `GetReleasedFinalOutputRecorderBuffers`:
/// `{ u32 count, u32 _pad, u64 released }`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetReleasedBuffersOut {
    pub count: u32,
    pub _pad: u32,
    pub released: u64,
}

const_assert_eq!(size_of::<GetReleasedBuffersOut>(), 0x10);
