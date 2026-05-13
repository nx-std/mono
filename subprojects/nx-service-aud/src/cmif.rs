//! CMIF protocol operations for the audio services.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};
use static_assertions::const_assert_eq;

use crate::proto;

/// Wire input for suspend/resume commands: `{ u64 pid, u64 delay }`.
#[derive(Clone, Copy)]
#[repr(C)]
struct PidDelayIn {
    pid: u64,
    delay: u64,
}

const_assert_eq!(size_of::<PidDelayIn>(), 0x10);

/// Wire input for set-volume commands: `{ f32 volume, pad, u64 pid, u64 delay }`.
#[derive(Clone, Copy)]
#[repr(C)]
struct SetVolumeIn {
    volume: f32,
    _pad: [u8; 4],
    pid: u64,
    delay: u64,
}

const_assert_eq!(size_of::<SetVolumeIn>(), 0x18);

// ---------------------------------------------------------------------------
// aud:a commands
// ---------------------------------------------------------------------------

/// Suspends audio for a process.
pub fn request_suspend_audio(
    session: SessionHandle,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(session, proto::REQUEST_SUSPEND_AUDIO, pid, delay)
}

/// Resumes audio for a process.
pub fn request_resume_audio(
    session: SessionHandle,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(session, proto::REQUEST_RESUME_AUDIO, pid, delay)
}

/// Gets the master volume for a process's audio output.
pub fn get_audio_output_process_master_volume(
    session: SessionHandle,
    pid: u64,
) -> Result<f32, GetVolumeError> {
    dispatch_get_volume(session, proto::GET_AUDIO_OUTPUT_PROCESS_MASTER_VOLUME, pid)
}

/// Sets the master volume for a process's audio output.
pub fn set_audio_output_process_master_volume(
    session: SessionHandle,
    pid: u64,
    delay: u64,
    volume: f32,
) -> Result<(), SetVolumeError> {
    dispatch_set_volume(
        session,
        proto::SET_AUDIO_OUTPUT_PROCESS_MASTER_VOLUME,
        pid,
        delay,
        volume,
    )
}

/// Gets the master volume for a process's audio input.
pub fn get_audio_input_process_master_volume(
    session: SessionHandle,
    pid: u64,
) -> Result<f32, GetVolumeError> {
    dispatch_get_volume(session, proto::GET_AUDIO_INPUT_PROCESS_MASTER_VOLUME, pid)
}

/// Sets the master volume for a process's audio input and output.
pub fn set_audio_input_process_master_volume(
    session: SessionHandle,
    pid: u64,
    delay: u64,
    volume: f32,
) -> Result<(), SetVolumeError> {
    dispatch_set_volume(
        session,
        proto::SET_AUDIO_INPUT_PROCESS_MASTER_VOLUME,
        pid,
        delay,
        volume,
    )
}

/// Gets the record volume for a process's audio output.
pub fn get_audio_output_process_record_volume(
    session: SessionHandle,
    pid: u64,
) -> Result<f32, GetVolumeError> {
    dispatch_get_volume(session, proto::GET_AUDIO_OUTPUT_PROCESS_RECORD_VOLUME, pid)
}

/// Sets the record volume for a process's audio output.
pub fn set_audio_output_process_record_volume(
    session: SessionHandle,
    pid: u64,
    delay: u64,
    volume: f32,
) -> Result<(), SetVolumeError> {
    dispatch_set_volume(
        session,
        proto::SET_AUDIO_OUTPUT_PROCESS_RECORD_VOLUME,
        pid,
        delay,
        volume,
    )
}

// ---------------------------------------------------------------------------
// aud:d commands
// ---------------------------------------------------------------------------

/// Suspends audio for a process (debug).
pub fn request_suspend_audio_for_debug(
    session: SessionHandle,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(session, proto::REQUEST_SUSPEND_AUDIO_FOR_DEBUG, pid, delay)
}

/// Resumes audio for a process (debug).
pub fn request_resume_audio_for_debug(
    session: SessionHandle,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    dispatch_pid_delay(session, proto::REQUEST_RESUME_AUDIO_FOR_DEBUG, pid, delay)
}

// ---------------------------------------------------------------------------
// Shared dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_pid_delay(
    session: SessionHandle,
    cmd: u32,
    pid: u64,
    delay: u64,
) -> Result<(), SuspendResumeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd)
        .data_size(size_of::<PidDelayIn>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = PidDelayIn { pid, delay };

    // SAFETY: req.data points to valid payload area with space for PidDelayIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<PidDelayIn>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SuspendResumeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SuspendResumeError::ParseResponse)?;

    Ok(())
}

fn dispatch_get_volume(session: SessionHandle, cmd: u32, pid: u64) -> Result<f32, GetVolumeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd)
        .data_size(size_of::<u64>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), pid);
    }

    ipc::send_sync_request(session).map_err(GetVolumeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<f32>()) }
        .map_err(GetVolumeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f32.
    let volume = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(volume)
}

fn dispatch_set_volume(
    session: SessionHandle,
    cmd: u32,
    pid: u64,
    delay: u64,
    volume: f32,
) -> Result<(), SetVolumeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd)
        .data_size(size_of::<SetVolumeIn>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    let input = SetVolumeIn {
        volume,
        _pad: [0; 4],
        pid,
        delay,
    };

    // SAFETY: req.data points to valid payload area with space for SetVolumeIn.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<SetVolumeIn>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(SetVolumeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(SetVolumeError::ParseResponse)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by suspend/resume operations.
#[derive(Debug, thiserror::Error)]
pub enum SuspendResumeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by get-volume operations.
#[derive(Debug, thiserror::Error)]
pub enum GetVolumeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by set-volume operations.
#[derive(Debug, thiserror::Error)]
pub enum SetVolumeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
