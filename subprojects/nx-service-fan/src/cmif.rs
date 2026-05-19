//! CMIF protocol operations for the fan service.

use core::{mem::size_of, ptr};

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Opens an `IController` session for the given device code.
pub fn open_controller(
    session: SessionHandle,
    device_code: u32,
) -> Result<SessionHandle, OpenControllerError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::OPEN_CONTROLLER)
            .data_size(size_of::<u32>())
            .send()
            .map_err(OpenControllerError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u32>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u32>(), device_code) };
    }

    ipc::send_sync_request(session).map_err(OpenControllerError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, 0).map_err(OpenControllerError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(OpenControllerError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Sets the fan rotation speed level on the controller.
pub fn set_rotation_speed_level(
    session: SessionHandle,
    level: f32,
) -> Result<(), SetRotationSpeedLevelError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SET_ROTATION_SPEED_LEVEL)
            .data_size(size_of::<f32>())
            .send()
            .map_err(SetRotationSpeedLevelError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<f32>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<f32>(), level) };
    }

    ipc::send_sync_request(session).map_err(SetRotationSpeedLevelError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(SetRotationSpeedLevelError::ParseResponse)?;

    Ok(())
}

/// Gets the current fan rotation speed level from the controller.
pub fn get_rotation_speed_level(session: SessionHandle) -> Result<f32, GetRotationSpeedLevelError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        // The get-rotation-speed-level request carries no payload data.
        cmif::CmifBuilder::new(&mut buf, proto::GET_ROTATION_SPEED_LEVEL)
            .send()
            .map_err(GetRotationSpeedLevelError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetRotationSpeedLevelError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<f32>())
        .map_err(GetRotationSpeedLevelError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<f32>()` bytes.
    let level = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(level)
}

/// Error returned by [`open_controller`].
#[derive(Debug, thiserror::Error)]
pub enum OpenControllerError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response did not contain the expected move handle.
    #[error("missing controller handle in response")]
    MissingHandle,
}

/// Error returned by [`set_rotation_speed_level`].
#[derive(Debug, thiserror::Error)]
pub enum SetRotationSpeedLevelError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by [`get_rotation_speed_level`].
#[derive(Debug, thiserror::Error)]
pub enum GetRotationSpeedLevelError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
