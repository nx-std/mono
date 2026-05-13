//! Screenshot control wire-layout types.

use nx_service_vi::ViLayerStack;
use static_assertions::const_assert_eq;

/// Recommended JPEG output buffer size (512 KiB).
pub const JPEG_BUFFER_SIZE: usize = 0x80000;

/// Wire-layout input for [`capture_raw_image_with_timeout`](crate::cmif::capture_raw_image_with_timeout) (cmd 2).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CaptureRawImageIn {
    pub layer_stack: ViLayerStack,
    pub _pad: u32,
    pub width: u64,
    pub height: u64,
    pub buffer_count: i64,
    pub buffer_index: i64,
    pub timeout: i64,
}

const_assert_eq!(size_of::<CaptureRawImageIn>(), 0x30);

/// Wire-layout input for [`open_raw_screen_shot_read_stream`](crate::cmif::open_raw_screen_shot_read_stream) (cmd 1201)
/// and [`capture_jpeg_screen_shot`](crate::cmif::capture_jpeg_screen_shot) (cmd 1204).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LayerStackTimeoutIn {
    pub layer_stack: ViLayerStack,
    pub _pad: u32,
    pub timeout: i64,
}

const_assert_eq!(size_of::<LayerStackTimeoutIn>(), 0x10);

/// Wire-layout output for [`open_raw_screen_shot_read_stream`](crate::cmif::open_raw_screen_shot_read_stream) (cmd 1201).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenReadStreamOut {
    pub size: u64,
    pub width: u64,
    pub height: u64,
}

const_assert_eq!(size_of::<OpenReadStreamOut>(), 0x18);
