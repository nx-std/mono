//! Mii image wire-layout types.

use static_assertions::const_assert_eq;

/// Image identifier, wrapping a UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct MiiimgImageId {
    /// Raw UUID bytes.
    pub uuid: [u8; 16],
}

const_assert_eq!(size_of::<MiiimgImageId>(), 0x10);

/// Mii create identifier, wrapping a UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct MiiCreateId {
    /// Raw UUID bytes.
    pub uuid: [u8; 16],
}

const_assert_eq!(size_of::<MiiCreateId>(), 0x10);

/// Image attribute returned by [`MiiimgService::get_attribute`](crate::MiiimgService::get_attribute).
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MiiimgImageAttribute {
    /// Image ID.
    pub image_id: MiiimgImageId,
    /// Mii's create ID.
    pub create_id: MiiCreateId,
    /// Unknown field.
    pub unk: u32,
    /// Mii name in UTF-16BE, null-terminated.
    pub mii_name: [u16; 11],
}

const_assert_eq!(size_of::<MiiimgImageAttribute>(), 0x3A);
