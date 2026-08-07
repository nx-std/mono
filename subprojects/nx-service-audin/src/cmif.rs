//! CMIF protocol operations for the audio input service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    OutHandleAttr,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::dispatch_no_io,
    proto,
    types::{
        AudioInBuffer,
        OpenAudioInIn,
        OpenAudioInOut,
    },
};

/// Lists available audio input devices (auto-select). \[3.0.0+\]
///
/// Returns the number of device names written to `device_names_buf`.
pub(crate) fn list_audio_ins(
    service: &Session,
    device_names_buf: &mut [u8],
) -> Result<u32, ListAudioInsError> {
    list_audio_ins_impl(
        service,
        proto::LIST_AUDIO_INS,
        device_names_buf,
        BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
    )
}

/// Lists available audio input devices (map-alias, legacy). \[1.0.0-2.x.x\]
///
/// Returns the number of device names written to `device_names_buf`.
pub(crate) fn list_audio_ins_legacy(
    service: &Session,
    device_names_buf: &mut [u8],
) -> Result<u32, ListAudioInsError> {
    list_audio_ins_impl(
        service,
        proto::LIST_AUDIO_INS_LEGACY,
        device_names_buf,
        BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
    )
}

fn list_audio_ins_impl(
    service: &Session,
    cmd_id: u32,
    device_names_buf: &mut [u8],
    buffer_attr: BufferAttr,
) -> Result<u32, ListAudioInsError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u32>())
        .out_buffer(device_names_buf, buffer_attr)
        .send(&mut buf)
        .map_err(ListAudioInsError)?;

    Ok(*result.value::<u32>())
}

/// Opens an audio input device (auto-select). \[3.0.0+\]
///
/// Returns the IAudioIn session handle (move handle) and the negotiated
/// output parameters.
pub(crate) fn open_audio_in(
    service: &Session,
    input: &OpenAudioInIn,
    device_name_in: &[u8],
    device_name_out: &mut [u8],
) -> Result<(u32, OpenAudioInOut), OpenAudioInError> {
    open_audio_in_impl(
        service,
        proto::OPEN_AUDIO_IN,
        input,
        device_name_in,
        device_name_out,
        BufferAttr::HIPC_AUTO_SELECT,
    )
}

/// Opens an audio input device (map-alias, legacy). \[1.0.0-2.x.x\]
///
/// Returns the IAudioIn session handle (move handle) and the negotiated
/// output parameters.
pub(crate) fn open_audio_in_legacy(
    service: &Session,
    input: &OpenAudioInIn,
    device_name_in: &[u8],
    device_name_out: &mut [u8],
) -> Result<(u32, OpenAudioInOut), OpenAudioInError> {
    open_audio_in_impl(
        service,
        proto::OPEN_AUDIO_IN_LEGACY,
        input,
        device_name_in,
        device_name_out,
        BufferAttr::HIPC_MAP_ALIAS,
    )
}

fn open_audio_in_impl(
    service: &Session,
    cmd_id: u32,
    input: &OpenAudioInIn,
    device_name_in: &[u8],
    device_name_out: &mut [u8],
    transfer_attr: BufferAttr,
) -> Result<(u32, OpenAudioInOut), OpenAudioInError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .in_handle(nx_svc::raw::CUR_PROCESS_HANDLE)
        .send_pid()
        .in_buffer(device_name_in, transfer_attr)
        .out_buffer(device_name_out, transfer_attr)
        .out_size(size_of::<OpenAudioInOut>())
        .send(&mut buf)
        .map_err(OpenAudioInError::Dispatch)?;

    let Some(&handle) = result.move_handles.first() else {
        return Err(OpenAudioInError::MissingHandle);
    };

    Ok((handle, *result.value::<OpenAudioInOut>()))
}

/// Gets the current audio input state.
pub(crate) fn audio_in_get_state(service: &Session) -> Result<u32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::AUDIO_IN_GET_STATE)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;

    Ok(*result.value::<u32>())
}

/// Starts audio input capture.
pub(crate) fn audio_in_start(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::AUDIO_IN_START)
}

/// Stops audio input capture.
pub(crate) fn audio_in_stop(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::AUDIO_IN_STOP)
}

/// Registers the buffer event (returns copy handle).
pub(crate) fn audio_in_register_buffer_event(service: &Session) -> Result<u32, DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::AUDIO_IN_REGISTER_BUFFER_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)?;

    Ok(result.copy_handles[0])
}

