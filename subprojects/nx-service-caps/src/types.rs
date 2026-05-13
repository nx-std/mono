//! Shared capture-service wire-layout types.

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

/// Controls whether the screenshot-taken overlay notification is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlbumReportOption {
    Disable = 0,
    Enable = 1,
}

/// Album storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlbumStorage {
    Nand = 0,
    Sd = 1,
}

/// Content type for album entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContentType {
    Screenshot = 0,
    Movie = 1,
    ExtraMovie = 3,
}

/// Album file contents classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AlbumFileContents {
    ScreenShot = 0,
    Movie = 1,
    ExtraScreenShot = 2,
    ExtraMovie = 3,
}

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

bitflags::bitflags! {
    /// Flags for querying album contents by file type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct AlbumFileContentsFlag: u32 {
        /// Query for screenshot files.
        const SCREEN_SHOT = 1 << 0;
        /// Query for movie files.
        const MOVIE = 1 << 1;
    }
}

bitflags::bitflags! {
    /// Flags for album contents usage reporting.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct AlbumContentsUsageFlag: u32 {
        /// Additional files exist beyond the count/size fields.
        const HAS_GREATER_USAGE = 1 << 0;
        /// The file is not a known content type.
        const IS_UNKNOWN_CONTENTS = 1 << 1;
    }
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

/// Screenshot attributes for application use.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenShotAttributeForApplication {
    pub unk_x0: u32,
    pub unk_x4: u8,
    pub unk_x5: u8,
    pub unk_x6: u8,
    pub pad: u8,
    pub unk_x8: u32,
    pub unk_xc: u32,
    pub unk_x10: u32,
    pub unk_x14: u32,
    pub unk_x18: u32,
    pub unk_x1c: u32,
    pub unk_x20: u16,
    pub unk_x22: u16,
    pub unk_x24: u16,
    pub unk_x26: u16,
    pub reserved: [u8; 0x18],
}

const_assert_eq!(size_of::<ScreenShotAttributeForApplication>(), 0x40);

/// Decode options passed to JPEG decode/shrink commands.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ScreenShotDecodeOption {
    pub flags: ScreenShotDecoderFlag,
    pub reserved: [u64; 3],
}

const_assert_eq!(size_of::<ScreenShotDecodeOption>(), 0x20);

/// Album file date-time. Corresponds to each field in the album entry
/// filename prior to the "-": "YYYYMMDDHHMMSSII".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AlbumFileDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub id: u8,
}

const_assert_eq!(size_of::<AlbumFileDateTime>(), 0x8);

impl AlbumFileDateTime {
    /// Default start date-time (1970-01-01).
    pub const fn default_start() -> Self {
        Self {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            id: 0,
        }
    }

    /// Default end date-time (3000-01-01).
    pub const fn default_end() -> Self {
        Self {
            year: 3000,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            id: 0,
        }
    }
}

/// Album file identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AlbumFileId {
    pub application_id: u64,
    pub datetime: AlbumFileDateTime,
    pub storage: u8,
    pub content: u8,
    pub unknown_12: u8,
    pub unknown_13: u8,
    pub pad_x14: [u8; 4],
}

const_assert_eq!(size_of::<AlbumFileId>(), 0x18);

/// Album entry with file size and identifier.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AlbumEntry {
    pub size: u64,
    pub file_id: AlbumFileId,
}

const_assert_eq!(size_of::<AlbumEntry>(), 0x20);

/// Opaque application album entry returned by save operations.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ApplicationAlbumEntry {
    pub data: [u8; 0x20],
}

const_assert_eq!(size_of::<ApplicationAlbumEntry>(), 0x20);

/// Application album file entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ApplicationAlbumFileEntry {
    pub entry: ApplicationAlbumEntry,
    pub datetime: AlbumFileDateTime,
    pub unk_x28: u64,
}

const_assert_eq!(size_of::<ApplicationAlbumFileEntry>(), 0x30);

/// Application user data attached to a screenshot.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ApplicationData {
    pub userdata: [u8; 0x400],
    pub size: u32,
}

const_assert_eq!(size_of::<ApplicationData>(), 0x404);

/// Album contents usage statistics.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AlbumContentsUsage {
    pub count: i64,
    pub size: i64,
    pub flags: u32,
    pub file_contents: u8,
    pub pad_x15: [u8; 3],
}

const_assert_eq!(size_of::<AlbumContentsUsage>(), 0x18);

/// Album usage with 2 content-type slots.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AlbumUsage2 {
    pub usages: [AlbumContentsUsage; 2],
}

const_assert_eq!(size_of::<AlbumUsage2>(), 0x30);

/// Album usage with 3 content-type slots.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AlbumUsage3 {
    pub usages: [AlbumContentsUsage; 3],
}

const_assert_eq!(size_of::<AlbumUsage3>(), 0x48);

/// Album usage with 16 content-type slots.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AlbumUsage16 {
    pub usages: [AlbumContentsUsage; 16],
}

const_assert_eq!(size_of::<AlbumUsage16>(), 0x180);

/// List of user IDs attached to a screenshot.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct UserIdList {
    pub uids: [AccountUid; USER_LIST_SIZE],
    pub count: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<UserIdList>(), 0x88);

/// Output from loading an album screenshot image for application use.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct LoadAlbumScreenShotImageOutputForApplication {
    pub width: i64,
    pub height: i64,
    pub attr: ScreenShotAttributeForApplication,
    pub appdata: ApplicationData,
    pub reserved: [u8; 0xac],
}

const_assert_eq!(
    size_of::<LoadAlbumScreenShotImageOutputForApplication>(),
    0x500
);

/// Output from loading an album screenshot image.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct LoadAlbumScreenShotImageOutput {
    pub width: i64,
    pub height: i64,
    pub attr: ScreenShotAttribute,
    pub unk_x50: [u8; 0x400],
}

const_assert_eq!(size_of::<LoadAlbumScreenShotImageOutput>(), 0x450);

/// Album cache information.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AlbumCache {
    pub count: u64,
    pub unk_x8: u64,
}

const_assert_eq!(size_of::<AlbumCache>(), 0x10);
