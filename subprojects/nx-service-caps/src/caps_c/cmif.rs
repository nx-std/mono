//! CMIF protocol operations for the album control service.

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
        AlbumEntry,
        AlbumFileId,
        ApplicationAlbumEntry,
    },
    dispatch::{
        dispatch_in_no_out,
        dispatch_in_out,
        dispatch_in_pid_no_out,
        dispatch_in_u64_no_out,
        dispatch_in_u64_out_u64,
    },
    screenshot::ScreenShotAttribute,
};

/// Application ID structure used by the album control service.
///
/// On 19.0.0+, the full struct is sent on the wire. On older firmware,
/// only the `application_id` field is sent as a bare `u64`.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct CapsApplicationId {
    /// The application ID itself.
    pub application_id: u64,
    /// Unknown byte at offset 0x8.
    pub unknown_08: u8,
    /// Unknown byte at offset 0x9.
    pub unknown_09: u8,
    /// Reserved bytes at offset 0xa.
    pub reserved: [u8; 6],
}

const_assert_eq!(size_of::<CapsApplicationId>(), 0x10);

/// Wire-layout input for [`set_shim_library_version`] (cmd 33).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct SetShimVersionIn {
    /// Shim library version the caller implements.
    version: u64,
    /// Applet resource user ID the session belongs to.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<SetShimVersionIn>(), 0x10);

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

/// Wire-layout input for the register/unregister ARUID commands, legacy form
/// (pre-19.0.0, cmds 2011, 2012).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct RegisterAruidLegacyIn {
    /// Applet resource user ID being registered.
    applet_resource_user_id: u64,
    /// Application the ARUID is bound to.
    application_id: u64,
}

const_assert_eq!(size_of::<RegisterAruidLegacyIn>(), 0x10);

/// Wire-layout input for the register/unregister ARUID commands
/// (19.0.0+, cmds 2011, 2012).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct RegisterAruidIn {
    /// Applet resource user ID being registered.
    applet_resource_user_id: u64,
    /// Application the ARUID is bound to.
    application_id: CapsApplicationId,
}

const_assert_eq!(size_of::<RegisterAruidIn>(), 0x18);

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

/// Wire-layout input for `generate_current_album_file_id`, legacy form
/// (pre-19.0.0, cmd 2101).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct GenerateFileIdLegacyIn {
    /// Content type the generated ID is for.
    contents: u8,
    /// Padding the wire form carries after the content type.
    _pad: [u8; 7],
    /// Application the generated ID is attributed to.
    application_id: u64,
}

const_assert_eq!(size_of::<GenerateFileIdLegacyIn>(), 0x10);

/// Wire-layout input for `generate_current_album_file_id` (19.0.0+, cmd 2101).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct GenerateFileIdIn {
    /// Content type the generated ID is for.
    contents: u8,
    /// Padding the wire form carries after the content type.
    _pad: [u8; 7],
    /// Application the generated ID is attributed to.
    application_id: CapsApplicationId,
}

const_assert_eq!(size_of::<GenerateFileIdIn>(), 0x18);

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

/// Wire-layout input for [`generate_application_album_entry`] (cmd 2102).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct GenerateAppAlbumEntryIn {
    /// Album entry to derive the application entry from.
    entry: AlbumEntry,
    /// Application the derived entry is for.
    application_id: u64,
}

const_assert_eq!(size_of::<GenerateAppAlbumEntryIn>(), 0x28);

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
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SAVE_ALBUM_SCREENSHOT_FILE)
        .in_raw(file_id.as_bytes())
        .in_buffer(
            buffer,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(SaveScreenShotError)
}

/// Wire-layout input for [`save_album_screenshot_file_ex`] (cmd 2202).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct SaveScreenShotFileExIn {
    /// File the screenshot is saved as.
    file_id: AlbumFileId,
    /// Format version of the saved file.
    version: u64,
    /// Offset of the maker note within the JPEG buffer.
    makernote_offset: u64,
    /// Size of the maker note, in bytes.
    makernote_size: u64,
}

const_assert_eq!(size_of::<SaveScreenShotFileExIn>(), 0x30);

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

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SAVE_ALBUM_SCREENSHOT_FILE_EX)
        .in_raw(input.as_bytes())
        .in_buffer(
            buffer,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(SaveScreenShotError)
}

