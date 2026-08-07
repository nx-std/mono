//! CMIF protocol operations for the album accessor service.

use core::mem::size_of;

use nx_service_caps::{
    AlbumCache,
    AlbumEntry,
    AlbumFileId,
    AlbumUsage2,
    AlbumUsage3,
    AlbumUsage16,
    ApplicationAlbumEntry,
    LoadAlbumScreenShotImageOutput,
    ScreenShotAttribute,
    ScreenShotDecodeOption,
};
use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use static_assertions::const_assert_eq;
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in_no_out,
        dispatch_in_out,
        dispatch_in_pid_out,
        dispatch_out,
    },
    proto,
    types::{
        GetAlbumCacheExIn,
        GetAlbumEntryFromAppEntryAruidIn,
        GetAlbumEntryFromAppEntryIn,
        GetLastOverlayThumbnailOut,
        GetMinMaxAppletIdOut,
        GetRequiredStorageSizeIn,
        LoadScreenShotEx0Out,
        LoadScreenShotOut,
        ReadStreamIn,
        StorageCopyAlbumFileIn,
        StorageFlagsIn,
    },
};

/// Gets the number of album files in a storage (cmd 0).
pub(crate) fn get_album_file_count(service: &Session, storage: u8) -> Result<u64, DispatchError> {
    dispatch_in_out::<u8, u64>(service, proto::GET_ALBUM_FILE_COUNT, &storage)
}

/// Gets a listing of album entries (cmd 1).
pub(crate) fn get_album_file_list(
    service: &Session,
    storage: u8,
    entries: &mut [u8],
) -> Result<u64, GetAlbumFileListError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_ALBUM_FILE_LIST)
        .in_raw(storage.as_bytes())
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(GetAlbumFileListError)?;

    Ok(*result.value::<u64>())
}

/// Loads an album file into a buffer (cmd 2).
pub(crate) fn load_album_file(
    service: &Session,
    file_id: &AlbumFileId,
    filebuf: &mut [u8],
) -> Result<u64, LoadAlbumFileError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::LOAD_ALBUM_FILE)
        .in_raw(file_id.as_bytes())
        .out_buffer(filebuf, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(LoadAlbumFileError)?;

    Ok(*result.value::<u64>())
}

/// Deletes an album file (cmd 3).
pub(crate) fn delete_album_file(
    service: &Session,
    file_id: &AlbumFileId,
) -> Result<(), DispatchError> {
    dispatch_in_no_out(service, proto::DELETE_ALBUM_FILE, file_id)
}

/// Copies an album file to a different storage (cmd 4).
pub(crate) fn storage_copy_album_file(
    service: &Session,
    file_id: &AlbumFileId,
    dst_storage: u8,
) -> Result<(), DispatchError> {
    let input = StorageCopyAlbumFileIn {
        storage: dst_storage,
        _pad: [0; 7],
        file_id: *file_id,
    };
    dispatch_in_no_out(service, proto::STORAGE_COPY_ALBUM_FILE, &input)
}

/// Checks whether a storage is mounted (cmd 5).
pub(crate) fn is_album_mounted(service: &Session, storage: u8) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_in_out(service, proto::IS_ALBUM_MOUNTED, &storage)?;
    Ok(val != 0)
}

/// Gets album usage statistics (cmd 6).
pub(crate) fn get_album_usage(
    service: &Session,
    storage: u8,
) -> Result<AlbumUsage2, DispatchError> {
    dispatch_in_out::<u8, AlbumUsage2>(service, proto::GET_ALBUM_USAGE, &storage)
}

/// Gets the size of an album file (cmd 7).
pub(crate) fn get_album_file_size(
    service: &Session,
    file_id: &AlbumFileId,
) -> Result<u64, DispatchError> {
    dispatch_in_out::<AlbumFileId, u64>(service, proto::GET_ALBUM_FILE_SIZE, file_id)
}

