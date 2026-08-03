//! CMIF protocol operations for the IAudioDevice service.

use nx_sf::{
    cmif,
    hipc::{BufferMode, InputBuffer, OutputBuffer},
    ipc::Handle as RawSessionHandle,
    service::{BorrowedSessionHandle, OwnedSessionHandle},
};

use crate::{proto, types::AudioDeviceName};

/// Opens an `IAudioDevice` session from the `audren:u` manager.
///
/// Returns the move handle for the new IAudioDevice session.
pub fn get_audio_device_service(
    session: BorrowedSessionHandle<'_>,
    aruid: u64,
) -> Result<OwnedSessionHandle, GetAudioDeviceServiceError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_AUDIO_DEVICE_SERVICE)
        .with_data_value(&aruid)
        .build();
    req.send(&mut buf, session)
        .map_err(GetAudioDeviceServiceError::SendRequest)?;

    let resp =
        cmif::parse_response::<()>(&buf).map_err(GetAudioDeviceServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetAudioDeviceServiceError::MissingHandle);
    };

    // SAFETY: handle is from a valid IPC response.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(handle),
    ))
}

/// Error returned by [`get_audio_device_service`].
#[derive(Debug, thiserror::Error)]
pub enum GetAudioDeviceServiceError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Lists audio device names (3.0.0+, auto-select buffers).
///
/// Returns the number of names written to `names`.
pub fn list_audio_device_name(
    session: BorrowedSessionHandle<'_>,
    names: &mut [AudioDeviceName],
) -> Result<i32, ListAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `AudioDeviceName` is `#[repr(C)]` wrapping `[u8; 0x100]`, so the
    // typed slice can be reinterpreted as a byte slice for the call duration.
    let names_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            names.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(names),
        )
    };
    let req = cmif::CmifRequestBuilder::new(proto::LIST_AUDIO_DEVICE_NAME)
        .add_out_auto_buffer(OutputBuffer::new(names_bytes, BufferMode::Normal));
    req.build()
        .send(&mut buf, session)
        .map_err(ListAudioDeviceNameError::SendRequest)?;

    let resp =
        cmif::parse_response::<&i32>(&buf).map_err(ListAudioDeviceNameError::ParseResponse)?;

    let total = *resp.payload;

    Ok(total)
}

/// Lists audio device names (pre-3.0.0, mapped buffers).
///
/// Returns the number of names written to `names`.
pub fn list_audio_device_name_legacy(
    session: BorrowedSessionHandle<'_>,
    names: &mut [AudioDeviceName],
) -> Result<i32, ListAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // SAFETY: `AudioDeviceName` is `#[repr(C)]` wrapping `[u8; 0x100]`, so the
    // typed slice can be reinterpreted as a byte slice for the call duration.
    let names_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            names.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(names),
        )
    };
    let req = cmif::CmifRequestBuilder::new(proto::LIST_AUDIO_DEVICE_NAME_OLD)
        .add_output_buffer(OutputBuffer::new(names_bytes, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(ListAudioDeviceNameError::SendRequest)?;

    let resp =
        cmif::parse_response::<&i32>(&buf).map_err(ListAudioDeviceNameError::ParseResponse)?;

    let total = *resp.payload;

    Ok(total)
}

/// Error returned by [`list_audio_device_name`] / [`list_audio_device_name_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum ListAudioDeviceNameError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Sets the output volume for a named audio device (3.0.0+, auto-select buffers).
pub fn set_audio_device_output_volume(
    session: BorrowedSessionHandle<'_>,
    device_name: &AudioDeviceName,
    volume: f32,
) -> Result<(), SetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SET_AUDIO_DEVICE_OUTPUT_VOLUME)
        .with_data_value(&volume)
        .add_in_auto_buffer(InputBuffer::new(&device_name.name, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(SetAudioDeviceOutputVolumeError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetAudioDeviceOutputVolumeError::ParseResponse)?;

    Ok(())
}

/// Sets the output volume for a named audio device (pre-3.0.0, mapped buffers).
pub fn set_audio_device_output_volume_legacy(
    session: BorrowedSessionHandle<'_>,
    device_name: &AudioDeviceName,
    volume: f32,
) -> Result<(), SetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD)
        .with_data_value(&volume)
        .add_input_buffer(InputBuffer::new(&device_name.name, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(SetAudioDeviceOutputVolumeError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetAudioDeviceOutputVolumeError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_audio_device_output_volume`] / [`set_audio_device_output_volume_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum SetAudioDeviceOutputVolumeError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Gets the output volume for a named audio device (3.0.0+, auto-select buffers).
pub fn get_audio_device_output_volume(
    session: BorrowedSessionHandle<'_>,
    device_name: &AudioDeviceName,
) -> Result<f32, GetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_AUDIO_DEVICE_OUTPUT_VOLUME)
        .add_in_auto_buffer(InputBuffer::new(&device_name.name, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetAudioDeviceOutputVolumeError::SendRequest)?;

    let resp = cmif::parse_response::<&f32>(&buf)
        .map_err(GetAudioDeviceOutputVolumeError::ParseResponse)?;

    let volume = *resp.payload;

    Ok(volume)
}

/// Gets the output volume for a named audio device (pre-3.0.0, mapped buffers).
pub fn get_audio_device_output_volume_legacy(
    session: BorrowedSessionHandle<'_>,
    device_name: &AudioDeviceName,
) -> Result<f32, GetAudioDeviceOutputVolumeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_AUDIO_DEVICE_OUTPUT_VOLUME_OLD)
        .add_input_buffer(InputBuffer::new(&device_name.name, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetAudioDeviceOutputVolumeError::SendRequest)?;

    let resp = cmif::parse_response::<&f32>(&buf)
        .map_err(GetAudioDeviceOutputVolumeError::ParseResponse)?;

    let volume = *resp.payload;

    Ok(volume)
}

/// Error returned by [`get_audio_device_output_volume`] / [`get_audio_device_output_volume_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum GetAudioDeviceOutputVolumeError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Gets the active audio device name (3.0.0+, auto-select buffers).
pub fn get_active_audio_device_name(
    session: BorrowedSessionHandle<'_>,
    device_name: &mut AudioDeviceName,
) -> Result<(), GetActiveAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_ACTIVE_AUDIO_DEVICE_NAME)
        .add_out_auto_buffer(OutputBuffer::new(&mut device_name.name, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetActiveAudioDeviceNameError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(GetActiveAudioDeviceNameError::ParseResponse)?;

    Ok(())
}

/// Gets the active audio device name (pre-3.0.0, mapped buffers).
pub fn get_active_audio_device_name_legacy(
    session: BorrowedSessionHandle<'_>,
    device_name: &mut AudioDeviceName,
) -> Result<(), GetActiveAudioDeviceNameError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_ACTIVE_AUDIO_DEVICE_NAME_OLD)
        .add_output_buffer(OutputBuffer::new(&mut device_name.name, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetActiveAudioDeviceNameError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(GetActiveAudioDeviceNameError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`get_active_audio_device_name`] / [`get_active_audio_device_name_legacy`].
#[derive(Debug, thiserror::Error)]
pub enum GetActiveAudioDeviceNameError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
