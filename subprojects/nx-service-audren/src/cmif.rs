//! CMIF protocol operations for the audio renderer service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::dispatch_no_io,
    proto,
    types::{AudioRendererParameter, OpenAudioRendererIn},
};

// ---------------------------------------------------------------------------
// IAudioRendererManager commands
// ---------------------------------------------------------------------------

/// Opens an audio renderer (cmd 0).
///
/// Sends PID + two copy handles (transfer-memory handle and process handle).
/// Returns the IAudioRenderer session as a move handle.
pub(crate) fn open_audio_renderer(
    service: &Session,
    param: &AudioRendererParameter,
    work_buffer_size: u64,
    aruid: u64,
    tmem_handle: u32,
    process_handle: u32,
) -> Result<u32, OpenAudioRendererError> {
    let input = OpenAudioRendererIn {
        param: *param,
        _pad: 0,
        work_buffer_size,
        aruid,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<OpenAudioRendererIn>()` bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<OpenAudioRendererIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::OPEN_AUDIO_RENDERER)
        .in_raw(in_bytes)
        .send_pid()
        .in_handle(tmem_handle)
        .in_handle(process_handle)
        .send(&mut ipc_buf)
        .map_err(OpenAudioRendererError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenAudioRendererError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// Gets the required work buffer size for the given parameters (cmd 1).
pub(crate) fn get_work_buffer_size(
    service: &Session,
    param: &AudioRendererParameter,
) -> Result<u64, GetWorkBufferSizeError> {
    // SAFETY: `param` is a valid `&AudioRendererParameter`; viewing its bytes
    // as a slice is sound, and the slice borrows `param`.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (param as *const AudioRendererParameter).cast::<u8>(),
            size_of::<AudioRendererParameter>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_WORK_BUFFER_SIZE)
        .in_raw(in_bytes)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(GetWorkBufferSizeError)?;

    // SAFETY: response payload is at least size_of::<u64>().
    let size = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(size)
}

// ---------------------------------------------------------------------------
// IAudioRenderer commands
// ---------------------------------------------------------------------------

/// Gets the current renderer state (cmd 3).
pub(crate) fn renderer_get_state(service: &Session) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::RENDERER_GET_STATE)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let state = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(state)
}

/// Requests update of the audio renderer (auto-select). \[3.0.0+\]
pub(crate) fn renderer_request_update(
    service: &Session,
    in_param_buf: &[u8],
    out_param_buf: &mut [u8],
    perf_buf: &mut [u8],
) -> Result<(), RequestUpdateError> {
    renderer_request_update_impl(
        service,
        proto::RENDERER_REQUEST_UPDATE,
        in_param_buf,
        out_param_buf,
        perf_buf,
        BufferAttr::HIPC_AUTO_SELECT,
    )
}

/// Requests update of the audio renderer (map-alias, legacy). \[1.0.0-2.x.x\]
pub(crate) fn renderer_request_update_legacy(
    service: &Session,
    in_param_buf: &[u8],
    out_param_buf: &mut [u8],
    perf_buf: &mut [u8],
) -> Result<(), RequestUpdateError> {
    renderer_request_update_impl(
        service,
        proto::RENDERER_REQUEST_UPDATE_LEGACY,
        in_param_buf,
        out_param_buf,
        perf_buf,
        BufferAttr::HIPC_MAP_ALIAS,
    )
}

fn renderer_request_update_impl(
    service: &Session,
    cmd_id: u32,
    in_param_buf: &[u8],
    out_param_buf: &mut [u8],
    perf_buf: &mut [u8],
    transfer_attr: BufferAttr,
) -> Result<(), RequestUpdateError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(cmd_id)
        .out_buffer(out_param_buf, transfer_attr)
        .out_buffer(perf_buf, transfer_attr)
        .in_buffer(in_param_buf, transfer_attr)
        .send(&mut ipc_buf)
        .map(|_| ())
        .map_err(RequestUpdateError)
}

/// Starts the audio renderer (cmd 5).
pub(crate) fn renderer_start(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::RENDERER_START)
}

/// Stops the audio renderer (cmd 6).
pub(crate) fn renderer_stop(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::RENDERER_STOP)
}

/// Queries the system event (cmd 7, copy handle output).
pub(crate) fn renderer_query_system_event(service: &Session) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::RENDERER_QUERY_SYSTEM_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;

    Ok(result.copy_handles[0])
}

/// Sets the rendering time limit as a percentage (cmd 8).
pub(crate) fn renderer_set_rendering_time_limit(
    service: &Session,
    percent: i32,
) -> Result<(), DispatchError> {
    // SAFETY: `percent` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<i32>()` bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const percent).cast::<u8>(), size_of::<i32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    service
        .dispatch(proto::RENDERER_SET_RENDERING_TIME_LIMIT)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by [`open_audio_renderer`].
#[derive(Debug, thiserror::Error)]
pub enum OpenAudioRendererError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenAudioRenderer")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenAudioRenderer response missing move handle")]
    MissingHandle,
}

/// Error returned by [`get_work_buffer_size`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get audio renderer work buffer size")]
pub struct GetWorkBufferSizeError(#[source] pub DispatchError);

/// Error returned by request-update operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to request audio renderer update")]
pub struct RequestUpdateError(#[source] pub DispatchError);
