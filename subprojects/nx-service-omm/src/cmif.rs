//! CMIF protocol operations for the operation mode manager service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Gets the current operation mode.
///
/// Returns the raw `u8` value; the caller should convert via
/// [`OperationMode::from_raw`](crate::OperationMode::from_raw).
pub fn get_operation_mode(session: SessionHandle) -> Result<u8, GetOperationModeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_OPERATION_MODE).build();
    req.write_to(&mut buf)
        .map_err(GetOperationModeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetOperationModeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(GetOperationModeError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u8>()` bytes.
    let mode = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(mode)
}

/// Error returned by [`get_operation_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetOperationModeError {
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

/// Sets the operation mode policy (3.0.0+).
pub fn set_operation_mode_policy(
    session: SessionHandle,
    policy: u8,
) -> Result<(), SetOperationModePolicyError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SET_OPERATION_MODE_POLICY)
        .data_value(&policy)
        .build();
    req.write_to(&mut buf)
        .map_err(SetOperationModePolicyError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SetOperationModePolicyError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(SetOperationModePolicyError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_operation_mode_policy`].
#[derive(Debug, thiserror::Error)]
pub enum SetOperationModePolicyError {
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

/// Gets the default display resolution (3.0.0+).
///
/// Returns `(width, height)`.
pub fn get_default_display_resolution(
    session: SessionHandle,
) -> Result<(i32, i32), GetDefaultDisplayResolutionError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_DEFAULT_DISPLAY_RESOLUTION).build();
    req.write_to(&mut buf)
        .map_err(GetDefaultDisplayResolutionError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetDefaultDisplayResolutionError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<[i32; 2]>())
        .map_err(GetDefaultDisplayResolutionError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<[i32; 2]>()` bytes.
    let (width, height) = unsafe {
        let data_ptr = resp.data.as_ptr().cast::<i32>();
        (
            ptr::read_unaligned(data_ptr),
            ptr::read_unaligned(data_ptr.add(1)),
        )
    };

    Ok((width, height))
}

/// Error returned by [`get_default_display_resolution`].
#[derive(Debug, thiserror::Error)]
pub enum GetDefaultDisplayResolutionError {
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
