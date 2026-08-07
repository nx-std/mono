//! Mii service wire-layout types.

use bitflags::bitflags;
use static_assertions::const_assert_eq;

/// Mii age filter for random generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MiiAge {
    Young = 0,
    Normal = 1,
    Old = 2,
    All = 3,
}

/// Mii gender filter for random generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MiiGender {
    Male = 0,
    Female = 1,
    All = 2,
}

/// Mii face color filter for random generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MiiFaceColor {
    Black = 0,
    White = 1,
    Asian = 2,
    All = 3,
}

/// Mii special key code for database access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MiiSpecialKeyCode {
    Normal = 0,
    Special = 0xA523_B78F,
}

bitflags! {
    /// Source flag for filtering Miis in database queries.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MiiSourceFlag: u32 {
        /// Miis created by the user.
        const DATABASE = 1 << 0;
        /// Default console Miis.
        const DEFAULT = 1 << 1;
    }
}

/// Mii create ID (UUID).
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
pub struct MiiCreateId {
    pub uuid: [u8; 0x10],
}

const_assert_eq!(size_of::<MiiCreateId>(), 0x10);

/// Mii character info data structure.
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
pub struct MiiCharInfo {
    pub create_id: MiiCreateId,
    pub mii_name: [u16; 11],
    pub unk_x26: u8,
    pub mii_color: u8,
    pub mii_sex: u8,
    pub mii_height: u8,
    pub mii_width: u8,
    pub unk_x2b: [u8; 2],
    pub mii_face_shape: u8,
    pub mii_face_color: u8,
    pub mii_wrinkles_style: u8,
    pub mii_makeup_style: u8,
    pub mii_hair_style: u8,
    pub mii_hair_color: u8,
    pub mii_has_hair_flipped: u8,
    pub mii_eye_style: u8,
    pub mii_eye_color: u8,
    pub mii_eye_size: u8,
    pub mii_eye_thickness: u8,
    pub mii_eye_angle: u8,
    pub mii_eye_pos_x: u8,
    pub mii_eye_pos_y: u8,
    pub mii_eyebrow_style: u8,
    pub mii_eyebrow_color: u8,
    pub mii_eyebrow_size: u8,
    pub mii_eyebrow_thickness: u8,
    pub mii_eyebrow_angle: u8,
    pub mii_eyebrow_pos_x: u8,
    pub mii_eyebrow_pos_y: u8,
    pub mii_nose_style: u8,
    pub mii_nose_size: u8,
    pub mii_nose_pos: u8,
    pub mii_mouth_style: u8,
    pub mii_mouth_color: u8,
    pub mii_mouth_size: u8,
    pub mii_mouth_thickness: u8,
    pub mii_mouth_pos: u8,
    pub mii_facial_hair_color: u8,
    pub mii_beard_style: u8,
    pub mii_mustache_style: u8,
    pub mii_mustache_size: u8,
    pub mii_mustache_pos: u8,
    pub mii_glasses_style: u8,
    pub mii_glasses_color: u8,
    pub mii_glasses_size: u8,
    pub mii_glasses_pos: u8,
    pub mii_has_mole: u8,
    pub mii_mole_size: u8,
    pub mii_mole_pos_x: u8,
    pub mii_mole_pos_y: u8,
    pub unk_x57: u8,
}

const_assert_eq!(size_of::<MiiCharInfo>(), 0x58);

/// Mii store data.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MiiStoreData {
    pub data: [u8; 0x44],
}

const_assert_eq!(size_of::<MiiStoreData>(), 0x44);

/// Mii format used in 3DS.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MiiVer3StoreData {
    pub data: [u8; 0x5C],
}

const_assert_eq!(size_of::<MiiVer3StoreData>(), 0x5C);

/// Original Mii colors and types before Ver3StoreData conversion.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MiiNfpStoreDataExtension {
    pub faceline_color: u8,
    pub hair_color: u8,
    pub eye_color: u8,
    pub eyebrow_color: u8,
    pub mouth_color: u8,
    pub beard_color: u8,
    pub glass_color: u8,
    pub glass_type: u8,
}

const_assert_eq!(size_of::<MiiNfpStoreDataExtension>(), 0x08);

/// Wire-layout input for `BuildRandom`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct BuildRandomIn {
    pub age: u32,
    pub gender: u32,
    pub face_color: u32,
}

const_assert_eq!(size_of::<BuildRandomIn>(), 0x0C);
