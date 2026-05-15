//! CMIF protocol operations for the audio output service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::dispatch_no_io,
    proto,
    types::{AudioOutBuffer, OpenAudioOutIn, OpenAudioOutOut, PidDelayIn, SetVolumeIn},
};

// ---------------------------------------------------------------------------
// Root service commands (IAudioOutManager)
// ---------------------------------------------------------------------------

/// Lists available audio output devices (auto-select). \[3.0.0+\]
pub(crate) fn list_audio_outs(
    service: &Session,
    device_names_buf: &mut [u8],
) -> Result<u32, ListAudioOutsError> {
    list_audio_outs_impl(
        service,
        proto::LIST_AUDIO_OUTS,
        device_names_buf,
        BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
    )
}

/// Lists available audio output devices (map-alias, legacy). \[1.0.0-2.x.x\]
pub(crate) fn list_audio_outs_legacy(
    service: &Session,
    device_names_buf: &mut [u8],
) -> Result<u32, ListAudioOutsError> {
    list_audio_outs_impl(
        service,
        proto::LIST_AUDIO_OUTS_LEGACY,
        device_names_buf,
        BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
    )
}

fn list_audio_outs_impl(
    service: &Session,
    cmd_id: u32,
    device_names_buf: &mut [u8],
    buffer_attr: BufferAttr,
) -> Result<u32, ListAudioOutsError> {
    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u32>())
        .out_buffer(device_names_buf, buffer_attr)
        .send()
        .map_err(ListAudioOutsError)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let count = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(count)
}

/// Opens an audio output device (auto-select). \[3.0.0+\]
pub(crate) fn open_audio_out(
    service: &Session,
    input: &OpenAudioOutIn,
    device_name_in: &[u8],
    device_name_out: &mut [u8],
) -> Result<(u32, OpenAudioOutOut), OpenAudioOutError> {
    open_audio_out_impl(
        service,
        proto::OPEN_AUDIO_OUT,
        input,
        device_name_in,
        device_name_out,
        BufferAttr::HIPC_AUTO_SELECT,
    )
}

/// Opens an audio output device (map-alias, legacy). \[1.0.0-2.x.x\]
pub(crate) fn open_audio_out_legacy(
    service: &Session,
    input: &OpenAudioOutIn,
    device_name_in: &[u8],
    device_name_out: &mut [u8],
) -> Result<(u32, OpenAudioOutOut), OpenAudioOutError> {
    open_audio_out_impl(
        service,
        proto::OPEN_AUDIO_OUT_LEGACY,
        input,
        device_name_in,
        device_name_out,
        BufferAttr::HIPC_MAP_ALIAS,
    )
}

fn open_audio_out_impl(
    service: &Session,
    cmd_id: u32,
    input: &OpenAudioOutIn,
    device_name_in: &[u8],
    device_name_out: &mut [u8],
    transfer_attr: BufferAttr,
) -> Result<(u32, OpenAudioOutOut), OpenAudioOutError> {
    // SAFETY: `input` is a `Copy`-compatible value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *input).cast::<u8>(),
            size_of::<OpenAudioOutIn>(),
        )
    };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_handle(nx_svc::raw::CUR_PROCESS_HANDLE)
        .send_pid()
        .in_buffer(device_name_in, transfer_attr)
        .out_buffer(device_name_out, transfer_attr)
        .out_size(size_of::<OpenAudioOutOut>())
        .send()
        .map_err(OpenAudioOutError::Dispatch)?;

    let Some(&handle) = result.move_handles.first() else {
        return Err(OpenAudioOutError::MissingHandle);
    };

    // SAFETY: response payload is at least size_of::<OpenAudioOutOut>().
    let out = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<OpenAudioOutOut>()) };

    Ok((handle, out))
}

// ---------------------------------------------------------------------------
// Audio-out sub-object commands (IAudioOut)
// ---------------------------------------------------------------------------