/// Appends an audio input buffer (auto-select). \[3.0.0+\]
pub(crate) fn audio_in_append_buffer(
    service: &Session,
    buffer_client_ptr: u64,
    buffer: &AudioInBuffer,
) -> Result<(), AppendBufferError> {
    append_buffer_impl(
        service,
        proto::AUDIO_IN_APPEND_BUFFER,
        buffer_client_ptr,
        buffer,
        BufferAttr::IN.or(BufferAttr::HIPC_AUTO_SELECT),
    )
}

/// Appends an audio input buffer (map-alias, legacy). \[1.0.0-2.x.x\]
pub(crate) fn audio_in_append_buffer_legacy(
    service: &Session,
    buffer_client_ptr: u64,
    buffer: &AudioInBuffer,
) -> Result<(), AppendBufferError> {
    append_buffer_impl(
        service,
        proto::AUDIO_IN_APPEND_BUFFER_LEGACY,
        buffer_client_ptr,
        buffer,
        BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
    )
}

fn append_buffer_impl(
    service: &Session,
    cmd_id: u32,
    buffer_client_ptr: u64,
    buffer: &AudioInBuffer,
    buffer_attr: BufferAttr,
) -> Result<(), AppendBufferError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(cmd_id)
        .in_raw(buffer_client_ptr.as_bytes())
        .in_buffer(buffer.as_bytes(), buffer_attr)
        .send(&mut buf)
        .map(|_| ())
        .map_err(AppendBufferError)
}

/// Gets a released audio input buffer (auto-select). \[3.0.0+\]
///
/// Writes the released buffer's client-side pointer to `out_buffer_ptr`.
/// Returns the number of released buffers.
pub(crate) fn audio_in_get_released_buffer(
    service: &Session,
    out_buffer_ptr: &mut u64,
) -> Result<u32, GetReleasedBufferError> {
    get_released_buffer_impl(
        service,
        proto::AUDIO_IN_GET_RELEASED_BUFFER,
        out_buffer_ptr,
        BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
    )
}

/// Gets a released audio input buffer (map-alias, legacy). \[1.0.0-2.x.x\]
///
/// Writes the released buffer's client-side pointer to `out_buffer_ptr`.
/// Returns the number of released buffers.
pub(crate) fn audio_in_get_released_buffer_legacy(
    service: &Session,
    out_buffer_ptr: &mut u64,
) -> Result<u32, GetReleasedBufferError> {
    get_released_buffer_impl(
        service,
        proto::AUDIO_IN_GET_RELEASED_BUFFER_LEGACY,
        out_buffer_ptr,
        BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
    )
}

fn get_released_buffer_impl(
    service: &Session,
    cmd_id: u32,
    out_buffer_ptr: &mut u64,
    buffer_attr: BufferAttr,
) -> Result<u32, GetReleasedBufferError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u32>())
        .out_buffer(out_buffer_ptr.as_mut_bytes(), buffer_attr)
        .send(&mut buf)
        .map_err(GetReleasedBufferError)?;

    Ok(*result.value::<u32>())
}

/// Checks whether a buffer is contained in the audio input.
pub(crate) fn audio_in_contains_buffer(
    service: &Session,
    buffer_client_ptr: u64,
) -> Result<bool, ContainsBufferError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::AUDIO_IN_CONTAINS_BUFFER)
        .in_raw(buffer_client_ptr.as_bytes())
        .out_size(size_of::<u8>())
        .send(&mut buf)
        .map_err(ContainsBufferError)?;

    Ok(*result.value::<u8>() & 1 != 0)
}

/// Error returned by list-audio-ins operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to list audio input devices")]
pub struct ListAudioInsError(#[source] pub DispatchError);

/// Error returned by open-audio-in operations.
#[derive(Debug, thiserror::Error)]
pub enum OpenAudioInError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenAudioIn")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenAudioIn response missing move handle")]
    MissingHandle,
}

/// Error returned by append-buffer operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to append audio input buffer")]
pub struct AppendBufferError(#[source] pub DispatchError);

/// Error returned by get-released-buffer operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get released audio input buffer")]
pub struct GetReleasedBufferError(#[source] pub DispatchError);

/// Error returned by contains-buffer operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to check audio input buffer containment")]
pub struct ContainsBufferError(#[source] pub DispatchError);
