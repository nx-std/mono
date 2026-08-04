//! Album control wire-layout types.

use nx_service_caps::{
    AlbumEntry,
    AlbumFileId,
};
use static_assertions::const_assert_eq;

/// Application ID structure used by the album control service.
///
/// On 19.0.0+, the full struct is sent on the wire. On older firmware,
/// only the `application_id` field is sent as a bare `u64`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct CapsApplicationId {
    pub application_id: u64,
    pub unknown_08: u8,
    pub unknown_09: u8,
    pub reserved: [u8; 6],
}

const_assert_eq!(size_of::<CapsApplicationId>(), 0x10);

/// Wire-layout input for [`set_shim_library_version`](crate::cmif::set_shim_library_version) (cmd 33).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetShimVersionIn {
    pub version: u64,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SetShimVersionIn>(), 0x10);

/// Wire-layout input for `register`/`unregister_applet_resource_user_id` legacy (pre-19.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RegisterAruidLegacyIn {
    pub applet_resource_user_id: u64,
    pub application_id: u64,
}

const_assert_eq!(size_of::<RegisterAruidLegacyIn>(), 0x10);

/// Wire-layout input for `register`/`unregister_applet_resource_user_id` (19.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct RegisterAruidIn {
    pub applet_resource_user_id: u64,
    pub application_id: CapsApplicationId,
}

const_assert_eq!(size_of::<RegisterAruidIn>(), 0x18);

/// Wire-layout input for `generate_current_album_file_id` legacy (pre-19.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GenerateFileIdLegacyIn {
    pub contents: u8,
    pub _pad: [u8; 7],
    pub application_id: u64,
}

const_assert_eq!(size_of::<GenerateFileIdLegacyIn>(), 0x10);

/// Wire-layout input for `generate_current_album_file_id` (19.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GenerateFileIdIn {
    pub contents: u8,
    pub _pad: [u8; 7],
    pub application_id: CapsApplicationId,
}

const_assert_eq!(size_of::<GenerateFileIdIn>(), 0x18);

/// Wire-layout input for `generate_application_album_entry` (cmd 2102).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GenerateAppAlbumEntryIn {
    pub entry: AlbumEntry,
    pub application_id: u64,
}

const_assert_eq!(size_of::<GenerateAppAlbumEntryIn>(), 0x28);

/// Wire-layout input for `save_album_screenshot_file_ex` (cmd 2202).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SaveScreenShotFileExIn {
    pub file_id: AlbumFileId,
    pub version: u64,
    pub makernote_offset: u64,
    pub makernote_size: u64,
}

const_assert_eq!(size_of::<SaveScreenShotFileExIn>(), 0x30);

/// Wire-layout input for `open_control_session` (cmd 60001).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenControlSessionIn {
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<OpenControlSessionIn>(), 0x08);

/// Wire-layout input for stream read-data / read-image-data commands.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StreamReadDataIn {
    pub stream: u64,
    pub offset: u64,
}

const_assert_eq!(size_of::<StreamReadDataIn>(), 0x10);

/// Wire-layout input for stream write-data / write-meta commands.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct StreamWriteDataIn {
    pub stream: u64,
    pub offset: u64,
}

const_assert_eq!(size_of::<StreamWriteDataIn>(), 0x10);

/// Wire-layout input for `set_album_movie_write_stream_data_size` (cmd 2434).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetStreamDataSizeIn {
    pub stream: u64,
    pub size: u64,
}

const_assert_eq!(size_of::<SetStreamDataSizeIn>(), 0x10);
