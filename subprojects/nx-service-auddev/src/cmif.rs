//! CMIF protocol operations for the IAudioDevice service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::{proto, types::AudioDeviceName};

/// Opens an `IAudioDevice` session from the `audren:u` manager.
///
/// Returns the move handle for the new IAudioDevice session.
pub fn get_audio_device_service(
    session: SessionHandle,
    aruid: u64,
) -> Result<SessionHandle, GetAudioDeviceServiceError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_AUDIO_DEVICE_SERVICE)
        .data_size(size_of::<u64>())
        .build();
    req.write_to(&mut buf)
        .map_err(GetAudioDeviceServiceError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<u64>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<u64>(), aruid) };

    ipc::send_sync_request(&mut buf, session).map_err(GetAudioDeviceServiceError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, 0).map_err(GetAudioDeviceServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetAudioDeviceServiceError::MissingHandle);
    };

    // SAFETY: handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by [`get_audio_device_service`].
#[derive(Debug, thiserror::Error)]
pub enum GetAudioDeviceServiceError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Lists audio device names (3.0.0+, auto-select buffers).
///
/// Returns the number of names written to `names`.
pub fn list_audio_device_name(
    session: SessionHandle,
    names: &mut [AudioDeviceName],
) -> Result<i32, ListAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::LIST_AUDIO_DEVICE_NAME).add_out_auto_buffer(
        names.as_mut_ptr().cast::<u8>(),
        core::mem::size_of_val(names),
        BufferMode::Normal,
    );
    req.build()
        .write_to(&mut buf)
        .map_err(ListAudioDeviceNameError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(ListAudioDeviceNameError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
        .map_err(ListAudioDeviceNameError::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<i32>()` bytes.
    let total = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(total)
}

/// Lists audio device names (pre-3.0.0, mapped buffers).
///
/// Returns the number of names written to `names`.
pub fn list_audio_device_name_legacy(
    session: SessionHandle,
    names: &mut [AudioDeviceName],
) -> Result<i32, ListAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::LIST_AUDIO_DEVICE_NAME_OLD)
        .add_out_buffer(
            names.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(names),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(ListAudioDeviceNameError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(ListAudioDeviceNameError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
        .map_err(ListAudioDeviceNameError::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<i32>()` bytes.
    let total = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(total)
}

/// Error returned by [`list_audio_device_name`] / [`list_audio_device_name_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum ListAudioDeviceNameError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Sets the output volume for a named audio device (3.0.0+, auto-select buffers).
pub fn set_audio_device_output_volume(
    session: SessionHandle,
    device_name: &AudioDeviceName,
    volume: f32,
) -> Result<(), SetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::SET_AUDIO_DEVICE_OUTPUT_VOLUME)
        .data_size(size_of::<f32>())
        .add_in_auto_buffer(
            (device_name as *const AudioDeviceName).cast::<u8>(),
            size_of::<AudioDeviceName>(),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(SetAudioDeviceOutputVolumeError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<f32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<f32>(), volume) };

    ipc::send_sync_request(&mut buf, session)
        .map_err(SetAudioDeviceOutputVolumeError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(SetAudioDeviceOutputVolumeError::ParseResponse)?;

    Ok(())
}

/// Sets the output volume for a named audio device (pre-3.0.0, mapped buffers).
pub fn set_audio_device_output_volume_legacy(
    session: SessionHandle,
    device_name: &AudioDeviceName,
    volume: f32,
) -> Result<(), SetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::SET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD)
        .data_size(size_of::<f32>())
        .add_in_buffer(
            (device_name as *const AudioDeviceName).cast::<u8>(),
            size_of::<AudioDeviceName>(),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(SetAudioDeviceOutputVolumeError::BuildRequest)?;

    // SAFETY: `req` is exactly `size_of::<f32>()` bytes.
    unsafe { ptr::write_unaligned(buf.as_array_mut().as_mut_ptr().cast::<f32>(), volume) };

    ipc::send_sync_request(&mut buf, session)
        .map_err(SetAudioDeviceOutputVolumeError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(SetAudioDeviceOutputVolumeError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_audio_device_output_volume`] / [`set_audio_device_output_volume_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum SetAudioDeviceOutputVolumeError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Gets the output volume for a named audio device (3.0.0+, auto-select buffers).
pub fn get_audio_device_output_volume(
    session: SessionHandle,
    device_name: &AudioDeviceName,
) -> Result<f32, GetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_AUDIO_DEVICE_OUTPUT_VOLUME)
        .add_in_auto_buffer(
            (device_name as *const AudioDeviceName).cast::<u8>(),
            size_of::<AudioDeviceName>(),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(GetAudioDeviceOutputVolumeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetAudioDeviceOutputVolumeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<f32>())
        .map_err(GetAudioDeviceOutputVolumeError::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<f32>()` bytes.
    let volume = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(volume)
}

/// Gets the output volume for a named audio device (pre-3.0.0, mapped buffers).
pub fn get_audio_device_output_volume_legacy(
    session: SessionHandle,
    device_name: &AudioDeviceName,
) -> Result<f32, GetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD)
        .add_in_buffer(
            (device_name as *const AudioDeviceName).cast::<u8>(),
            size_of::<AudioDeviceName>(),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(GetAudioDeviceOutputVolumeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetAudioDeviceOutputVolumeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<f32>())
        .map_err(GetAudioDeviceOutputVolumeError::ParseResponse)?;

    // SAFETY: resp.data points to at least `size_of::<f32>()` bytes.
    let volume = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(volume)
}

/// Error returned by [`get_audio_device_output_volume`] / [`get_audio_device_output_volume_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum GetAudioDeviceOutputVolumeError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Gets the active audio device name (3.0.0+, auto-select buffers).
pub fn get_active_audio_device_name(
    session: SessionHandle,
    device_name: &mut AudioDeviceName,
) -> Result<(), GetActiveAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_ACTIVE_AUDIO_DEVICE_NAME)
        .add_out_auto_buffer(
            (device_name as *mut AudioDeviceName).cast::<u8>(),
            size_of::<AudioDeviceName>(),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(GetActiveAudioDeviceNameError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetActiveAudioDeviceNameError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(GetActiveAudioDeviceNameError::ParseResponse)?;

    Ok(())
}

/// Gets the active audio device name (pre-3.0.0, mapped buffers).
pub fn get_active_audio_device_name_legacy(
    session: SessionHandle,
    device_name: &mut AudioDeviceName,
) -> Result<(), GetActiveAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_ACTIVE_AUDIO_DEVICE_NAME_OLD)
        .add_out_buffer(
            (device_name as *mut AudioDeviceName).cast::<u8>(),
            size_of::<AudioDeviceName>(),
            BufferMode::Normal,
        )
        .build();
    req.write_to(&mut buf)
        .map_err(GetActiveAudioDeviceNameError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetActiveAudioDeviceNameError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(GetActiveAudioDeviceNameError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`get_active_audio_device_name`] / [`get_active_audio_device_name_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum GetActiveAudioDeviceNameError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
