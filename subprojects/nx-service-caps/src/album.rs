//! Album files: how the album identifies, classifies and accounts for what it
//! stores.

use static_assertions::const_assert_eq;

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

/// Controls whether the screenshot-taken overlay notification is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlbumReportOption {
    Disable = 0,
    Enable = 1,
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

/// Album file date-time. Corresponds to each field in the album entry
/// filename prior to the "-": "YYYYMMDDHHMMSSII".
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct AlbumFileDateTime {
    /// Four-digit year.
    pub year: u16,
    /// Month, 1-12.
    pub month: u8,
    /// Day of the month, 1-31.
    pub day: u8,
    /// Hour, 0-23.
    pub hour: u8,
    /// Minute, 0-59.
    pub minute: u8,
    /// Second, 0-59.
    pub second: u8,
    /// Discriminator for files sharing the same second.
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
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct AlbumFileId {
    /// Application the file was captured by.
    pub application_id: u64,
    /// When the file was captured.
    pub datetime: AlbumFileDateTime,
    /// Storage the file lives on, an [`AlbumStorage`] discriminant.
    pub storage: u8,
    /// What the file holds, an [`AlbumFileContents`] discriminant.
    pub content: u8,
    /// Unknown byte at offset 0x12.
    pub unknown_12: u8,
    /// Unknown byte at offset 0x13.
    pub unknown_13: u8,
    /// Padding the wire form carries at offset 0x14.
    pub pad_x14: [u8; 4],
}

const_assert_eq!(size_of::<AlbumFileId>(), 0x18);

/// Album entry with file size and identifier.
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
pub struct AlbumEntry {
    /// Size of the file, in bytes.
    pub size: u64,
    /// Identifier the file is addressed by.
    pub file_id: AlbumFileId,
}

const_assert_eq!(size_of::<AlbumEntry>(), 0x20);

/// Opaque application album entry returned by save operations.
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
pub struct ApplicationAlbumEntry {
    /// Opaque entry bytes; only the service interprets them.
    pub data: [u8; 0x20],
}

const_assert_eq!(size_of::<ApplicationAlbumEntry>(), 0x20);

/// Application album file entry.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct ApplicationAlbumFileEntry {
    /// Opaque entry the file is addressed by.
    pub entry: ApplicationAlbumEntry,
    /// When the file was captured.
    pub datetime: AlbumFileDateTime,
    /// Unknown word at offset 0x28.
    pub unk_x28: u64,
}

const_assert_eq!(size_of::<ApplicationAlbumFileEntry>(), 0x30);

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

/// Album contents usage statistics.
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
pub struct AlbumContentsUsage {
    /// Number of files of this content type.
    pub count: i64,
    /// Total size of those files, in bytes.
    pub size: i64,
    /// [`AlbumContentsUsageFlag`] bits qualifying the counts above.
    pub flags: u32,
    /// Content type counted, an [`AlbumFileContents`] discriminant.
    pub file_contents: u8,
    /// Padding the wire form carries at offset 0x15.
    pub pad_x15: [u8; 3],
}

const_assert_eq!(size_of::<AlbumContentsUsage>(), 0x18);

/// Album usage with 2 content-type slots.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct AlbumUsage2 {
    /// Per-content-type usage, one slot each.
    pub usages: [AlbumContentsUsage; 2],
}

const_assert_eq!(size_of::<AlbumUsage2>(), 0x30);

/// Album usage with 3 content-type slots.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct AlbumUsage3 {
    /// Per-content-type usage, one slot each.
    pub usages: [AlbumContentsUsage; 3],
}

const_assert_eq!(size_of::<AlbumUsage3>(), 0x48);

/// Album usage with 16 content-type slots.
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
pub struct AlbumUsage16 {
    /// Per-content-type usage, one slot each.
    pub usages: [AlbumContentsUsage; 16],
}

const_assert_eq!(size_of::<AlbumUsage16>(), 0x180);

/// Album cache information.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct AlbumCache {
    /// Number of cached entries.
    pub count: u64,
    /// Unknown word at offset 0x8.
    pub unk_x8: u64,
}

const_assert_eq!(size_of::<AlbumCache>(), 0x10);
