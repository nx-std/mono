//! Screenshots: the attributes an image is saved with, the options it is
//! decoded with, and what a decode returns.

// Scoped to the module rather than to `ScreenShotDecoderFlag`, because the code that trips the
// lint is emitted by `#[modular_bitfield::bitfield]`, which does not carry an item-level
// attribute through into its expansion.
#![expect(
    clippy::identity_op,
    reason = "modular_bitfield computes every field's shift the same way, so the first field's is `<< 0`"
)]

use static_assertions::const_assert_eq;

/// Image orientation for album screenshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlbumImageOrientation {
    Unknown0 = 0,
    Unknown1 = 1,
    Unknown2 = 2,
    Unknown3 = 3,
}

/// Screenshot attributes for album save operations.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct ScreenShotAttribute {
    /// Unknown word at offset 0x0.
    pub unk_x0: u32,
    /// Image orientation, an [`AlbumImageOrientation`] discriminant.
    pub orientation: u32,
    /// Unknown word at offset 0x8.
    pub unk_x8: u32,
    /// Unknown word at offset 0xc.
    pub unk_xc: u32,
    /// Unknown bytes at offset 0x10.
    pub unk_x10: [u8; 0x30],
}

const_assert_eq!(size_of::<ScreenShotAttribute>(), 0x40);

impl ScreenShotAttribute {
    /// Creates a default attribute with the given orientation and `unk_xc = 1`.
    pub fn with_orientation(orientation: AlbumImageOrientation) -> Self {
        Self {
            unk_x0: 0,
            orientation: orientation as u32,
            unk_x8: 0,
            unk_xc: 1,
            unk_x10: [0; 0x30],
        }
    }
}

/// Screenshot attributes for application use.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct ScreenShotAttributeForApplication {
    /// Unknown word at offset 0x0.
    pub unk_x0: u32,
    /// Unknown byte at offset 0x4.
    pub unk_x4: u8,
    /// Unknown byte at offset 0x5.
    pub unk_x5: u8,
    /// Unknown byte at offset 0x6.
    pub unk_x6: u8,
    /// Padding the wire form carries at offset 0x7.
    pub pad: u8,
    /// Unknown word at offset 0x8.
    pub unk_x8: u32,
    /// Unknown word at offset 0xc.
    pub unk_xc: u32,
    /// Unknown word at offset 0x10.
    pub unk_x10: u32,
    /// Unknown word at offset 0x14.
    pub unk_x14: u32,
    /// Unknown word at offset 0x18.
    pub unk_x18: u32,
    /// Unknown word at offset 0x1c.
    pub unk_x1c: u32,
    /// Unknown half-word at offset 0x20.
    pub unk_x20: u16,
    /// Unknown half-word at offset 0x22.
    pub unk_x22: u16,
    /// Unknown half-word at offset 0x24.
    pub unk_x24: u16,
    /// Unknown half-word at offset 0x26.
    pub unk_x26: u16,
    /// Reserved bytes at offset 0x28.
    pub reserved: [u8; 0x18],
}

const_assert_eq!(size_of::<ScreenShotAttributeForApplication>(), 0x40);

/// Flags controlling JPEG decode behaviour.
///
/// A bitfield rather than a `bitflags` set, so it derives its own wire
/// encoding: `bitflags` keeps its bits behind a generated inner type that
/// implements no `zerocopy` trait, which forces every payload carrying these
/// flags to encode itself by hand.
#[modular_bitfield::bitfield]
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
    __: modular_bitfield::specifiers::B62,
}

const_assert_eq!(size_of::<ScreenShotDecoderFlag>(), 0x8);

/// Decode options passed to JPEG decode/shrink commands.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ScreenShotDecodeOption {
    /// Flags controlling how the decoder treats the image.
    pub flags: ScreenShotDecoderFlag,
    /// Reserved words the wire form carries after the flags. Private: a caller
    /// has nothing to put in them, and [`new`](Self::new) zeroes them.
    _reserved: [u64; 3],
}

const_assert_eq!(size_of::<ScreenShotDecodeOption>(), 0x20);

impl ScreenShotDecodeOption {
    /// Creates decode options carrying `flags`.
    pub fn new(flags: ScreenShotDecoderFlag) -> Self {
        Self {
            flags,
            _reserved: [0; 3],
        }
    }
}

/// Application user data attached to a screenshot.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct ApplicationData {
    /// Buffer the application's own data is written into.
    pub userdata: [u8; 0x400],
    /// Number of bytes of `userdata` that are meaningful.
    pub size: u32,
}

const_assert_eq!(size_of::<ApplicationData>(), 0x404);

/// Output from loading an album screenshot image.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct LoadAlbumScreenShotImageOutput {
    /// Width of the decoded image, in pixels.
    pub width: i64,
    /// Height of the decoded image, in pixels.
    pub height: i64,
    /// Attributes the image was saved with.
    pub attr: ScreenShotAttribute,
    /// Unknown bytes at offset 0x50.
    pub unk_x50: [u8; 0x400],
}

const_assert_eq!(size_of::<LoadAlbumScreenShotImageOutput>(), 0x450);

/// Output from loading an album screenshot image for application use.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct LoadAlbumScreenShotImageOutputForApplication {
    /// Width of the decoded image, in pixels.
    pub width: i64,
    /// Height of the decoded image, in pixels.
    pub height: i64,
    /// Attributes the image was saved with.
    pub attr: ScreenShotAttributeForApplication,
    /// Application data attached when the image was saved.
    pub appdata: ApplicationData,
    /// Reserved bytes trailing the output.
    pub reserved: [u8; 0xac],
}

const_assert_eq!(
    size_of::<LoadAlbumScreenShotImageOutputForApplication>(),
    0x500
);
