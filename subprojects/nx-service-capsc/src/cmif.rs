//! CMIF protocol operations for the album control service.

use core::{mem::size_of, ptr};

use nx_service_caps::{AlbumEntry, AlbumFileId, ApplicationAlbumEntry, ScreenShotAttribute};
use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{
        dispatch_in_no_out, dispatch_in_out, dispatch_in_pid_no_out, dispatch_in_u64_no_out,
        dispatch_in_u64_out_u64,
    },
    proto,
    types::{
        CapsApplicationId, GenerateAppAlbumEntryIn, GenerateFileIdIn, GenerateFileIdLegacyIn,
        OpenControlSessionIn, RegisterAruidIn, RegisterAruidLegacyIn, SaveScreenShotFileExIn,
        SetShimVersionIn, SetStreamDataSizeIn, StreamReadDataIn, StreamWriteDataIn,
    },
};

// ---------------------------------------------------------------------------
// Root service commands (IAlbumControlService)
// ---------------------------------------------------------------------------

/// Sets the shim library version (cmd 33). \[7.0.0+\]
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

/// Notifies that an album storage is available (cmd 2001).
pub(crate) fn notify_album_storage_is_available(
    service: &Session,
    storage: u8,
) -> Result<(), DispatchError> {
    let value = storage as u64;
    dispatch_in_no_out(service, proto::NOTIFY_ALBUM_STORAGE_IS_AVAILABLE, &value)
}

/// Notifies that an album storage is unavailable (cmd 2002).
pub(crate) fn notify_album_storage_is_unavailable(
    service: &Session,
    storage: u8,
) -> Result<(), DispatchError> {
    let value = storage as u64;
    dispatch_in_no_out(service, proto::NOTIFY_ALBUM_STORAGE_IS_UNAVAILABLE, &value)
}

/// Registers an applet resource user ID, legacy wire format (pre-19.0.0, cmd 2011).
pub(crate) fn register_applet_resource_user_id_legacy(
    service: &Session,
    applet_resource_user_id: u64,
    application_id: u64,
) -> Result<(), DispatchError> {
    let input = RegisterAruidLegacyIn {
        applet_resource_user_id,
        application_id,
    };
    dispatch_in_no_out(service, proto::REGISTER_APPLET_RESOURCE_USER_ID, &input)
}

/// Registers an applet resource user ID (19.0.0+, cmd 2011).
pub(crate) fn register_applet_resource_user_id(
    service: &Session,
    applet_resource_user_id: u64,
    application_id: &CapsApplicationId,
) -> Result<(), DispatchError> {
    let input = RegisterAruidIn {
        applet_resource_user_id,
        application_id: *application_id,
    };
    dispatch_in_no_out(service, proto::REGISTER_APPLET_RESOURCE_USER_ID, &input)
}

/// Unregisters an applet resource user ID, legacy wire format (pre-19.0.0, cmd 2012).
pub(crate) fn unregister_applet_resource_user_id_legacy(
    service: &Session,
    applet_resource_user_id: u64,
    application_id: u64,
) -> Result<(), DispatchError> {
    let input = RegisterAruidLegacyIn {
        applet_resource_user_id,
        application_id,
    };
    dispatch_in_no_out(service, proto::UNREGISTER_APPLET_RESOURCE_USER_ID, &input)
}

/// Unregisters an applet resource user ID (19.0.0+, cmd 2012).
pub(crate) fn unregister_applet_resource_user_id(
    service: &Session,
    applet_resource_user_id: u64,
    application_id: &CapsApplicationId,
) -> Result<(), DispatchError> {
    let input = RegisterAruidIn {
        applet_resource_user_id,
        application_id: *application_id,
    };
    dispatch_in_no_out(service, proto::UNREGISTER_APPLET_RESOURCE_USER_ID, &input)
}

/// Gets the application ID from an ARUID, legacy wire format (pre-19.0.0, cmd 2013).
pub(crate) fn get_application_id_from_aruid_legacy(
    service: &Session,
    aruid: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::GET_APPLICATION_ID_FROM_ARUID, &aruid)
}

