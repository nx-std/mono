//! JPEG decoder wire-layout types.

use static_assertions::const_assert_eq;

bitflags::bitflags! {
    /// Flags controlling JPEG decode behaviour.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct ScreenShotDecoderFlag: u64 {
        /// No special processing.
        const NONE = 0;
        /// See libjpeg-turbo `do_fancy_upsampling`.
        const ENABLE_FANCY_UPSAMPLING = 1 << 0;
        /// See libjpeg-turbo `do_block_smoothing`.
        const ENABLE_BLOCK_SMOOTHING = 1 << 1;
    }
}

/// Decode options passed to JPEG decode/shrink commands.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenShotDecodeOption {
    /// Bitflags controlling decoder behaviour.
    pub flags: ScreenShotDecoderFlag,
    /// Reserved for future use.
    pub reserved: [u64; 3],
}

const_assert_eq!(size_of::<ScreenShotDecodeOption>(), 0x20);

/// Wire-layout input for [`decode_jpeg`](crate::cmif::decode_jpeg) (cmd 3001)
/// and [`shrink_jpeg`](crate::cmif::shrink_jpeg) (cmd 4001).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DecodeJpegIn {
    pub width: u32,
    pub height: u32,
    pub opts: ScreenShotDecodeOption,
}

const_assert_eq!(size_of::<DecodeJpegIn>(), 0x28);

/// Wire-layout input for [`shrink_jpeg_ex`](crate::cmif::shrink_jpeg_ex) (cmd 4002).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ShrinkJpegExIn {
    pub scaled_width: u32,
    pub scaled_height: u32,
    pub jpeg_quality: u32,
    pub _pad: [u8; 4],
    pub opts: ScreenShotDecodeOption,
}

const_assert_eq!(size_of::<ShrinkJpegExIn>(), 0x30);