/// Loads the thumbnail for an album file (cmd 8).
pub(crate) fn load_album_file_thumbnail(
    service: &Session,
    file_id: &AlbumFileId,
    image: &mut [u8],
) -> Result<u64, LoadAlbumFileError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::LOAD_ALBUM_FILE_THUMBNAIL)
        .in_raw(file_id.as_bytes())
        .out_buffer(image, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(LoadAlbumFileError)?;

    Ok(*result.value::<u64>())
}

/// Loads a screenshot image (cmds 9, 10). \[2.0.0+\]
pub(crate) fn load_album_screen_shot_image(
    service: &Session,
    cmd_id: u32,
    file_id: &AlbumFileId,
    image: &mut [u8],
    workbuf: &mut [u8],
) -> Result<LoadScreenShotOut, LoadScreenShotError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(file_id.as_bytes())
        .out_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .out_buffer(workbuf, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<LoadScreenShotOut>())
        .send(&mut ipc_buf)
        .map_err(LoadScreenShotError)?;

    Ok(*result.value::<LoadScreenShotOut>())
}

/// Gets an AlbumEntry from an ApplicationAlbumEntry (cmd 11). \[2.0.0+\]
pub(crate) fn get_album_entry_from_app_album_entry(
    service: &Session,
    application_entry: &ApplicationAlbumEntry,
    application_id: u64,
) -> Result<AlbumEntry, DispatchError> {
    let input = GetAlbumEntryFromAppEntryIn {
        application_entry: *application_entry,
        application_id,
    };
    dispatch_in_out::<GetAlbumEntryFromAppEntryIn, AlbumEntry>(
        service,
        proto::GET_ALBUM_ENTRY_FROM_APP_ALBUM_ENTRY,
        &input,
    )
}

/// Loads a screenshot image with decode options (cmds 12, 13). \[3.0.0+\]
pub(crate) fn load_album_screen_shot_image_ex(
    service: &Session,
    cmd_id: u32,
    file_id: &AlbumFileId,
    opts: &ScreenShotDecodeOption,
    image: &mut [u8],
    workbuf: &mut [u8],
) -> Result<LoadScreenShotOut, LoadScreenShotError> {
    /// Wire-layout input for screenshot commands with decode options.
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct LoadScreenShotExIn {
        file_id: AlbumFileId,
        opts: ScreenShotDecodeOption,
    }

    const_assert_eq!(size_of::<LoadScreenShotExIn>(), 0x38);

    let input = LoadScreenShotExIn {
        file_id: *file_id,
        opts: *opts,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .out_buffer(workbuf, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<LoadScreenShotOut>())
        .send(&mut ipc_buf)
        .map_err(LoadScreenShotError)?;

    Ok(*result.value::<LoadScreenShotOut>())
}

/// Loads a screenshot image with decode options and attributes (cmds 14, 1001). \[3.0.0+\]
pub(crate) fn load_album_screen_shot_image_ex0(
    service: &Session,
    cmd_id: u32,
    file_id: &AlbumFileId,
    opts: &ScreenShotDecodeOption,
    image: &mut [u8],
    workbuf: &mut [u8],
) -> Result<LoadScreenShotEx0Out, LoadScreenShotError> {
    /// Wire-layout input for screenshot commands with decode options.
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct LoadScreenShotExIn {
        file_id: AlbumFileId,
        opts: ScreenShotDecodeOption,
    }

    const_assert_eq!(size_of::<LoadScreenShotExIn>(), 0x38);

    let input = LoadScreenShotExIn {
        file_id: *file_id,
        opts: *opts,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .out_buffer(workbuf, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<LoadScreenShotEx0Out>())
        .send(&mut ipc_buf)
        .map_err(LoadScreenShotError)?;

    Ok(*result.value::<LoadScreenShotEx0Out>())
}

/// Gets album usage statistics, 3-slot (cmd 15). \[4.0.0+\]
pub(crate) fn get_album_usage3(
    service: &Session,
    storage: u8,
) -> Result<AlbumUsage3, DispatchError> {
    dispatch_in_out::<u8, AlbumUsage3>(service, proto::GET_ALBUM_USAGE3, &storage)
}

/// Gets the mount result for a storage (cmd 16). \[4.0.0+\]
pub(crate) fn get_album_mount_result(service: &Session, storage: u8) -> Result<(), DispatchError> {
    dispatch_in_no_out(service, proto::GET_ALBUM_MOUNT_RESULT, &storage)
}

/// Gets album usage statistics, 16-slot (cmd 17). \[4.0.0+\]
pub(crate) fn get_album_usage16(
    service: &Session,
    storage: u8,
    flags: u8,
    out: &mut AlbumUsage16,
) -> Result<(), GetAlbumUsage16Error> {
    let input = StorageFlagsIn {
        storage,
        _pad1: [0; 7],
        flags,
        _pad2: [0; 7],
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::GET_ALBUM_USAGE16)
        .in_raw(input.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(GetAlbumUsage16Error)
}

/// Gets the min/max applet ID range (cmd 18). \[6.0.0+\]
pub(crate) fn get_min_max_applet_id(
    service: &Session,
    app_ids: &mut [u64; 2],
) -> Result<GetMinMaxAppletIdOut, GetMinMaxAppletIdError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_MIN_MAX_APPLET_ID)
        .out_buffer(
            app_ids.as_mut_bytes(),
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE),
        )
        .out_size(size_of::<GetMinMaxAppletIdOut>())
        .send(&mut ipc_buf)
        .map_err(GetMinMaxAppletIdError)?;

    Ok(*result.value::<GetMinMaxAppletIdOut>())
}

