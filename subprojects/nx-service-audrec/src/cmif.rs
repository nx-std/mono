//! CMIF protocol operations for the audio recorder service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::dispatch_no_io,
    proto,
    types::{
        FinalOutputRecorderBuffer, FinalOutputRecorderParameterInternal, GetReleasedBuffersOut,
        OpenRecorderIn,
    },
};

/// Opens a final output recorder sub-object on the root service.
///
/// Returns the recorder session handle (move handle) and the internal
/// parameters negotiated by the server.
pub(crate) fn open_final_output_recorder(
    service: &Session,
    input: &OpenRecorderIn,
) -> Result<(u32, FinalOutputRecorderParameterInternal), OpenRecorderError> {
    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::OPEN_FINAL_OUTPUT_RECORDER)
            .in_raw(
                (&raw const *input).cast::<u8>(),
                size_of::<OpenRecorderIn>(),
            )
            .in_handle(nx_svc::raw::CUR_PROCESS_HANDLE)
            .out_size(size_of::<FinalOutputRecorderParameterInternal>())
            .send()
            .map_err(OpenRecorderError::Dispatch)?
    };

    if result.move_handles.is_empty() {
        return Err(OpenRecorderError::MissingHandle);
    }

    // SAFETY: response payload is at least size_of::<FinalOutputRecorderParameterInternal>().
    let param_out = unsafe {
        ptr::read_unaligned(
            result
                .data
                .as_ptr()
                .cast::<FinalOutputRecorderParameterInternal>(),
        )
    };

    Ok((result.move_handles[0], param_out))
}

/// Starts the recorder.
pub(crate) fn recorder_start(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::RECORDER_START)
}

/// Stops the recorder.
pub(crate) fn recorder_stop(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::RECORDER_STOP)
}

/// Registers the buffer event (returns copy handle).
pub(crate) fn recorder_register_buffer_event(service: &Session) -> Result<u32, DispatchError> {
    let result = service
        .dispatch(proto::RECORDER_REGISTER_BUFFER_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;

    Ok(result.copy_handles[0])
}

/// Appends a final output recorder buffer (auto-select, [3.0.0+]).
pub(crate) fn recorder_append_buffer(
    service: &Session,
    buffer_client_ptr: u64,
    param: &FinalOutputRecorderBuffer,
) -> Result<(), AppendBufferError> {
    // SAFETY: `buffer_client_ptr` and `param` live on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::RECORDER_APPEND_BUFFER)
            .in_raw(
                (&raw const buffer_client_ptr).cast::<u8>(),
                size_of::<u64>(),
            )
            .buffer(
                (&raw const *param).cast::<u8>(),
                size_of::<FinalOutputRecorderBuffer>(),
                BufferAttr::IN.or(BufferAttr::HIPC_AUTO_SELECT),
            )
            .send()
            .map(|_| ())
            .map_err(AppendBufferError)
    }
}

/// Appends a final output recorder buffer (map-alias, legacy [1.0.0-2.x.x]).
pub(crate) fn recorder_append_buffer_legacy(
    service: &Session,
    buffer_client_ptr: u64,
    param: &FinalOutputRecorderBuffer,
) -> Result<(), AppendBufferError> {
    // SAFETY: `buffer_client_ptr` and `param` live on the stack until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::RECORDER_APPEND_BUFFER_LEGACY)
            .in_raw(
                (&raw const buffer_client_ptr).cast::<u8>(),
                size_of::<u64>(),
            )
            .buffer(
                (&raw const *param).cast::<u8>(),
                size_of::<FinalOutputRecorderBuffer>(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
            .map_err(AppendBufferError)
    }
}

/// Gets released final output recorder buffers (auto-select, [3.0.0+]).
///
/// Returns `(count, released)` where `count` is the number of buffer
/// pointers written to `out_buffers` and `released` is a release counter.
pub(crate) fn recorder_get_released_buffers(
    service: &Session,
    out_buffers: &mut [u64],
) -> Result<(u32, u64), GetReleasedBuffersError> {
    get_released_impl(
        service,
        proto::RECORDER_GET_RELEASED_BUFFERS,
        out_buffers,
        BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
    )
}

/// Gets released final output recorder buffers (map-alias, legacy [1.0.0-2.x.x]).
///
/// Returns `(count, released)` where `count` is the number of buffer
/// pointers written to `out_buffers` and `released` is a release counter.
pub(crate) fn recorder_get_released_buffers_legacy(
    service: &Session,
    out_buffers: &mut [u64],
) -> Result<(u32, u64), GetReleasedBuffersError> {
    get_released_impl(
        service,
        proto::RECORDER_GET_RELEASED_BUFFERS_LEGACY,
        out_buffers,
        BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
    )
}

fn get_released_impl(
    service: &Session,
    cmd_id: u32,
    out_buffers: &mut [u64],
    buffer_attr: BufferAttr,
) -> Result<(u32, u64), GetReleasedBuffersError> {
    let buf_size = core::mem::size_of_val(out_buffers);

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<GetReleasedBuffersOut>())
        .buffer(out_buffers.as_mut_ptr().cast::<u8>(), buf_size, buffer_attr)
        .send()
        .map_err(GetReleasedBuffersError)?;

    // SAFETY: response payload is at least size_of::<GetReleasedBuffersOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<GetReleasedBuffersOut>()) };

    Ok((out.count, out.released))
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by [`open_final_output_recorder`].
#[derive(Debug, thiserror::Error)]
pub enum OpenRecorderError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenFinalOutputRecorder")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenFinalOutputRecorder response missing move handle")]
    MissingHandle,
}

/// Error returned by append-buffer operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to append final output recorder buffer")]
pub struct AppendBufferError(#[source] pub DispatchError);

/// Error returned by get-released-buffers operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get released final output recorder buffers")]
pub struct GetReleasedBuffersError(#[source] pub DispatchError);