/// Gets the current audio output state.
pub(crate) fn audio_out_get_state(service: &Session) -> Result<u32, DispatchError> {
    let result = service
        .dispatch(proto::AUDIO_OUT_GET_STATE)
        .out_size(size_of::<u32>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u32>().
    let state = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(state)
}

/// Starts audio output playback.
pub(crate) fn audio_out_start(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::AUDIO_OUT_START)
}

/// Stops audio output playback.
pub(crate) fn audio_out_stop(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::AUDIO_OUT_STOP)
}

/// Registers the buffer event (returns copy handle).
pub(crate) fn audio_out_register_buffer_event(service: &Session) -> Result<u32, DispatchError> {
    let result = service
        .dispatch(proto::AUDIO_OUT_REGISTER_BUFFER_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;

    Ok(result.copy_handles[0])
}

/// Appends an audio output buffer (auto-select). \[3.0.0+\]
pub(crate) fn audio_out_append_buffer(
    service: &Session,
    buffer_client_ptr: u64,
    buffer: &AudioOutBuffer,
) -> Result<(), AppendBufferError> {
    append_buffer_impl(
        service,
        proto::AUDIO_OUT_APPEND_BUFFER,
        buffer_client_ptr,
        buffer,
        BufferAttr::IN.or(BufferAttr::HIPC_AUTO_SELECT),
    )
}

/// Appends an audio output buffer (map-alias, legacy). \[1.0.0-2.x.x\]
pub(crate) fn audio_out_append_buffer_legacy(
    service: &Session,
    buffer_client_ptr: u64,
    buffer: &AudioOutBuffer,
) -> Result<(), AppendBufferError> {
    append_buffer_impl(
        service,
        proto::AUDIO_OUT_APPEND_BUFFER_LEGACY,
        buffer_client_ptr,
        buffer,
        BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
    )
}

fn append_buffer_impl(
    service: &Session,
    cmd_id: u32,
    buffer_client_ptr: u64,
    buffer: &AudioOutBuffer,
    buffer_attr: BufferAttr,
) -> Result<(), AppendBufferError> {
    // SAFETY: `buffer_client_ptr` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const buffer_client_ptr).cast::<u8>(),
            size_of::<u64>(),
        )
    };
    // SAFETY: `buffer` is a valid `&AudioOutBuffer`; viewing it as a byte
    // slice for the IN buffer is sound, and the slice borrows `buffer`.
    let buf_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *buffer).cast::<u8>(),
            size_of::<AudioOutBuffer>(),
        )
    };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(buf_bytes, buffer_attr)
        .send()
        .map(|_| ())
        .map_err(AppendBufferError)
}

/// Gets a released audio output buffer (auto-select). \[3.0.0+\]
pub(crate) fn audio_out_get_released_buffer(
    service: &Session,
    out_buffer_ptr: &mut u64,
) -> Result<u32, GetReleasedBufferError> {
    get_released_buffer_impl(
        service,
        proto::AUDIO_OUT_GET_RELEASED_BUFFER,
        out_buffer_ptr,
        BufferAttr::OUT.or(BufferAttr::HIPC_AUTO_SELECT),
    )
}

/// Gets a released audio output buffer (map-alias, legacy). \[1.0.0-2.x.x\]
pub(crate) fn audio_out_get_released_buffer_legacy(
    service: &Session,
    out_buffer_ptr: &mut u64,
) -> Result<u32, GetReleasedBufferError> {
    get_released_buffer_impl(
        service,
        proto::AUDIO_OUT_GET_RELEASED_BUFFER_LEGACY,
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
    // SAFETY: `out_buffer_ptr` is a valid `&mut u64`; viewing it as a byte
    // slice for the OUT buffer is sound, and the slice borrows it.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut((out_buffer_ptr as *mut u64).cast::<u8>(), size_of::<u64>())
    };
    let result = service
        .dispatch(cmd_id)
        .out_size(size_of::<u32>())
        .out_buffer(out_bytes, buffer_attr)
        .send()
        .map_err(GetReleasedBufferError)?;

    // SAFETY: response payload is at least size_of::<u32>().
    let count = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(count)
}