/// Error returned by save-screenshot operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to save album screenshot file")]
pub struct SaveScreenShotError(#[source] pub DispatchError);

/// Sets overlay thumbnail data (cmds 2301, 2302).
pub(crate) fn set_overlay_thumbnail_data(
    service: &Session,
    cmd_id: u32,
    file_id: &AlbumFileId,
    image: &[u8],
) -> Result<(), SetOverlayThumbnailError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(file_id.as_bytes())
        .in_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(SetOverlayThumbnailError)
}

/// Error returned by set-overlay-thumbnail operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to set overlay thumbnail data")]
pub struct SetOverlayThumbnailError(#[source] pub DispatchError);

/// Wire-layout input for [`open_control_session`] (cmd 60001).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct OpenControlSessionIn {
    /// Applet resource user ID the session is opened for.
    applet_resource_user_id: u64,
}

const_assert_eq!(size_of::<OpenControlSessionIn>(), 0x08);

/// Opens an album control session (cmd 60001). Returns the move handle.
pub(crate) fn open_control_session(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<u32, OpenControlSessionError> {
    let input = OpenControlSessionIn {
        applet_resource_user_id,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::OPEN_CONTROL_SESSION)
        .in_raw(input.as_bytes())
        .send_pid()
        .send(&mut ipc_buf)
        .map_err(OpenControlSessionError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenControlSessionError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

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

/// Wire-layout input for the stream data commands that address a byte offset
/// (ctrl cmds 2004, 2007, 2421, 2422, 2424).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct StreamOffsetIn {
    /// Stream handle the transfer applies to.
    stream: u64,
    /// Byte offset to transfer at, aligned to 0x40000.
    offset: u64,
}

const_assert_eq!(size_of::<StreamOffsetIn>(), 0x10);

/// Reads movie data from a read stream (ctrl cmd 2004).
pub(crate) fn ctrl_read_movie_data(
    service: &Session,
    stream: u64,
    offset: u64,
    buffer: &mut [u8],
) -> Result<u64, CapscReadStreamDataError> {
    let input = StreamOffsetIn { stream, offset };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::CTRL_READ_MOVIE_DATA_FROM_READ_STREAM)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(CapscReadStreamDataError)?;

    Ok(*result.value::<u64>())
}

/// Error returned by stream read-data operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to read data from album movie stream")]
pub struct CapscReadStreamDataError(#[source] pub DispatchError);

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
) -> Result<u64, CapscReadStreamDataError> {
    let input = StreamOffsetIn { stream, offset };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::CTRL_READ_IMAGE_DATA_FROM_READ_STREAM)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(CapscReadStreamDataError)?;

    Ok(*result.value::<u64>())
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
) -> Result<u64, CapscReadStreamDataError> {
    let input = StreamOffsetIn { stream, offset };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::CTRL_READ_DATA_FROM_WRITE_STREAM)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(CapscReadStreamDataError)?;

    Ok(*result.value::<u64>())
}

/// Writes data to a write stream (ctrl cmd 2422).
pub(crate) fn ctrl_write_data_to_write_stream(
    service: &Session,
    stream: u64,
    offset: u64,
    buffer: &[u8],
) -> Result<(), WriteStreamDataError> {
    let input = StreamOffsetIn { stream, offset };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::CTRL_WRITE_DATA_TO_WRITE_STREAM)
        .in_raw(input.as_bytes())
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
    let input = StreamOffsetIn { stream, offset };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::CTRL_WRITE_META_TO_WRITE_STREAM)
        .in_raw(input.as_bytes())
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(WriteStreamDataError)
}

/// Error returned by stream write-data operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to write data to album movie stream")]
pub struct WriteStreamDataError(#[source] pub DispatchError);

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

/// Wire-layout input for [`ctrl_set_write_stream_data_size`] (ctrl cmd 2434).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
struct SetStreamDataSizeIn {
    /// Stream handle the size applies to.
    stream: u64,
    /// Data size to set, in bytes; must not exceed 2 GiB.
    size: u64,
}

const_assert_eq!(size_of::<SetStreamDataSizeIn>(), 0x10);

/// Sets the data size of a write stream (ctrl cmd 2434).
pub(crate) fn ctrl_set_write_stream_data_size(
    service: &Session,
    stream: u64,
    size: u64,
) -> Result<(), DispatchError> {
    let input = SetStreamDataSizeIn { stream, size };
    dispatch_in_no_out(service, proto::CTRL_SET_WRITE_STREAM_DATA_SIZE, &input)
}
