//! CMIF protocol operations for the fan service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Opens an `IController` session for the given device code.
pub fn open_controller(
    session: SessionHandle,
    device_code: u32,
) -> Result<SessionHandle, OpenControllerError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_CONTROLLER)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), device_code);
    }

    ipc::send_sync_request(session).map_err(OpenControllerError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenControllerError::ParseResponse)?;

    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenControllerError::MissingHandle)?;

    // SAFETY: handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Sets the fan rotation speed level on the controller.
pub fn set_rotation_speed_level(
    session: SessionHandle,
    level: f32,
) -> Result<(), SetRotationSpeedLevelError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_ROTATION_SPEED_LEVEL)
        .data_size(size_of::<f32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for f32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<f32>().cast_mut(), level);
    }

    ipc::send_sync_request(session).map_err(SetRotationSpeedLevelError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetRotationSpeedLevelError::ParseResponse)?;

    Ok(())
}

/// Gets the current fan rotation speed level from the controller.
pub fn get_rotation_speed_level(session: SessionHandle) -> Result<f32, GetRotationSpeedLevelError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_ROTATION_SPEED_LEVEL).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetRotationSpeedLevelError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<f32>()) }
        .map_err(GetRotationSpeedLevelError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f32.
    let level = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(level)
}

/// Error returned by [`open_controller`].
#[derive(Debug, thiserror::Error)]
pub enum OpenControllerError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response did not contain the expected move handle.
    #[error("missing controller handle in response")]
    MissingHandle,
}

/// Error returned by [`set_rotation_speed_level`].
#[derive(Debug, thiserror::Error)]
pub enum SetRotationSpeedLevelError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_rotation_speed_level`].
#[derive(Debug, thiserror::Error)]
pub enum GetRotationSpeedLevelError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
