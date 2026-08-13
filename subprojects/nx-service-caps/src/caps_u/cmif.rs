//! CMIF protocol operations for the application album service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use static_assertions::const_assert_eq;
use zerocopy::IntoBytes as _;

use super::proto;
use crate::{
    album::{
        AlbumFileDateTime,
        ApplicationAlbumFileEntry,
    },
    dispatch::{
        dispatch_in_pid_no_out,
        dispatch_in_pid_out_u64,
        dispatch_in_u64_no_out,
        dispatch_in_u64_out_u64,
    },
    screenshot::{
        LoadAlbumScreenShotImageOutputForApplication,
        ScreenShotDecodeOption,
    },
    user::AccountUid,
};

/// Wire-layout input for [`set_shim_library_version`] (cmd 32).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct SetShimVersionIn {
    /// Shim library version the caller implements.
    version: u64,
    /// Applet resource user ID the session belongs to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SetShimVersionIn>(), 0x10);

/// Sets the shim library version (cmd 32). \[7.0.0+\]
pub(crate) fn set_shim_library_version(
    service: &Session,
    version: u64,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = SetShimVersionIn {
        version,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::SET_SHIM_LIBRARY_VERSION, &input)
}

/// Wire-layout input for [`get_album_file_list_deprecated0`] (cmd 102).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct GetAlbumFileListDeprecated0In {
    /// Content type to list.
    content_type: u8,
    /// Padding the wire form carries after the content type.
    _pad: [u8; 7],
    /// Inclusive lower bound of the listed range.
    start_timestamp: u64,
    /// Inclusive upper bound of the listed range.
    end_timestamp: u64,
    /// Applet resource user ID the listing is scoped to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileListDeprecated0In>(), 0x20);

/// Gets album file list by timestamp (pre-6.0.0, cmd 102).
pub(crate) fn get_album_file_list_deprecated0(
    service: &Session,
    content_type: u8,
    start_timestamp: u64,
    end_timestamp: u64,
    applet_resource_user_id: u64,
    entries: &mut [u8],
) -> Result<u64, CapsuGetAlbumFileListError> {
    let input = GetAlbumFileListDeprecated0In {
        content_type,
        _pad: [0; 7],
        start_timestamp,
        end_timestamp,
        applet_resource_user_id,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_ALBUM_FILE_LIST_DEPRECATED0)
        .in_raw(input.as_bytes())
        .send_pid()
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut buf)
        .map_err(CapsuGetAlbumFileListError)?;

    Ok(*result.value::<u64>())
}

