//! Audio output wire-layout types.

use static_assertions::const_assert_eq;

/// Audio output buffer descriptor.
///
/// Describes a sample buffer's layout for audio playback.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct AudioOutBuffer {
    /// Client-side pointer to the next buffer (linked list, unused).
    pub next_buffer_ptr: u64,
    /// Client-side pointer to the sample data (aligned to 0x1000).
    pub sample_buffer_ptr: u64,
    /// Total capacity of the sample buffer in bytes (aligned to 0x1000).
    pub sample_buffer_capacity: u64,
    /// Size of sample data in bytes.
    pub data_size: u64,
    /// Offset into the sample buffer where data begins.
    pub data_offset: u64,
}

const_assert_eq!(size_of::<AudioOutBuffer>(), 0x28);

/// Wire-layout input for `OpenAudioOut`:
/// `{ u32 sample_rate, u32 channel_count, u64 applet_resource_user_id }`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenAudioOutIn {
    pub sample_rate: u32,
    pub channel_count: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<OpenAudioOutIn>(), 0x10);

/// Output parameters returned when opening an audio output device.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct OpenAudioOutOut {
    /// Actual sample rate in Hz.
    pub sample_rate: u32,
    /// Actual number of audio channels.
    pub channel_count: u32,
    /// PCM sample format.
    pub pcm_format: u32,
    /// Initial device state.
    pub state: u32,
}

const_assert_eq!(size_of::<OpenAudioOutOut>(), 0x10);

/// Audio output device state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioOutState {
    /// The audio output is actively playing.
    Started = 0,
    /// The audio output is stopped.
    Stopped = 1,
}

/// Wire-layout input for audout:a suspend/resume commands:
/// `{ u64 pid, u64 delay }`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct PidDelayIn {
    pub pid: u64,
    pub delay: u64,
}

const_assert_eq!(size_of::<PidDelayIn>(), 0x10);

/// Wire-layout input for audout:a set-volume commands:
/// `{ f32 volume, pad[4], u64 pid, u64 delay }`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetVolumeIn {
    pub volume: f32,
    pub _pad: [u8; 4],
    pub pid: u64,
    pub delay: u64,
}

const_assert_eq!(size_of::<SetVolumeIn>(), 0x18);