/// Gets album file count filtered by type (cmd 100). \[5.0.0+\]
pub(crate) fn get_album_file_count_ex0(
    service: &Session,
    storage: u8,
    flags: u8,
) -> Result<u64, DispatchError> {
    let input = StorageFlagsIn {
        storage,
        _pad1: [0; 7],
        flags,
        _pad2: [0; 7],
    };
    dispatch_in_out::<StorageFlagsIn, u64>(service, proto::GET_ALBUM_FILE_COUNT_EX0, &input)
}

/// Gets album file list filtered by type (cmd 101). \[5.0.0+\]
pub(crate) fn get_album_file_list_ex0(
    service: &Session,
    storage: u8,
    flags: u8,
    entries: &mut [u8],
) -> Result<u64, GetAlbumFileListError> {
    let input = StorageFlagsIn {
        storage,
        _pad1: [0; 7],
        flags,
        _pad2: [0; 7],
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::GET_ALBUM_FILE_LIST_EX0)
        .in_raw(input.as_bytes())
        .out_buffer(entries, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(GetAlbumFileListError)?;

    Ok(*result.value::<u64>())
}

/// Gets the last overlay thumbnail (cmds 301, 302).
pub(crate) fn get_last_overlay_thumbnail(
    service: &Session,
    cmd_id: u32,
    image: &mut [u8],
) -> Result<GetLastOverlayThumbnailOut, GetOverlayThumbnailError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_buffer(image, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<GetLastOverlayThumbnailOut>())
        .send(&mut ipc_buf)
        .map_err(GetOverlayThumbnailError)?;

    Ok(*result.value::<GetLastOverlayThumbnailOut>())
}

/// Gets the auto-saving storage (cmd 401).
pub(crate) fn get_auto_saving_storage(service: &Session) -> Result<u8, DispatchError> {
    dispatch_out::<u8>(service, proto::GET_AUTO_SAVING_STORAGE)
}

/// Gets required storage space to copy all files (cmd 501).
pub(crate) fn get_required_storage_space_size_to_copy_all(
    service: &Session,
    dst_storage: u8,
    src_storage: u8,
) -> Result<u64, DispatchError> {
    let input = GetRequiredStorageSizeIn {
        dst_storage,
        src_storage,
    };
    dispatch_in_out::<GetRequiredStorageSizeIn, u64>(
        service,
        proto::GET_REQUIRED_STORAGE_SPACE_SIZE_TO_COPY_ALL,
        &input,
    )
}

