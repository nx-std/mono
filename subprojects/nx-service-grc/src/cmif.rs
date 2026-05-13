//! CMIF protocol operations for the GRC game recording service.

use core::{mem::size_of, ptr};

use nx_service_caps::ApplicationAlbumEntry;
use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::{dispatch_in_u64, dispatch_in_u64_out_u32, dispatch_no_io},
    proto,
    types::{
        BeginTrimIn, CompleteFinishIn, GameMovieId, OffscreenRecordingParameter, SetThumbnailIn,
        StartRecordingIn, TransferResult,
    },
};

// ---------------------------------------------------------------------------
// grc:d commands
// ---------------------------------------------------------------------------

/// Begins streaming (cmd 1).
pub(crate) fn begin(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::BEGIN)
}

/// Retrieves stream data from the continuous recorder (cmd 2).
pub(crate) fn transfer(
    service: &Session,
    stream: u32,
    buffer: *mut u8,
    buffer_len: usize,
) -> Result<TransferResult, TransferError> {
    let result = unsafe {
        service
            .dispatch(proto::TRANSFER)
            .in_raw((&raw const stream).cast::<u8>(), size_of::<u32>())
            .out_size(size_of::<TransferResult>())
            .buffer(
                buffer,
                buffer_len,
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map_err(TransferError)?
    };

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<TransferResult>()) })
}

// ---------------------------------------------------------------------------
// IGameMovieTrimmer commands
// ---------------------------------------------------------------------------

/// Begins trimming a game movie (cmd 1).
pub(crate) fn trimmer_begin_trim(
    service: &Session,
    id: &GameMovieId,
    start: i32,
    end: i32,
) -> Result<(), DispatchError> {
    let input = BeginTrimIn {
        start,
        end,
        id: *id,
    };

    unsafe {
        service
            .dispatch(proto::TRIMMER_BEGIN_TRIM)
            .in_raw((&raw const input).cast::<u8>(), size_of::<BeginTrimIn>())
            .send()
            .map(|_| ())
    }
}

/// Ends trimming and retrieves the output movie ID (cmd 2).
pub(crate) fn trimmer_end_trim(service: &Session) -> Result<GameMovieId, DispatchError> {
    let result = service
        .dispatch(proto::TRIMMER_END_TRIM)
        .out_size(size_of::<GameMovieId>())
        .send()?;

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<GameMovieId>()) })
}

