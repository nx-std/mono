//! CMIF protocol operations for the IAudioDevice service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{proto, types::AudioDeviceName};

// ---------------------------------------------------------------------------
// get_audio_device_service (IAudioRendererManager cmd 2)
// ---------------------------------------------------------------------------

/// Opens an `IAudioDevice` session from the `audren:u` manager.
///
/// Returns the move handle for the new IAudioDevice session.
pub fn get_audio_device_service(
    session: SessionHandle,
    aruid: u64,
) -> Result<SessionHandle, GetAudioDeviceServiceError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_AUDIO_DEVICE_SERVICE)
        .data_size(size_of::<u64>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), aruid);
    }

    ipc::send_sync_request(session).map_err(GetAudioDeviceServiceError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetAudioDeviceServiceError::ParseResponse)?;

    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(GetAudioDeviceServiceError::MissingHandle)?;

    // SAFETY: handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by [`get_audio_device_service`].
#[derive(Debug, thiserror::Error)]
pub enum GetAudioDeviceServiceError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing session handle in response")]
    MissingHandle,
}

// ---------------------------------------------------------------------------
// list_audio_device_name (3.0.0+)
// ---------------------------------------------------------------------------

/// Lists audio device names (3.0.0+, auto-select buffers).
///
/// Returns the number of names written to `names`.
pub fn list_audio_device_name(
    session: SessionHandle,
    names: &mut [AudioDeviceName],
) -> Result<i32, ListAudioDeviceNameError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::LIST_AUDIO_DEVICE_NAME)
        .out_auto_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_out_auto_buffer(
        names.as_mut_ptr().cast::<u8>(),
        size_of_val(names),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(ListAudioDeviceNameError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<i32>()) }
        .map_err(ListAudioDeviceNameError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let total = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(total)
}

// ---------------------------------------------------------------------------
// list_audio_device_name_legacy (pre-3.0.0)
// ---------------------------------------------------------------------------

/// Lists audio device names (pre-3.0.0, mapped buffers).
///
/// Returns the number of names written to `names`.
pub fn list_audio_device_name_legacy(
    session: SessionHandle,
    names: &mut [AudioDeviceName],
) -> Result<i32, ListAudioDeviceNameError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::LIST_AUDIO_DEVICE_NAME_OLD)
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_out_buffer(
        names.as_mut_ptr().cast::<u8>(),
        size_of_val(names),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(ListAudioDeviceNameError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<i32>()) }
        .map_err(ListAudioDeviceNameError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let total = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(total)
}

/// Error returned by [`list_audio_device_name`] / [`list_audio_device_name_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum ListAudioDeviceNameError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// set_audio_device_output_volume (3.0.0+)
// ---------------------------------------------------------------------------

/// Sets the output volume for a named audio device (3.0.0+, auto-select buffers).
pub fn set_audio_device_output_volume(
    session: SessionHandle,
    device_name: &AudioDeviceName,
    volume: f32,
) -> Result<(), SetAudioDeviceOutputVolumeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_AUDIO_DEVICE_OUTPUT_VOLUME)
        .data_size(size_of::<f32>())
        .in_auto_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for f32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<f32>().cast_mut(), volume);
    }

    req.add_in_auto_buffer(
        (device_name as *const AudioDeviceName).cast::<u8>(),
        size_of::<AudioDeviceName>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(SetAudioDeviceOutputVolumeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetAudioDeviceOutputVolumeError::ParseResponse)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// set_audio_device_output_volume_legacy (pre-3.0.0)
// ---------------------------------------------------------------------------

/// Sets the output volume for a named audio device (pre-3.0.0, mapped buffers).
pub fn set_audio_device_output_volume_legacy(
    session: SessionHandle,
    device_name: &AudioDeviceName,
    volume: f32,
) -> Result<(), SetAudioDeviceOutputVolumeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD)
        .data_size(size_of::<f32>())
        .in_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for f32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<f32>().cast_mut(), volume);
    }

    req.add_in_buffer(
        (device_name as *const AudioDeviceName).cast::<u8>(),
        size_of::<AudioDeviceName>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(SetAudioDeviceOutputVolumeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetAudioDeviceOutputVolumeError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_audio_device_output_volume`] / [`set_audio_device_output_volume_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum SetAudioDeviceOutputVolumeError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// get_audio_device_output_volume (3.0.0+)
// ---------------------------------------------------------------------------

/// Gets the output volume for a named audio device (3.0.0+, auto-select buffers).
pub fn get_audio_device_output_volume(
    session: SessionHandle,
    device_name: &AudioDeviceName,
) -> Result<f32, GetAudioDeviceOutputVolumeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_AUDIO_DEVICE_OUTPUT_VOLUME)
        .in_auto_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_in_auto_buffer(
        (device_name as *const AudioDeviceName).cast::<u8>(),
        size_of::<AudioDeviceName>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(GetAudioDeviceOutputVolumeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<f32>()) }
        .map_err(GetAudioDeviceOutputVolumeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f32.
    let volume = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(volume)
}

// ---------------------------------------------------------------------------
// get_audio_device_output_volume_legacy (pre-3.0.0)
// ---------------------------------------------------------------------------

/// Gets the output volume for a named audio device (pre-3.0.0, mapped buffers).
pub fn get_audio_device_output_volume_legacy(
    session: SessionHandle,
    device_name: &AudioDeviceName,
) -> Result<f32, GetAudioDeviceOutputVolumeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD)
        .in_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_in_buffer(
        (device_name as *const AudioDeviceName).cast::<u8>(),
        size_of::<AudioDeviceName>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(GetAudioDeviceOutputVolumeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<f32>()) }
        .map_err(GetAudioDeviceOutputVolumeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f32.
    let volume = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(volume)
}

/// Error returned by [`get_audio_device_output_volume`] / [`get_audio_device_output_volume_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum GetAudioDeviceOutputVolumeError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// get_active_audio_device_name (3.0.0+)
// ---------------------------------------------------------------------------

/// Gets the active audio device name (3.0.0+, auto-select buffers).
pub fn get_active_audio_device_name(
    session: SessionHandle,
    device_name: &mut AudioDeviceName,
) -> Result<(), GetActiveAudioDeviceNameError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_ACTIVE_AUDIO_DEVICE_NAME)
        .out_auto_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_out_auto_buffer(
        (device_name as *mut AudioDeviceName).cast::<u8>(),
        size_of::<AudioDeviceName>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(GetActiveAudioDeviceNameError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetActiveAudioDeviceNameError::ParseResponse)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// get_active_audio_device_name_legacy (pre-3.0.0)
// ---------------------------------------------------------------------------

/// Gets the active audio device name (pre-3.0.0, mapped buffers).
pub fn get_active_audio_device_name_legacy(
    session: SessionHandle,
    device_name: &mut AudioDeviceName,
) -> Result<(), GetActiveAudioDeviceNameError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_ACTIVE_AUDIO_DEVICE_NAME_OLD)
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_out_buffer(
        (device_name as *mut AudioDeviceName).cast::<u8>(),
        size_of::<AudioDeviceName>(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(GetActiveAudioDeviceNameError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetActiveAudioDeviceNameError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`get_active_audio_device_name`] / [`get_active_audio_device_name_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum GetActiveAudioDeviceNameError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
