//! Album accessor wire-layout types.

use nx_service_caps::{
    AlbumFileId, ApplicationAlbumEntry, ScreenShotAttribute, ScreenShotDecodeOption,
};
use static_assertions::const_assert_eq;

/// Wire-layout input for [`storage_copy_album_file`](crate::cmif::storage_copy_album_file) (cmd 4).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StorageCopyAlbumFileIn {
    pub storage: u8,
    pub _pad: [u8; 7],
    pub file_id: AlbumFileId,
}

const_assert_eq!(size_of::<StorageCopyAlbumFileIn>(), 0x20);

/// Wire-layout output for screenshot load commands returning width/height (cmds 9, 10).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LoadScreenShotOut {
    pub width: u64,
    pub height: u64,
}

const_assert_eq!(size_of::<LoadScreenShotOut>(), 0x10);

/// Wire-layout input for [`get_album_entry_from_app_album_entry`](crate::cmif::get_album_entry_from_app_album_entry) (cmd 11).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAlbumEntryFromAppEntryIn {
    pub application_entry: ApplicationAlbumEntry,
    pub application_id: u64,
}

const_assert_eq!(size_of::<GetAlbumEntryFromAppEntryIn>(), 0x28);

/// Wire-layout input for screenshot commands with decode options (cmds 12, 13, 14, 1001).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LoadScreenShotExIn {
    pub file_id: AlbumFileId,
    pub opts: ScreenShotDecodeOption,
}

const_assert_eq!(size_of::<LoadScreenShotExIn>(), 0x38);

/// Wire-layout output for screenshot commands returning attributes + dimensions (cmds 14, 1001).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LoadScreenShotEx0Out {
    pub attr: ScreenShotAttribute,
    pub width: i64,
    pub height: i64,
}

const_assert_eq!(size_of::<LoadScreenShotEx0Out>(), 0x50);

/// Wire-layout input for storage+flags commands (cmds 17, 100, 101).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StorageFlagsIn {
    pub storage: u8,
    pub _pad1: [u8; 7],
    pub flags: u8,
    pub _pad2: [u8; 7],
}

const_assert_eq!(size_of::<StorageFlagsIn>(), 0x10);

/// Wire-layout output for [`get_min_max_applet_id`](crate::cmif::get_min_max_applet_id) (cmd 18).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetMinMaxAppletIdOut {
    pub success: u8,
    pub _pad: [u8; 3],
}

const_assert_eq!(size_of::<GetMinMaxAppletIdOut>(), 0x4);

/// Wire-layout output for overlay thumbnail commands (cmds 301, 302).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetLastOverlayThumbnailOut {
    pub file_id: AlbumFileId,
    pub size: u64,
}

const_assert_eq!(size_of::<GetLastOverlayThumbnailOut>(), 0x20);

/// Wire-layout input for [`get_required_storage_space_size_to_copy_all`](crate::cmif::get_required_storage_space_size_to_copy_all) (cmd 501).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetRequiredStorageSizeIn {
    pub dst_storage: u8,
    pub src_storage: u8,
}

const_assert_eq!(size_of::<GetRequiredStorageSizeIn>(), 0x2);

/// Wire-layout input for [`get_album_cache_ex`](crate::cmif::get_album_cache_ex) (cmd 8013).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAlbumCacheExIn {
    pub storage: u8,
    pub contents: u8,
}

const_assert_eq!(size_of::<GetAlbumCacheExIn>(), 0x2);

/// Wire-layout input for [`get_album_entry_from_app_album_entry_aruid`](crate::cmif::get_album_entry_from_app_album_entry_aruid) (cmd 8021).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAlbumEntryFromAppEntryAruidIn {
    pub application_entry: ApplicationAlbumEntry,
    pub aruid: u64,
}

const_assert_eq!(size_of::<GetAlbumEntryFromAppEntryAruidIn>(), 0x28);

/// Wire-layout input for stream read commands (cmds 2004, 2007).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ReadStreamIn {
    pub stream: u64,
    pub offset: i64,
}

const_assert_eq!(size_of::<ReadStreamIn>(), 0x10);

/// Result from loading a screenshot that returns width and height.
#[derive(Debug, Clone, Copy)]
pub struct ScreenShotDimensions {
    pub width: u64,
    pub height: u64,
}

/// Result from loading a screenshot that returns dimensions and attributes.
#[derive(Debug, Clone, Copy)]
pub struct ScreenShotImageEx0Result {
    pub attr: ScreenShotAttribute,
    pub width: i64,
    pub height: i64,
}

/// Result from overlay thumbnail queries.
#[derive(Debug, Clone, Copy)]
pub struct OverlayThumbnailResult {
    pub file_id: AlbumFileId,
    pub size: u64,
}

/// Result from [`get_min_max_applet_id`](crate::CapsaService::get_min_max_applet_id).
#[derive(Debug, Clone, Copy)]
pub struct MinMaxAppletIdResult {
    pub success: bool,
    pub min: u64,
    pub max: u64,
}