/// Gets the "not trimming" event (cmd 10, copy handle).
pub(crate) fn trimmer_get_not_trimming_event(service: &Session) -> Result<u32, DispatchError> {
    let result = service
        .dispatch(proto::TRIMMER_GET_NOT_TRIMMING_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;

    Ok(result.copy_handles[0])
}

/// Sets the thumbnail RGBA image for the trimmed movie (cmd 20).
pub(crate) fn trimmer_set_thumbnail_rgba(
    service: &Session,
    buffer: *const u8,
    buffer_len: usize,
    width: i32,
    height: i32,
) -> Result<(), DispatchError> {
    let input = SetThumbnailIn { width, height };

    unsafe {
        service
            .dispatch(proto::TRIMMER_SET_THUMBNAIL_RGBA)
            .in_raw((&raw const input).cast::<u8>(), size_of::<SetThumbnailIn>())
            .buffer(
                buffer.cast_mut(),
                buffer_len,
                BufferAttr::IN
                    .or(BufferAttr::MAP_TRANSFER_ALLOWS_NON_SECURE)
                    .or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

// ---------------------------------------------------------------------------
// IMovieMaker commands
// ---------------------------------------------------------------------------

/// Creates a video proxy sub-object (cmd 2). Returns the move handle.
pub(crate) fn maker_create_video_proxy(service: &Session) -> Result<u32, CreateVideoProxyError> {
    let result = service
        .dispatch(proto::MAKER_CREATE_VIDEO_PROXY)
        .send()
        .map_err(CreateVideoProxyError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(CreateVideoProxyError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// Sets the album shim library version (cmd 9). \[7.0.0+\]
pub(crate) fn maker_set_album_shim_library_version(
    service: &Session,
    version: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64(
        service,
        proto::MAKER_SET_ALBUM_SHIM_LIBRARY_VERSION,
        version,
    )
}

/// Opens an offscreen layer (cmd 10). Returns the binder ID.
pub(crate) fn maker_open_offscreen_layer(
    service: &Session,
    layer_handle: u64,
) -> Result<u32, DispatchError> {
    dispatch_in_u64_out_u32(service, proto::MAKER_OPEN_OFFSCREEN_LAYER, layer_handle)
}

/// Closes an offscreen layer (cmd 11).
pub(crate) fn maker_close_offscreen_layer(
    service: &Session,
    layer_handle: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64(service, proto::MAKER_CLOSE_OFFSCREEN_LAYER, layer_handle)
}

/// Aborts offscreen recording (cmd 21).
pub(crate) fn maker_abort_offscreen_recording(
    service: &Session,
    layer_handle: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64(
        service,
        proto::MAKER_ABORT_OFFSCREEN_RECORDING,
        layer_handle,
    )
}

/// Requests offscreen recording finish ready (cmd 22).
pub(crate) fn maker_request_offscreen_recording_finish_ready(
    service: &Session,
    layer_handle: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64(
        service,
        proto::MAKER_REQUEST_OFFSCREEN_RECORDING_FINISH_READY,
        layer_handle,
    )
}

/// Starts offscreen recording (cmd 24).
pub(crate) fn maker_start_offscreen_recording(
    service: &Session,
    layer_handle: u64,
    param: &OffscreenRecordingParameter,
) -> Result<(), DispatchError> {
    let input = StartRecordingIn {
        layer_handle,
        param: *param,
    };

    unsafe {
        service
            .dispatch(proto::MAKER_START_OFFSCREEN_RECORDING)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<StartRecordingIn>(),
            )
            .send()
            .map(|_| ())
    }
}

/// Completes offscreen recording finish (pre-7.0.0, cmd 25).
#[expect(clippy::too_many_arguments)]
pub(crate) fn maker_complete_offscreen_recording_finish_ex0(
    service: &Session,
    layer_handle: u64,
    width: i32,
    height: i32,
    userdata: *const u8,
    userdata_len: usize,
    thumbnail: *const u8,
    thumbnail_len: usize,
) -> Result<(), DispatchError> {
    let input = CompleteFinishIn {
        width,
        height,
        layer_handle,
    };

    unsafe {
        service
            .dispatch(proto::MAKER_COMPLETE_OFFSCREEN_RECORDING_FINISH_EX0)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<CompleteFinishIn>(),
            )
            .buffer(
                userdata.cast_mut(),
                userdata_len,
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .buffer(
                thumbnail.cast_mut(),
                thumbnail_len,
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// Completes offscreen recording finish (7.0.0+, cmd 26). Returns album entry.
#[expect(clippy::too_many_arguments)]
pub(crate) fn maker_complete_offscreen_recording_finish_ex1(
    service: &Session,
    layer_handle: u64,
    width: i32,
    height: i32,
    userdata: *const u8,
    userdata_len: usize,
    thumbnail: *const u8,
    thumbnail_len: usize,
) -> Result<ApplicationAlbumEntry, CompleteFinishEx1Error> {
    let input = CompleteFinishIn {
        width,
        height,
        layer_handle,
    };

    let result = unsafe {
        service
            .dispatch(proto::MAKER_COMPLETE_OFFSCREEN_RECORDING_FINISH_EX1)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<CompleteFinishIn>(),
            )
            .out_size(size_of::<ApplicationAlbumEntry>())
            .buffer(
                userdata.cast_mut(),
                userdata_len,
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .buffer(
                thumbnail.cast_mut(),
                thumbnail_len,
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map_err(CompleteFinishEx1Error)?
    };

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<ApplicationAlbumEntry>()) })
}

/// Gets the offscreen layer error (cmd 30).
pub(crate) fn maker_get_offscreen_layer_error(
    service: &Session,
    layer_handle: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64(
        service,
        proto::MAKER_GET_OFFSCREEN_LAYER_ERROR,
        layer_handle,
    )
}

/// Encodes offscreen layer audio sample data (cmd 41).
/// Returns the number of bytes consumed.
pub(crate) fn maker_encode_offscreen_layer_audio_sample(
    service: &Session,
    layer_handle: u64,
    buffer: *const u8,
    buffer_len: usize,
) -> Result<u64, DispatchError> {
    let result = unsafe {
        service
            .dispatch(proto::MAKER_ENCODE_OFFSCREEN_LAYER_AUDIO_SAMPLE)
            .in_raw((&raw const layer_handle).cast::<u8>(), size_of::<u64>())
            .out_size(size_of::<u64>())
            .buffer(
                buffer.cast_mut(),
                buffer_len,
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()?
    };

    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) })
}

/// Gets the offscreen layer recording finish ready event (cmd 50, copy handle).
pub(crate) fn maker_get_offscreen_layer_recording_finish_ready_event(
    service: &Session,
    layer_handle: u64,
) -> Result<u32, DispatchError> {
    let result = unsafe {
        service
            .dispatch(proto::MAKER_GET_OFFSCREEN_LAYER_RECORDING_FINISH_READY_EVENT)
            .in_raw((&raw const layer_handle).cast::<u8>(), size_of::<u64>())
            .out_handle(0, OutHandleAttr::Copy)
            .send()?
    };

    Ok(result.copy_handles[0])
}

/// Gets the offscreen layer audio encode ready event (cmd 52, copy handle).
pub(crate) fn maker_get_offscreen_layer_audio_encode_ready_event(
    service: &Session,
    layer_handle: u64,
) -> Result<u32, DispatchError> {
    let result = unsafe {
        service
            .dispatch(proto::MAKER_GET_OFFSCREEN_LAYER_AUDIO_ENCODE_READY_EVENT)
            .in_raw((&raw const layer_handle).cast::<u8>(), size_of::<u64>())
            .out_handle(0, OutHandleAttr::Copy)
            .send()?
    };

    Ok(result.copy_handles[0])
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by [`transfer`].
#[derive(Debug, thiserror::Error)]
#[error("failed to transfer stream data")]
pub struct TransferError(#[source] pub DispatchError);

/// Error returned by [`maker_create_video_proxy`].
#[derive(Debug, thiserror::Error)]
pub enum CreateVideoProxyError {
    /// IPC dispatch failed.
    #[error("failed to dispatch CreateVideoProxy")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("CreateVideoProxy response missing move handle")]
    MissingHandle,
}

/// Error returned by [`maker_complete_offscreen_recording_finish_ex1`].
#[derive(Debug, thiserror::Error)]
#[error("failed to complete offscreen recording finish")]
pub struct CompleteFinishEx1Error(#[source] pub DispatchError);
