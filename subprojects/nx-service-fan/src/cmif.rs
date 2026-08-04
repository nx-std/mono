//! CMIF protocol operations for the fan service.

use nx_sf::{
    cmif,
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
};

use crate::proto;

/// Opens an `IController` session for the given device code.
pub fn open_controller(
    session: BorrowedSessionHandle<'_>,
    device_code: u32,
) -> Result<OwnedSessionHandle, OpenControllerError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::OPEN_CONTROLLER)
        .with_data_value(&device_code)
        .build();
    req.send(&mut buf, session)
        .map_err(OpenControllerError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenControllerError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(OpenControllerError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(handle),
    ))
}

/// Sets the fan rotation speed level on the controller.
pub fn set_rotation_speed_level(
    session: BorrowedSessionHandle<'_>,
    level: f32,
) -> Result<(), SetRotationSpeedLevelError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SET_ROTATION_SPEED_LEVEL)
        .with_data_value(&level)
        .build();
    req.send(&mut buf, session)
        .map_err(SetRotationSpeedLevelError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetRotationSpeedLevelError::ParseResponse)?;

    Ok(())
}

/// Gets the current fan rotation speed level from the controller.
pub fn get_rotation_speed_level(
    session: BorrowedSessionHandle<'_>,
) -> Result<f32, GetRotationSpeedLevelError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // The get-rotation-speed-level request carries no payload data.
    let req = cmif::CmifRequestBuilder::new(proto::GET_ROTATION_SPEED_LEVEL).build();
    req.send(&mut buf, session)
        .map_err(GetRotationSpeedLevelError::SendRequest)?;

    let resp =
        cmif::parse_response::<&f32>(&buf).map_err(GetRotationSpeedLevelError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`open_controller`].
#[derive(Debug, thiserror::Error)]
pub enum OpenControllerError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not contain the expected move handle.
    #[error("missing controller handle in response")]
    MissingHandle,
}

/// Error returned by [`set_rotation_speed_level`].
#[derive(Debug, thiserror::Error)]
pub enum SetRotationSpeedLevelError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_rotation_speed_level`].
#[derive(Debug, thiserror::Error)]
pub enum GetRotationSpeedLevelError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
