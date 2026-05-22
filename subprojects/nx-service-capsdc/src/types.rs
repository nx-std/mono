//! JPEG decoder wire-layout types.

#![expect(unused_parens, clippy::identity_op)]

use modular_bitfield::prelude::*;
use static_assertions::const_assert_eq;

/// Flags controlling JPEG decode behaviour.
#[bitfield]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct ScreenShotDecoderFlag {
    /// See libjpeg-turbo `do_fancy_upsampling`.
    pub enable_fancy_upsampling: bool,
    /// See libjpeg-turbo `do_block_smoothing`.
    pub enable_block_smoothing: bool,
    #[skip]
    __: B62,
}

/// Decode options passed to JPEG decode/shrink commands.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
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
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct DecodeJpegIn {
    pub width: u32,
    pub height: u32,
    pub opts: ScreenShotDecodeOption,
}

const_assert_eq!(size_of::<DecodeJpegIn>(), 0x28);

/// Wire-layout input for [`shrink_jpeg_ex`](crate::cmif::shrink_jpeg_ex) (cmd 4002).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ShrinkJpegExIn {
    pub scaled_width: u32,
    pub scaled_height: u32,
    pub jpeg_quality: u32,
    pub _pad: [u8; 4],
    pub opts: ScreenShotDecodeOption,
}

const_assert_eq!(size_of::<ShrinkJpegExIn>(), 0x30);