/// Loads a screenshot image/thumbnail with full output (cmds 1002, 1003). \[4.0.0+\]
pub(crate) fn load_album_screen_shot_image_ex1(
    service: &Session,
    cmd_id: u32,
    file_id: &AlbumFileId,
    opts: &ScreenShotDecodeOption,
    out: &mut LoadAlbumScreenShotImageOutput,
    image: &mut [u8],
    workbuf: &mut [u8],
) -> Result<(), LoadScreenShotError> {
    /// Wire-layout input for screenshot commands with decode options.
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct LoadScreenShotExIn {
        file_id: AlbumFileId,
        opts: ScreenShotDecodeOption,
    }

    const_assert_eq!(size_of::<LoadScreenShotExIn>(), 0x38);

    let input = LoadScreenShotExIn {
        file_id: *file_id,
        opts: *opts,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::HIPC_MAP_ALIAS.or(BufferAttr::FIXED_SIZE),
        )
        .out_buffer(
            image,
            BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .out_buffer(workbuf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(LoadScreenShotError)
}

/// Force-unmounts a storage (cmd 8001).
pub(crate) fn force_album_unmounted(service: &Session, storage: u8) -> Result<(), DispatchError> {
    dispatch_in_no_out(service, proto::FORCE_ALBUM_UNMOUNTED, &storage)
}

/// Resets album mount status (cmd 8002).
pub(crate) fn reset_album_mount_status(
    service: &Session,
    storage: u8,
) -> Result<(), DispatchError> {
    dispatch_in_no_out(service, proto::RESET_ALBUM_MOUNT_STATUS, &storage)
}

/// Refreshes album cache (cmd 8011).
pub(crate) fn refresh_album_cache(service: &Session, storage: u8) -> Result<(), DispatchError> {
    dispatch_in_no_out(service, proto::REFRESH_ALBUM_CACHE, &storage)
}

/// Gets album cache (cmd 8012).
pub(crate) fn get_album_cache(service: &Session, storage: u8) -> Result<AlbumCache, DispatchError> {
    dispatch_in_out::<u8, AlbumCache>(service, proto::GET_ALBUM_CACHE, &storage)
}

/// Gets album cache by content type (cmd 8013). \[4.0.0+\]
pub(crate) fn get_album_cache_ex(
    service: &Session,
    storage: u8,
    contents: u8,
) -> Result<AlbumCache, DispatchError> {
    let input = GetAlbumCacheExIn { storage, contents };
    dispatch_in_out::<GetAlbumCacheExIn, AlbumCache>(service, proto::GET_ALBUM_CACHE_EX, &input)
}

/// Gets an AlbumEntry from an ApplicationAlbumEntry with ARUID (cmd 8021). \[2.0.0+\]
pub(crate) fn get_album_entry_from_app_album_entry_aruid(
    service: &Session,
    application_entry: &ApplicationAlbumEntry,
    aruid: u64,
) -> Result<AlbumEntry, DispatchError> {
    let input = GetAlbumEntryFromAppEntryAruidIn {
        application_entry: *application_entry,
        aruid,
    };
    dispatch_in_pid_out::<GetAlbumEntryFromAppEntryAruidIn, AlbumEntry>(
        service,
        proto::GET_ALBUM_ENTRY_FROM_APP_ALBUM_ENTRY_ARUID,
        &input,
    )
}

/// Opens an accessor session (cmd 60002). Returns the move handle.
pub(crate) fn open_accessor_session(
    service: &Session,
    aruid: u64,
) -> Result<u32, OpenAccessorSessionError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::OPEN_ACCESSOR_SESSION)
        .in_raw(aruid.as_bytes())
        .send_pid()
        .send(&mut ipc_buf)
        .map_err(OpenAccessorSessionError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenAccessorSessionError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// Opens an album movie read stream (cmd 2001).
pub(crate) fn open_album_movie_read_stream(
    service: &Session,
    file_id: &AlbumFileId,
) -> Result<u64, DispatchError> {
    dispatch_in_out::<AlbumFileId, u64>(service, proto::OPEN_ALBUM_MOVIE_READ_STREAM, file_id)
}

/// Closes an album movie stream (cmd 2002).
pub(crate) fn close_album_movie_stream(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_no_out(service, proto::CLOSE_ALBUM_MOVIE_STREAM, &stream)
}

/// Gets the size of a movie stream (cmd 2003).
pub(crate) fn get_album_movie_stream_size(
    service: &Session,
    stream: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_out::<u64, u64>(service, proto::GET_ALBUM_MOVIE_STREAM_SIZE, &stream)
}

/// Reads movie data from a read stream (cmd 2004).
pub(crate) fn read_movie_data_from_stream(
    service: &Session,
    stream: u64,
    offset: i64,
    buffer: &mut [u8],
) -> Result<u64, ReadStreamDataError> {
    let input = ReadStreamIn { stream, offset };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::READ_MOVIE_DATA_FROM_STREAM)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(ReadStreamDataError)?;

    Ok(*result.value::<u64>())
}

/// Gets the broken reason for a read stream (cmd 2005).
pub(crate) fn get_album_movie_read_stream_broken_reason(
    service: &Session,
    stream: u64,
) -> Result<(), DispatchError> {
    dispatch_in_no_out(
        service,
        proto::GET_ALBUM_MOVIE_READ_STREAM_BROKEN_REASON,
        &stream,
    )
}

/// Gets the image data size of a read stream (cmd 2006).
pub(crate) fn get_album_movie_read_stream_image_data_size(
    service: &Session,
    stream: u64,
) -> Result<u64, DispatchError> {
    dispatch_in_out::<u64, u64>(
        service,
        proto::GET_ALBUM_MOVIE_READ_STREAM_IMAGE_DATA_SIZE,
        &stream,
    )
}

/// Reads image data from a read stream (cmd 2007).
pub(crate) fn read_image_data_from_stream(
    service: &Session,
    stream: u64,
    offset: i64,
    buffer: &mut [u8],
) -> Result<u64, ReadStreamDataError> {
    let input = ReadStreamIn { stream, offset };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::READ_IMAGE_DATA_FROM_STREAM)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(ReadStreamDataError)?;

    Ok(*result.value::<u64>())
}

