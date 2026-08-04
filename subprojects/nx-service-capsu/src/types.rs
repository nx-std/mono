//! Application album wire-layout types.

use nx_service_caps::{
    AccountUid,
    AlbumFileDateTime,
    ApplicationAlbumFileEntry,
    ScreenShotDecodeOption,
};
use static_assertions::const_assert_eq;

/// Wire-layout input for [`set_shim_library_version`](crate::cmif::set_shim_library_version) (cmd 32).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct SetShimVersionIn {
    pub version: u64,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SetShimVersionIn>(), 0x10);

/// Wire-layout input for [`get_album_file_list_deprecated0`](crate::cmif::get_album_file_list_deprecated0) (cmd 102).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAlbumFileListDeprecated0In {
    pub content_type: u8,
    pub _pad: [u8; 7],
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileListDeprecated0In>(), 0x20);

/// Wire-layout input for [`delete_album_file`](crate::cmif::delete_album_file) (cmd 103).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct DeleteAlbumFileIn {
    pub content_type: u8,
    pub _pad: [u8; 7],
    pub entry: ApplicationAlbumFileEntry,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<DeleteAlbumFileIn>(), 0x40);

/// Wire-layout input for [`get_album_file_size`](crate::cmif::get_album_file_size) (cmd 104).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAlbumFileSizeIn {
    pub entry: ApplicationAlbumFileEntry,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileSizeIn>(), 0x38);

/// Wire-layout input for load-screenshot commands (cmds 110, 120).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LoadScreenShotIn {
    pub entry: ApplicationAlbumFileEntry,
    pub option: ScreenShotDecodeOption,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<LoadScreenShotIn>(), 0x58);

/// Wire-layout input for [`precheck_to_create_contents`](crate::cmif::precheck_to_create_contents) (cmd 130).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct PrecheckToCreateContentsIn {
    pub content_type: u8,
    pub _pad: [u8; 7],
    pub unk: u64,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<PrecheckToCreateContentsIn>(), 0x18);

/// Wire-layout input for album file list commands (cmds 140, 142).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAlbumFileListAaeIn {
    pub content_type: u8,
    pub _pad: u8,
    pub start_datetime: AlbumFileDateTime,
    pub end_datetime: AlbumFileDateTime,
    pub _pad2: [u8; 6],
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileListAaeIn>(), 0x20);

/// Wire-layout input for album file list commands with UID (cmds 141, 143).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct GetAlbumFileListAaeUidIn {
    pub content_type: u8,
    pub _pad: u8,
    pub start_datetime: AlbumFileDateTime,
    pub end_datetime: AlbumFileDateTime,
    pub _pad2: [u8; 6],
    pub uid: AccountUid,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileListAaeUidIn>(), 0x30);

/// Wire-layout input for [`open_accessor_session`](crate::cmif::open_accessor_session) (cmd 60002).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenAccessorSessionIn {
    pub entry: ApplicationAlbumFileEntry,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<OpenAccessorSessionIn>(), 0x38);

/// Wire-layout input for [`open_album_movie_read_stream`](crate::cmif::open_album_movie_read_stream) (cmd 2001).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct OpenMovieStreamIn {
    pub entry: ApplicationAlbumFileEntry,
    pub applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<OpenMovieStreamIn>(), 0x38);

/// Wire-layout input for [`read_movie_data`](crate::cmif::read_movie_data) (cmd 2004).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct ReadMovieDataIn {
    pub stream: u64,
    pub offset: i64,
}

const_assert_eq!(size_of::<ReadMovieDataIn>(), 0x10);
