//! GRC game recording wire-layout types.

use core::mem::size_of;

use nx_service_caps::AlbumFileId;
use static_assertions::const_assert_eq;

/// Stream type for transfer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GrcStream {
    /// Video stream with H.264 NAL units.
    Video = 0,
    /// Audio stream (PCM Int16, 2 channels, 48 kHz).
    Audio = 1,
}

/// Game movie identifier.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct GameMovieId {
    pub file_id: AlbumFileId,
    pub reserved: [u8; 0x28],
}

const_assert_eq!(size_of::<GameMovieId>(), 0x40);

/// Offscreen recording parameter.
///
/// Callers construct this directly. libnx's `grcCreateOffscreenRecordingParameter`
/// uses hosversion-dependent defaults; per IC-4 the caller picks the appropriate
/// values.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct OffscreenRecordingParameter {
    pub unknown_x0: [u8; 0x10],
    pub unknown_x10: u32,
    pub video_bitrate: i32,
    pub video_width: i32,
    pub video_height: i32,
    pub video_framerate: i32,
    pub video_key_frame_interval: i32,
    pub audio_bitrate: i32,
    pub audio_samplerate: i32,
    pub audio_channel_count: i32,
    pub audio_sample_format: i32,
    pub video_image_orientation: i32,
    pub unknown_x3c: [u8; 0x44],
}

const_assert_eq!(size_of::<OffscreenRecordingParameter>(), 0x80);

/// Transfer result returned by `grc:d` Transfer (cmd 2).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct TransferResult {
    pub num_frames: u32,
    pub data_size: u32,
    pub start_timestamp: u64,
}

const_assert_eq!(size_of::<TransferResult>(), 0x10);

/// Input payload for IGameMovieTrimmer::BeginTrim (cmd 1).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct BeginTrimIn {
    pub start: i32,
    pub end: i32,
    pub id: GameMovieId,
}

const_assert_eq!(size_of::<BeginTrimIn>(), 0x48);

/// Input payload for IGameMovieTrimmer::SetThumbnailRgba (cmd 20).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetThumbnailIn {
    pub width: i32,
    pub height: i32,
}

const_assert_eq!(size_of::<SetThumbnailIn>(), 0x08);

/// Input payload for IMovieMaker::StartOffscreenRecording (cmd 24).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct StartRecordingIn {
    pub layer_handle: u64,
    pub param: OffscreenRecordingParameter,
}

const_assert_eq!(size_of::<StartRecordingIn>(), 0x88);

/// Input payload for IMovieMaker::CompleteOffscreenRecordingFinishEx0/Ex1
/// (cmds 25, 26).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct CompleteFinishIn {
    pub width: i32,
    pub height: i32,
    pub layer_handle: u64,
}

const_assert_eq!(size_of::<CompleteFinishIn>(), 0x10);