/// Error returned by album file list operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get album file list")]
pub struct CapsuGetAlbumFileListError(#[source] pub DispatchError);

/// Wire-layout input for [`delete_album_file`] (cmd 103).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct DeleteAlbumFileIn {
    /// Content type of the file being deleted.
    content_type: u8,
    /// Padding the wire form carries after the content type.
    _pad: [u8; 7],
    /// Entry identifying the file.
    entry: ApplicationAlbumFileEntry,
    /// Applet resource user ID the file belongs to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<DeleteAlbumFileIn>(), 0x40);

/// Deletes an album file (cmd 103).
pub(crate) fn delete_album_file(
    service: &Session,
    content_type: u8,
    entry: &ApplicationAlbumFileEntry,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = DeleteAlbumFileIn {
        content_type,
        _pad: [0; 7],
        entry: *entry,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::DELETE_ALBUM_FILE, &input)
}

/// Wire-layout input for [`get_album_file_size`] (cmd 104).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct GetAlbumFileSizeIn {
    /// Entry identifying the file.
    entry: ApplicationAlbumFileEntry,
    /// Applet resource user ID the file belongs to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileSizeIn>(), 0x38);

/// Gets album file size (cmd 104).
pub(crate) fn get_album_file_size(
    service: &Session,
    entry: &ApplicationAlbumFileEntry,
    applet_resource_user_id: u64,
) -> Result<u64, DispatchError> {
    let input = GetAlbumFileSizeIn {
        entry: *entry,
        applet_resource_user_id,
    };
    dispatch_in_pid_out_u64(service, proto::GET_ALBUM_FILE_SIZE, &input)
}

/// Wire-layout input for the load-screenshot commands (cmds 110, 120).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct LoadScreenShotIn {
    /// Entry identifying the image to load.
    entry: ApplicationAlbumFileEntry,
    /// Decoder behaviour flags.
    option: ScreenShotDecodeOption,
    /// Applet resource user ID the image belongs to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<LoadScreenShotIn>(), 0x58);

/// Loads an album screenshot image (cmds 110, 120).
#[expect(
    clippy::too_many_arguments,
    reason = "the parameters are the fields of LoadScreenShotIn plus the three output buffers the command \
              maps; the command id is what makes one body serve cmds 110 and 120"
)]
pub(crate) fn load_album_screenshot_image(
    service: &Session,
    cmd_id: u32,
    entry: &ApplicationAlbumFileEntry,
    option: &ScreenShotDecodeOption,
    applet_resource_user_id: u64,
    out: &mut LoadAlbumScreenShotImageOutputForApplication,
    image: &mut [u8],
    workbuf: &mut [u8],
) -> Result<(), LoadScreenShotImageError> {
    let input = LoadScreenShotIn {
        entry: *entry,
        option: *option,
        applet_resource_user_id,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send_pid()
        .out_buffer(out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .out_buffer(workbuf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut buf)
        .map(|_| ())
        .map_err(LoadScreenShotImageError)
}

/// Error returned by load-screenshot operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to load album screenshot image")]
pub struct LoadScreenShotImageError(#[source] pub DispatchError);

/// Wire-layout input for [`precheck_to_create_contents`] (cmd 130).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct PrecheckToCreateContentsIn {
    /// Content type the caller intends to create.
    content_type: u8,
    /// Padding the wire form carries after the content type.
    _pad: [u8; 7],
    /// Unknown parameter the command carries.
    unk: u64,
    /// Applet resource user ID the check is scoped to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<PrecheckToCreateContentsIn>(), 0x18);

/// Prechecks to create contents (cmd 130).
pub(crate) fn precheck_to_create_contents(
    service: &Session,
    content_type: u8,
    unk: u64,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = PrecheckToCreateContentsIn {
        content_type,
        _pad: [0; 7],
        unk,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::PRECHECK_TO_CREATE_CONTENTS, &input)
}

/// Wire-layout input for the date-ranged album file list commands (cmds 140, 142).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct GetAlbumFileListAaeIn {
    /// Content type to list.
    content_type: u8,
    /// Padding the wire form carries after the content type.
    _pad: u8,
    /// Inclusive lower bound of the listed range.
    start_datetime: AlbumFileDateTime,
    /// Inclusive upper bound of the listed range.
    end_datetime: AlbumFileDateTime,
    /// Padding the wire form carries before the ARUID.
    _pad2: [u8; 6],
    /// Applet resource user ID the listing is scoped to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileListAaeIn>(), 0x20);

/// Gets album file list (datetime-based, no UID). \[6.0.0+\]
pub(crate) fn get_album_file_list_aae(
    service: &Session,
    cmd_id: u32,
    content_type: u8,
    start_datetime: &AlbumFileDateTime,
    end_datetime: &AlbumFileDateTime,
    applet_resource_user_id: u64,
    entries: &mut [u8],
) -> Result<u64, CapsuGetAlbumFileListError> {
    let input = GetAlbumFileListAaeIn {
        content_type,
        _pad: 0,
        start_datetime: *start_datetime,
        end_datetime: *end_datetime,
        _pad2: [0; 6],
        applet_resource_user_id,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send_pid()
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut buf)
        .map_err(CapsuGetAlbumFileListError)?;

    Ok(*result.value::<u64>())
}

/// Wire-layout input for the date-ranged, UID-filtered album file list commands
/// (cmds 141, 143).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct GetAlbumFileListAaeUidIn {
    /// Content type to list.
    content_type: u8,
    /// Padding the wire form carries after the content type.
    _pad: u8,
    /// Inclusive lower bound of the listed range.
    start_datetime: AlbumFileDateTime,
    /// Inclusive upper bound of the listed range.
    end_datetime: AlbumFileDateTime,
    /// Padding the wire form carries before the user ID.
    _pad2: [u8; 6],
    /// User the listing is filtered to.
    uid: AccountUid,
    /// Applet resource user ID the listing is scoped to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<GetAlbumFileListAaeUidIn>(), 0x30);

/// Gets album file list (datetime-based, with UID). \[6.0.0+\]
#[expect(
    clippy::too_many_arguments,
    reason = "the parameters are the fields of GetAlbumFileListAaeUidIn plus the output buffer; the command \
              id is what makes one body serve cmds 141 and 143"
)]
pub(crate) fn get_album_file_list_aae_uid(
    service: &Session,
    cmd_id: u32,
    content_type: u8,
    start_datetime: &AlbumFileDateTime,
    end_datetime: &AlbumFileDateTime,
    uid: AccountUid,
    applet_resource_user_id: u64,
    entries: &mut [u8],
) -> Result<u64, CapsuGetAlbumFileListError> {
    let input = GetAlbumFileListAaeUidIn {
        content_type,
        _pad: 0,
        start_datetime: *start_datetime,
        end_datetime: *end_datetime,
        _pad2: [0; 6],
        uid,
        applet_resource_user_id,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send_pid()
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut buf)
        .map_err(CapsuGetAlbumFileListError)?;

    Ok(*result.value::<u64>())
}

/// Wire-layout input for the commands that name an entry and its owner
/// (cmds 60002, 2001).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct EntryAruidIn {
    /// Entry identifying the album file.
    entry: ApplicationAlbumFileEntry,
    /// Applet resource user ID the entry belongs to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<EntryAruidIn>(), 0x38);

/// Opens an accessor session (cmd 60002). Returns the move handle.
pub(crate) fn open_accessor_session(
    service: &Session,
    entry: &ApplicationAlbumFileEntry,
    applet_resource_user_id: u64,
) -> Result<u32, CapsuOpenAccessorSessionError> {
    let input = EntryAruidIn {
        entry: *entry,
        applet_resource_user_id,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::OPEN_ACCESSOR_SESSION)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut buf)
        .map_err(CapsuOpenAccessorSessionError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(CapsuOpenAccessorSessionError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// Error returned by [`open_accessor_session`].
#[derive(Debug, thiserror::Error)]
pub enum CapsuOpenAccessorSessionError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenAccessorSession")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenAccessorSession response missing move handle")]
    MissingHandle,
}

