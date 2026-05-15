//! CMIF protocol operations for the application album service.

use core::{mem::size_of, ptr};

use nx_service_caps::LoadAlbumScreenShotImageOutputForApplication;
use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{
        dispatch_in_pid_no_out, dispatch_in_pid_out_u64, dispatch_in_u64_no_out,
        dispatch_in_u64_out_u64,
    },
    proto,
    types::{
        DeleteAlbumFileIn, GetAlbumFileListAaeIn, GetAlbumFileListAaeUidIn,
        GetAlbumFileListDeprecated0In, GetAlbumFileSizeIn, LoadScreenShotIn, OpenAccessorSessionIn,
        OpenMovieStreamIn, PrecheckToCreateContentsIn, ReadMovieDataIn, SetShimVersionIn,
    },
};

// ---------------------------------------------------------------------------
// Root service commands (IApplicationAlbumInterface)
// ---------------------------------------------------------------------------

/// Sets the shim library version. \[7.0.0+\]
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

/// Gets album file list by timestamp (pre-6.0.0, cmd 102).
pub(crate) fn get_album_file_list_deprecated0(
    service: &Session,
    content_type: u8,
    start_timestamp: u64,
    end_timestamp: u64,
    applet_resource_user_id: u64,
    entries: &mut [u8],
) -> Result<u64, GetAlbumFileListError> {
    let input = GetAlbumFileListDeprecated0In {
        content_type,
        _pad: [0; 7],
        start_timestamp,
        end_timestamp,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetAlbumFileListDeprecated0In>(),
        )
    };
    let result = service
        .dispatch(proto::GET_ALBUM_FILE_LIST_DEPRECATED0)
        .in_raw(in_bytes)
        .send_pid()
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send()
        .map_err(GetAlbumFileListError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let total = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(total)
}

/// Deletes an album file (cmd 103).
pub(crate) fn delete_album_file(
    service: &Session,
    content_type: u8,
    entry: &nx_service_caps::ApplicationAlbumFileEntry,
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

/// Gets album file size (cmd 104).
pub(crate) fn get_album_file_size(
    service: &Session,
    entry: &nx_service_caps::ApplicationAlbumFileEntry,
    applet_resource_user_id: u64,
) -> Result<u64, DispatchError> {
    let input = GetAlbumFileSizeIn {
        entry: *entry,
        applet_resource_user_id,
    };
    dispatch_in_pid_out_u64(service, proto::GET_ALBUM_FILE_SIZE, &input)
}

/// Loads an album screenshot image (cmd 110 or 120).
#[allow(clippy::too_many_arguments)]
pub(crate) fn load_album_screenshot_image(
    service: &Session,
    cmd_id: u32,
    entry: &nx_service_caps::ApplicationAlbumFileEntry,
    option: &nx_service_caps::ScreenShotDecodeOption,
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

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<LoadScreenShotIn>(),
        )
    };
    // SAFETY: `out` is a valid exclusive reference; viewing it as bytes for
    // the OUT buffer is sound, and the byte slice borrows it.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut LoadAlbumScreenShotImageOutputForApplication).cast::<u8>(),
            size_of::<LoadAlbumScreenShotImageOutputForApplication>(),
        )
    };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .out_buffer(workbuf, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
        .map_err(LoadScreenShotImageError)
}

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

/// Gets album file list (datetime-based, no UID). \[6.0.0+\]
#[allow(clippy::too_many_arguments)]
pub(crate) fn get_album_file_list_aae(
    service: &Session,
    cmd_id: u32,
    content_type: u8,
    start_datetime: &nx_service_caps::AlbumFileDateTime,
    end_datetime: &nx_service_caps::AlbumFileDateTime,
    applet_resource_user_id: u64,
    entries: &mut [u8],
) -> Result<u64, GetAlbumFileListError> {
    let input = GetAlbumFileListAaeIn {
        content_type,
        _pad: 0,
        start_datetime: *start_datetime,
        end_datetime: *end_datetime,
        _pad2: [0; 6],
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetAlbumFileListAaeIn>(),
        )
    };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send()
        .map_err(GetAlbumFileListError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let total = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(total)
}

/// Gets album file list (datetime-based, with UID). \[6.0.0+\]
#[allow(clippy::too_many_arguments)]
pub(crate) fn get_album_file_list_aae_uid(
    service: &Session,
    cmd_id: u32,
    content_type: u8,
    start_datetime: &nx_service_caps::AlbumFileDateTime,
    end_datetime: &nx_service_caps::AlbumFileDateTime,
    uid: nx_service_caps::AccountUid,
    applet_resource_user_id: u64,
    entries: &mut [u8],
) -> Result<u64, GetAlbumFileListError> {
    let input = GetAlbumFileListAaeUidIn {
        content_type,
        _pad: 0,
        start_datetime: *start_datetime,
        end_datetime: *end_datetime,
        _pad2: [0; 6],
        uid,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetAlbumFileListAaeUidIn>(),
        )
    };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send_pid()
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send()
        .map_err(GetAlbumFileListError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let total = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(total)
}

/// Opens an accessor session (cmd 60002). Returns the move handle.
pub(crate) fn open_accessor_session(
    service: &Session,
    entry: &nx_service_caps::ApplicationAlbumFileEntry,
    applet_resource_user_id: u64,
) -> Result<u32, OpenAccessorSessionError> {
    let input = OpenAccessorSessionIn {
        entry: *entry,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<OpenAccessorSessionIn>(),
        )
    };
    let result = service
        .dispatch(proto::OPEN_ACCESSOR_SESSION)
        .in_raw(in_bytes)
        .send_pid()
        .send()
        .map_err(OpenAccessorSessionError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenAccessorSessionError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

// ---------------------------------------------------------------------------
// Accessor session commands (IAlbumAccessorApplicationSession)
// ---------------------------------------------------------------------------

/// Opens an album movie read stream (cmd 2001).
pub(crate) fn open_album_movie_read_stream(
    service: &Session,
    entry: &nx_service_caps::ApplicationAlbumFileEntry,
    applet_resource_user_id: u64,
) -> Result<u64, DispatchError> {
    let input = OpenMovieStreamIn {
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

/// Reads movie data from a read stream (cmd 2004).
pub(crate) fn read_movie_data(
    service: &Session,
    stream: u64,
    offset: i64,
    buffer: &mut [u8],
) -> Result<u64, ReadMovieDataError> {
    let input = ReadMovieDataIn { stream, offset };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<ReadMovieDataIn>(),
        )
    };
    let result = service
        .dispatch(proto::READ_MOVIE_DATA_FROM_STREAM)
        .in_raw(in_bytes)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send()
        .map_err(ReadMovieDataError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let actual_size = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(actual_size)
}

/// Gets the broken reason for a read stream (cmd 2005).
pub(crate) fn get_album_movie_stream_broken_reason(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::GET_ALBUM_MOVIE_STREAM_BROKEN_REASON, stream)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by album file list operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get album file list")]
pub struct GetAlbumFileListError(#[source] pub DispatchError);

/// Error returned by load-screenshot operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to load album screenshot image")]
pub struct LoadScreenShotImageError(#[source] pub DispatchError);

/// Error returned by [`open_accessor_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenAccessorSessionError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenAccessorSession")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenAccessorSession response missing move handle")]
    MissingHandle,
}

/// Error returned by [`read_movie_data`].
#[derive(Debug, thiserror::Error)]
#[error("failed to read movie data from stream")]
pub struct ReadMovieDataError(#[source] pub DispatchError);
