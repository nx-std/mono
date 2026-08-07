//! Audio input wire-layout types.

use static_assertions::const_assert_eq;

/// Audio input buffer descriptor.
///
/// Describes a sample buffer's layout for audio capture.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct AudioInBuffer {
    /// Client-side pointer to the next buffer (linked list, unused).
    pub next_buffer_ptr: u64,
    /// Client-side pointer to the sample data (aligned to 0x1000).
    pub sample_buffer_ptr: u64,
    /// Total capacity of the sample buffer in bytes (aligned to 0x1000).
    pub sample_buffer_capacity: u64,
    /// Size of captured data in bytes.
    pub data_size: u64,
    /// Offset into the sample buffer where data begins.
    pub data_offset: u64,
}

const_assert_eq!(size_of::<AudioInBuffer>(), 0x28);

/// Wire-layout input for `OpenAudioIn`:
/// `{ u32 sample_rate, u32 channel_count, u64 client_pid }`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct OpenAudioInIn {
    pub sample_rate: u32,
    pub channel_count: u32,
    pub client_pid: u64,
}

const_assert_eq!(size_of::<OpenAudioInIn>(), 0x10);

/// Output parameters returned when opening an audio input device.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct OpenAudioInOut {
    /// Actual sample rate in Hz.
    pub sample_rate: u32,
    /// Actual number of audio channels.
    pub channel_count: u32,
    /// PCM sample format.
    pub pcm_format: u32,
    /// Initial device state.
    pub state: u32,
}

const_assert_eq!(size_of::<OpenAudioInOut>(), 0x10);

/// Audio input device state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioInState {
    /// The audio input is actively capturing.
    Started = 0,
    /// The audio input is stopped.
    Stopped = 1,
}
