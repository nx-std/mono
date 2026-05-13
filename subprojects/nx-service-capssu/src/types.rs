//! Screenshot upload wire-layout types.

use static_assertions::const_assert_eq;

/// Maximum number of user IDs in a [`UserIdList`].
pub const USER_LIST_SIZE: usize = 8;

/// Account user ID (128-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AccountUid {
    pub uid: [u64; 2],
}

const_assert_eq!(size_of::<AccountUid>(), 0x10);

/// Image orientation for album screenshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlbumImageOrientation {
    Unknown0 = 0,
    Unknown1 = 1,
    Unknown2 = 2,
    Unknown3 = 3,
}

/// Album report option (controls overlay notification).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlbumReportOption {
    Disable = 0,
    Enable = 1,
}

/// Screenshot attributes for album save operations.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenShotAttribute {
    pub unk_x0: u32,
    pub orientation: u32,
    pub unk_x8: u32,
    pub unk_xc: u32,
    pub unk_x10: [u8; 0x30],
}

const_assert_eq!(size_of::<ScreenShotAttribute>(), 0x40);

impl ScreenShotAttribute {
    /// Creates a default attribute with the given orientation and `unk_xc = 1`,
    /// matching the libnx convenience wrapper pattern.
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

/// Opaque application album entry returned by save operations.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ApplicationAlbumEntry {
    pub data: [u8; 0x20],
}

const_assert_eq!(size_of::<ApplicationAlbumEntry>(), 0x20);

/// Application user data attached to a screenshot.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationData {
    pub userdata: [u8; 0x400],
    pub size: u32,
}

const_assert_eq!(size_of::<ApplicationData>(), 0x404);

/// List of user IDs attached to a screenshot.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UserIdList {
    pub uids: [AccountUid; USER_LIST_SIZE],
    pub count: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<UserIdList>(), 0x88);

/// Wire-layout input for [`set_shim_library_version`](crate::cmif::set_shim_library_version) (cmd 32).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetShimVersionIn {
    pub version: u64,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SetShimVersionIn>(), 0x10);

/// Wire-layout input for save screenshot commands (cmds 203, 205, 210).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SaveScreenShotIn {
    pub attr: ScreenShotAttribute,
    pub report_option: u32,
    pub _pad: u32,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SaveScreenShotIn>(), 0x50);