/// Reads file attributes from a read stream (cmd 2008).
pub(crate) fn read_file_attribute_from_stream(
    service: &Session,
    stream: u64,
) -> Result<ScreenShotAttribute, DispatchError> {
    dispatch_in_out::<u64, ScreenShotAttribute>(
        service,
        proto::READ_FILE_ATTRIBUTE_FROM_STREAM,
        &stream,
    )
}

/// Error returned by album file list operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get album file list")]
pub struct GetAlbumFileListError(#[source] pub DispatchError);

/// Error returned by album file load operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to load album file")]
pub struct LoadAlbumFileError(#[source] pub DispatchError);

/// Error returned by screenshot load operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to load album screenshot")]
pub struct LoadScreenShotError(#[source] pub DispatchError);

/// Error returned by [`get_album_usage16`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get album usage")]
pub struct GetAlbumUsage16Error(#[source] pub DispatchError);

/// Error returned by [`get_min_max_applet_id`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get min/max applet ID")]
pub struct GetMinMaxAppletIdError(#[source] pub DispatchError);

/// Error returned by overlay thumbnail operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get overlay thumbnail")]
pub struct GetOverlayThumbnailError(#[source] pub DispatchError);

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

/// Error returned by stream read operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to read stream data")]
pub struct ReadStreamDataError(#[source] pub DispatchError);