/// Gets the application ID from an ARUID (19.0.0+, cmd 2013).
pub(crate) fn get_application_id_from_aruid(
    service: &Session,
    aruid: u64,
) -> Result<CapsApplicationId, DispatchError> {
    dispatch_in_out(service, proto::GET_APPLICATION_ID_FROM_ARUID, &aruid)
}

/// Checks whether an application ID is registered (cmd 2014).
pub(crate) fn check_application_id_registered(
    service: &Session,
    application_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in_no_out(
        service,
        proto::CHECK_APPLICATION_ID_REGISTERED,
        &application_id,
    )
}

/// Generates a current album file ID, legacy wire format (pre-19.0.0, cmd 2101).
pub(crate) fn generate_current_album_file_id_legacy(
    service: &Session,
    contents: u8,
    application_id: u64,
) -> Result<AlbumFileId, DispatchError> {
    let input = GenerateFileIdLegacyIn {
        contents,
        _pad: [0; 7],
        application_id,
    };
    dispatch_in_out(service, proto::GENERATE_CURRENT_ALBUM_FILE_ID, &input)
}

/// Generates a current album file ID (19.0.0+, cmd 2101).
pub(crate) fn generate_current_album_file_id(
    service: &Session,
    contents: u8,
    application_id: &CapsApplicationId,
) -> Result<AlbumFileId, DispatchError> {
    let input = GenerateFileIdIn {
        contents,
        _pad: [0; 7],
        application_id: *application_id,
    };
    dispatch_in_out(service, proto::GENERATE_CURRENT_ALBUM_FILE_ID, &input)
}

/// Generates an application album entry (cmd 2102).
pub(crate) fn generate_application_album_entry(
    service: &Session,
    entry: &AlbumEntry,
    application_id: u64,
) -> Result<ApplicationAlbumEntry, DispatchError> {
    let input = GenerateAppAlbumEntryIn {
        entry: *entry,
        application_id,
    };
    dispatch_in_out(service, proto::GENERATE_APPLICATION_ALBUM_ENTRY, &input)
}