/// Checks whether a buffer is contained in the audio output.
pub(crate) fn audio_out_contains_buffer(
    service: &Session,
    buffer_client_ptr: u64,
) -> Result<bool, ContainsBufferError> {
    // SAFETY: `buffer_client_ptr` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const buffer_client_ptr).cast::<u8>(),
            size_of::<u64>(),
        )
    };
    let result = service
        .dispatch(proto::AUDIO_OUT_CONTAINS_BUFFER)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .send()
        .map_err(ContainsBufferError)?;

    // SAFETY: response payload is at least size_of::<u8>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr()) };

    Ok(val & 1 != 0)
}

/// Gets the number of queued audio output buffers. \[4.0.0+\]
pub(crate) fn audio_out_get_buffer_count(service: &Session) -> Result<u32, DispatchError> {
    let result = service
        .dispatch(proto::AUDIO_OUT_GET_BUFFER_COUNT)
        .out_size(size_of::<u32>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u32>().
    let count = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(count)
}

/// Gets the total number of played samples. \[4.0.0+\]
pub(crate) fn audio_out_get_played_sample_count(service: &Session) -> Result<u64, DispatchError> {
    let result = service
        .dispatch(proto::AUDIO_OUT_GET_PLAYED_SAMPLE_COUNT)
        .out_size(size_of::<u64>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u64>().
    let count = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(count)
}

/// Flushes all queued audio output buffers. \[4.0.0+\]
pub(crate) fn audio_out_flush_buffers(service: &Session) -> Result<bool, DispatchError> {
    let result = service
        .dispatch(proto::AUDIO_OUT_FLUSH_BUFFERS)
        .out_size(size_of::<u8>())
        .send()?;

    // SAFETY: response payload is at least size_of::<u8>().
    let val = unsafe { ptr::read_unaligned(result.data.as_ptr()) };

    Ok(val & 1 != 0)
}

/// Sets the audio output volume. \[6.0.0+\]
pub(crate) fn audio_out_set_volume(service: &Session, volume: f32) -> Result<(), SetVolumeError> {
    // SAFETY: `volume` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const volume).cast::<u8>(), size_of::<f32>()) };
    service
        .dispatch(proto::AUDIO_OUT_SET_VOLUME)
        .in_raw(in_bytes)
        .send()
        .map(|_| ())
        .map_err(SetVolumeError)
}

/// Gets the audio output volume. \[6.0.0+\]
pub(crate) fn audio_out_get_volume(service: &Session) -> Result<f32, GetVolumeError> {
    let result = service
        .dispatch(proto::AUDIO_OUT_GET_VOLUME)
        .out_size(size_of::<f32>())
        .send()
        .map_err(GetVolumeError)?;

    // SAFETY: response payload is at least size_of::<f32>().
    let volume = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<f32>()) };

    Ok(volume)
}

// ---------------------------------------------------------------------------
// audout:a commands (pre-11.0.0)
// ---------------------------------------------------------------------------

/// Suspends audio output for a process.
pub(crate) fn audouta_request_suspend(
    service: &Session,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(service, proto::AUDOUTA_REQUEST_SUSPEND, pid, delay)
}

/// Resumes audio output for a process.
pub(crate) fn audouta_request_resume(
    service: &Session,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(service, proto::AUDOUTA_REQUEST_RESUME, pid, delay)
}

/// Gets the master volume for a process.
pub(crate) fn audouta_get_process_master_volume(
    service: &Session,
    pid: u64,
) -> Result<f32, GetVolumeError> {
    dispatch_get_volume(service, proto::AUDOUTA_GET_PROCESS_MASTER_VOLUME, pid)
}