/// Opens an album movie read stream (cmd 2001).
pub(crate) fn open_album_movie_read_stream(
    service: &Session,
    entry: &ApplicationAlbumFileEntry,
    applet_resource_user_id: u64,
) -> Result<u64, DispatchError> {
    let input = EntryAruidIn {
        entry: *entry,
        applet_resource_user_id,
    };
    dispatch_in_pid_out_u64(service, proto::OPEN_ALBUM_MOVIE_READ_STREAM, &input)
}

/// Closes an album movie read stream (cmd 2002).
pub(crate) fn close_album_movie_read_stream(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CLOSE_ALBUM_MOVIE_READ_STREAM, stream)
}

/// Gets the movie data size of a read stream (cmd 2003).
pub(crate) fn get_album_movie_read_stream_data_size(
    service: &Session,
    stream: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_u64_out_u64(
        service,
        proto::GET_ALBUM_MOVIE_READ_STREAM_DATA_SIZE,
        stream,
    )
}

/// Wire-layout input for [`read_movie_data`] (cmd 2004).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct ReadMovieDataIn {
    /// Stream handle to read from.
    stream: u64,
    /// Byte offset to read at, aligned to 0x40000.
    offset: i64,
}

const_assert_eq!(size_of::<ReadMovieDataIn>(), 0x10);

/// Reads movie data from a read stream (cmd 2004).
pub(crate) fn read_movie_data(
    service: &Session,
    stream: u64,
    offset: i64,
    buffer: &mut [u8],
) -> Result<u64, ReadMovieDataError> {
    let input = ReadMovieDataIn { stream, offset };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::READ_MOVIE_DATA_FROM_STREAM)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut buf)
        .map_err(ReadMovieDataError)?;

    Ok(*result.value::<u64>())
}

/// Error returned by [`read_movie_data`].
#[derive(Debug, thiserror::Error)]
#[error("failed to read movie data from stream")]
pub struct ReadMovieDataError(#[source] pub DispatchError);

/// Gets the broken reason for a read stream (cmd 2005).
pub(crate) fn get_album_movie_stream_broken_reason(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::GET_ALBUM_MOVIE_STREAM_BROKEN_REASON, stream)
}