/// Saves an album screenshot file (cmd 2201). \[2.0.0–3.0.2\]
pub(crate) fn save_album_screenshot_file(
    service: &Session,
    file_id: &AlbumFileId,
    buffer: &[u8],
) -> Result<(), SaveScreenShotError> {
    // SAFETY: `file_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const *file_id).cast::<u8>(), size_of::<AlbumFileId>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::SAVE_ALBUM_SCREENSHOT_FILE)
        .in_raw(in_bytes)
        .in_buffer(
            buffer,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(SaveScreenShotError)
}

/// Saves an album screenshot file (extended, cmd 2202). \[4.0.0+\]
pub(crate) fn save_album_screenshot_file_ex(
    service: &Session,
    file_id: &AlbumFileId,
    version: u64,
    makernote_offset: u64,
    makernote_size: u64,
    buffer: &[u8],
) -> Result<(), SaveScreenShotError> {
    let input = SaveScreenShotFileExIn {
        file_id: *file_id,
        version,
        makernote_offset,
        makernote_size,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<SaveScreenShotFileExIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::SAVE_ALBUM_SCREENSHOT_FILE_EX)
        .in_raw(in_bytes)
        .in_buffer(
            buffer,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(SaveScreenShotError)
}

/// Sets overlay thumbnail data (shared impl for cmds 2301/2302).
pub(crate) fn set_overlay_thumbnail_data(
    service: &Session,
    cmd_id: u32,
    file_id: &AlbumFileId,
    image: &[u8],
) -> Result<(), SetOverlayThumbnailError> {
    // SAFETY: `file_id` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const *file_id).cast::<u8>(), size_of::<AlbumFileId>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(SetOverlayThumbnailError)
}

/// Opens an album control session (cmd 60001). Returns the sub-object service.
pub(crate) fn open_control_session(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<u32, OpenControlSessionError> {
    let input = OpenControlSessionIn {
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<OpenControlSessionIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::OPEN_CONTROL_SESSION)
        .in_raw(in_bytes)
        .send_pid()
        .send(&mut ipc_buf)
        .map_err(OpenControlSessionError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenControlSessionError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

// ---------------------------------------------------------------------------
// Control session commands (IAlbumControlSession)
// ---------------------------------------------------------------------------

/// Opens an album movie read stream (ctrl cmd 2001).
pub(crate) fn ctrl_open_album_movie_read_stream(
    service: &Session,
    file_id: &AlbumFileId,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::CTRL_OPEN_ALBUM_MOVIE_READ_STREAM, file_id)
}

/// Opens an album movie write stream (ctrl cmd 2401).
pub(crate) fn ctrl_open_album_movie_write_stream(
    service: &Session,
    file_id: &AlbumFileId,
) -> Result<u64, DispatchError> {
    dispatch_in_out(service, proto::CTRL_OPEN_ALBUM_MOVIE_WRITE_STREAM, file_id)
}

/// Closes an album movie stream (ctrl cmd 2002).
pub(crate) fn ctrl_close_album_movie_stream(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_CLOSE_ALBUM_MOVIE_STREAM, stream)
}

/// Gets the size of an album movie stream (ctrl cmd 2003).
pub(crate) fn ctrl_get_album_movie_stream_size(
    service: &Session,
    stream: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_u64_out_u64(service, proto::CTRL_GET_ALBUM_MOVIE_STREAM_SIZE, stream)
}

/// Reads movie data from a read stream (ctrl cmd 2004).
pub(crate) fn ctrl_read_movie_data(
    service: &Session,
    stream: u64,
    offset: u64,
    buffer: &mut [u8],
) -> Result<u64, ReadStreamDataError> {
    let input = StreamReadDataIn { stream, offset };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<StreamReadDataIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::CTRL_READ_MOVIE_DATA_FROM_READ_STREAM)
        .in_raw(in_bytes)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(ReadStreamDataError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let actual_size = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(actual_size)
}

/// Gets the broken reason for a read stream (ctrl cmd 2005).
pub(crate) fn ctrl_get_read_stream_broken_reason(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(
        service,
        proto::CTRL_GET_ALBUM_MOVIE_READ_STREAM_BROKEN_REASON,
        stream,
    )
}

/// Gets the image data size for a read stream (ctrl cmd 2006).
pub(crate) fn ctrl_get_read_stream_image_data_size(
    service: &Session,
    stream: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_u64_out_u64(
        service,
        proto::CTRL_GET_ALBUM_MOVIE_READ_STREAM_IMAGE_DATA_SIZE,
        stream,
    )
}

/// Reads image data from a read stream (ctrl cmd 2007).
pub(crate) fn ctrl_read_image_data(
    service: &Session,
    stream: u64,
    offset: u64,
    buffer: &mut [u8],
) -> Result<u64, ReadStreamDataError> {
    let input = StreamReadDataIn { stream, offset };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<StreamReadDataIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::CTRL_READ_IMAGE_DATA_FROM_READ_STREAM)
        .in_raw(in_bytes)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(ReadStreamDataError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let actual_size = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(actual_size)
}

/// Reads file attribute from a read stream (ctrl cmd 2008).
pub(crate) fn ctrl_read_file_attribute(
    service: &Session,
    stream: u64,
) -> Result<ScreenShotAttribute, DispatchError> {
    dispatch_in_out(
        service,
        proto::CTRL_READ_FILE_ATTRIBUTE_FROM_READ_STREAM,
        &stream,
    )
}

/// Finishes a write stream (ctrl cmd 2402).
pub(crate) fn ctrl_finish_write_stream(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_FINISH_ALBUM_MOVIE_WRITE_STREAM, stream)
}

/// Commits a write stream (ctrl cmd 2403).
pub(crate) fn ctrl_commit_write_stream(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_COMMIT_ALBUM_MOVIE_WRITE_STREAM, stream)
}

/// Discards a write stream (ctrl cmd 2404).
pub(crate) fn ctrl_discard_write_stream(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(
        service,
        proto::CTRL_DISCARD_ALBUM_MOVIE_WRITE_STREAM,
        stream,
    )
}

/// Discards a write stream without deleting temp file (ctrl cmd 2405).
pub(crate) fn ctrl_discard_write_stream_no_delete(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(
        service,
        proto::CTRL_DISCARD_ALBUM_MOVIE_WRITE_STREAM_NO_DELETE,
        stream,
    )
}

/// Commits a write stream (extended, returns AlbumEntry, ctrl cmd 2406).
pub(crate) fn ctrl_commit_write_stream_ex(
    service: &Session,
    stream: u64,
) -> Result<AlbumEntry, DispatchError> {
    dispatch_in_out(
        service,
        proto::CTRL_COMMIT_ALBUM_MOVIE_WRITE_STREAM_EX,
        &stream,
    )
}

/// Starts the data section of a write stream (ctrl cmd 2411).
pub(crate) fn ctrl_start_write_stream_data_section(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_START_WRITE_STREAM_DATA_SECTION, stream)
}