/// Sets the master volume for a process.
pub(crate) fn audouta_set_process_master_volume(
    service: &Session,
    pid: u64,
    delay: u64,
    volume: f32,
) -> Result<(), SetVolumeError> {
    dispatch_set_volume(
        service,
        proto::AUDOUTA_SET_PROCESS_MASTER_VOLUME,
        pid,
        delay,
        volume,
    )
}

/// Gets the record volume for a process.
pub(crate) fn audouta_get_process_record_volume(
    service: &Session,
    pid: u64,
) -> Result<f32, GetVolumeError> {
    dispatch_get_volume(service, proto::AUDOUTA_GET_PROCESS_RECORD_VOLUME, pid)
}

/// Sets the record volume for a process.
pub(crate) fn audouta_set_process_record_volume(
    service: &Session,
    pid: u64,
    delay: u64,
    volume: f32,
) -> Result<(), SetVolumeError> {
    dispatch_set_volume(
        service,
        proto::AUDOUTA_SET_PROCESS_RECORD_VOLUME,
        pid,
        delay,
        volume,
    )
}

// ---------------------------------------------------------------------------
// audout:d commands (pre-11.0.0)
// ---------------------------------------------------------------------------

/// Suspends audio output for a process (debug).
pub(crate) fn audoutd_request_suspend_for_debug(
    service: &Session,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(
        service,
        proto::AUDOUTD_REQUEST_SUSPEND_FOR_DEBUG,
        pid,
        delay,
    )
}

/// Resumes audio output for a process (debug).
pub(crate) fn audoutd_request_resume_for_debug(
    service: &Session,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(service, proto::AUDOUTD_REQUEST_RESUME_FOR_DEBUG, pid, delay)
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_pid_delay(
    service: &Session,
    cmd_id: u32,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    let input = PidDelayIn { pid, delay };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<PidDelayIn>())
    };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send()
        .map(|_| ())
        .map_err(SuspendResumeError)
}

fn dispatch_get_volume(service: &Session, cmd_id: u32, pid: u64) -> Result<f32, GetVolumeError> {
    // SAFETY: `pid` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const pid).cast::<u8>(), size_of::<u64>()) };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_size(size_of::<f32>())
        .send()
        .map_err(GetVolumeError)?;

    // SAFETY: response payload is at least size_of::<f32>().
    let volume = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<f32>()) };

    Ok(volume)
}

fn dispatch_set_volume(
    service: &Session,
    cmd_id: u32,
    pid: u64,
    delay: u64,
    volume: f32,
) -> Result<(), SetVolumeError> {
    let input = SetVolumeIn {
        volume,
        _pad: [0; 4],
        pid,
        delay,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<SetVolumeIn>())
    };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send()
        .map(|_| ())
        .map_err(SetVolumeError)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by list-audio-outs operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to list audio output devices")]
pub struct ListAudioOutsError(#[source] pub DispatchError);

/// Error returned by open-audio-out operations.
#[derive(Debug, thiserror::Error)]
pub enum OpenAudioOutError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenAudioOut")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected session handle.
    #[error("OpenAudioOut response missing move handle")]
    MissingHandle,
}

/// Error returned by append-buffer operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to append audio output buffer")]
pub struct AppendBufferError(#[source] pub DispatchError);

/// Error returned by get-released-buffer operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get released audio output buffer")]
pub struct GetReleasedBufferError(#[source] pub DispatchError);

/// Error returned by contains-buffer operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to check audio output buffer containment")]
pub struct ContainsBufferError(#[source] pub DispatchError);

/// Error returned by suspend/resume operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to suspend/resume audio output")]
pub struct SuspendResumeError(#[source] pub DispatchError);

/// Error returned by set-volume operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to set audio output volume")]
pub struct SetVolumeError(#[source] pub DispatchError);

/// Error returned by get-volume operations.
#[derive(Debug, thiserror::Error)]
#[error("failed to get audio output volume")]
pub struct GetVolumeError(#[source] pub DispatchError);