/// Ends the data section of a write stream (ctrl cmd 2412).
pub(crate) fn ctrl_end_write_stream_data_section(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_END_WRITE_STREAM_DATA_SECTION, stream)
}

/// Starts the meta section of a write stream (ctrl cmd 2413).
pub(crate) fn ctrl_start_write_stream_meta_section(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_START_WRITE_STREAM_META_SECTION, stream)
}

/// Ends the meta section of a write stream (ctrl cmd 2414).
pub(crate) fn ctrl_end_write_stream_meta_section(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_END_WRITE_STREAM_META_SECTION, stream)
}

/// Reads data from a write stream (ctrl cmd 2421).
pub(crate) fn ctrl_read_data_from_write_stream(
    service: &Session,
    stream: u64,
    offset: u64,
    buffer: &mut [u8],
) -> Result<u64, ReadStreamDataError> {
    let input = StreamReadDataIn { stream, offset };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<StreamReadDataIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::CTRL_READ_DATA_FROM_WRITE_STREAM)
        .in_raw(in_bytes)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(ReadStreamDataError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let actual_size = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(actual_size)
}

/// Writes data to a write stream (ctrl cmd 2422).
pub(crate) fn ctrl_write_data_to_write_stream(
    service: &Session,
    stream: u64,
    offset: u64,
    buffer: &[u8],
) -> Result<(), WriteStreamDataError> {
    let input = StreamWriteDataIn { stream, offset };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<StreamWriteDataIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::CTRL_WRITE_DATA_TO_WRITE_STREAM)
        .in_raw(in_bytes)
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(WriteStreamDataError)
}

/// Writes meta to a write stream (ctrl cmd 2424).
pub(crate) fn ctrl_write_meta_to_write_stream(
    service: &Session,
    stream: u64,
    offset: u64,
    buffer: &[u8],
) -> Result<(), WriteStreamDataError> {
    let input = StreamWriteDataIn { stream, offset };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<StreamWriteDataIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::CTRL_WRITE_META_TO_WRITE_STREAM)
        .in_raw(in_bytes)
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(WriteStreamDataError)
}

/// Gets the broken reason for a write stream (ctrl cmd 2431).
pub(crate) fn ctrl_get_write_stream_broken_reason(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64_no_out(service, proto::CTRL_GET_WRITE_STREAM_BROKEN_REASON, stream)
}

/// Gets the data size of a write stream (ctrl cmd 2433).
pub(crate) fn ctrl_get_write_stream_data_size(
    service: &Session,
    stream: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_u64_out_u64(service, proto::CTRL_GET_WRITE_STREAM_DATA_SIZE, stream)
}

/// Sets the data size of a write stream (ctrl cmd 2434).
pub(crate) fn ctrl_set_write_stream_data_size(
    service: &Session,
    stream: u64,
    size: u64,
) -> Result<(), DispatchError> {
    let input = SetStreamDataSizeIn { stream, size };
    dispatch_in_no_out(service, proto::CTRL_SET_WRITE_STREAM_DATA_SIZE, &input)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by save-screenshot operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to save album screenshot file")]
pub struct SaveScreenShotError(#[source] pub DispatchError);

/// Error returned by set-overlay-thumbnail operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to set overlay thumbnail data")]
pub struct SetOverlayThumbnailError(#[source] pub DispatchError);

/// Error returned by [`open_control_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenControlSessionError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenControlSession")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenControlSession response missing move handle")]
    MissingHandle,
}

/// Error returned by stream read-data operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to read data from album movie stream")]
pub struct ReadStreamDataError(#[source] pub DispatchError);

/// Error returned by stream write-data operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to write data to album movie stream")]
pub struct WriteStreamDataError(#[source] pub DispatchError);
