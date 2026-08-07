//! CMIF protocol operations for the operation mode manager service.

use nx_sf::{
    cmif,
    service::BorrowedSessionHandle,
};

use crate::proto;

/// Gets the current operation mode.
///
/// Returns the raw `u8` value; the caller should convert via
/// [`OperationMode::from_raw`](crate::OperationMode::from_raw).
pub fn get_operation_mode(session: BorrowedSessionHandle<'_>) -> Result<u8, GetOperationModeError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_OPERATION_MODE).build();
    req.send(&mut buf, session)
        .map_err(GetOperationModeError::SendRequest)?;

    let resp = cmif::parse_response::<&u8>(&buf).map_err(GetOperationModeError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`get_operation_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetOperationModeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Sets the operation mode policy (3.0.0+).
pub fn set_operation_mode_policy(
    session: BorrowedSessionHandle<'_>,
    policy: u8,
) -> Result<(), SetOperationModePolicyError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::SET_OPERATION_MODE_POLICY)
        .with_data_value(&policy)
        .build();
    req.send(&mut buf, session)
        .map_err(SetOperationModePolicyError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetOperationModePolicyError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`set_operation_mode_policy`].
#[derive(Debug, thiserror::Error)]
pub enum SetOperationModePolicyError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Gets the default display resolution (3.0.0+).
///
/// Returns `(width, height)`.
pub fn get_default_display_resolution(
    session: BorrowedSessionHandle<'_>,
) -> Result<(i32, i32), GetDefaultDisplayResolutionError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_DEFAULT_DISPLAY_RESOLUTION).build();
    req.send(&mut buf, session)
        .map_err(GetDefaultDisplayResolutionError::SendRequest)?;

    let resp = cmif::parse_response::<&[i32; 2]>(&buf)
        .map_err(GetDefaultDisplayResolutionError::ParseResponse)?;

    let [width, height] = *resp.payload;

    Ok((width, height))
}

/// Error returned by [`get_default_display_resolution`].
#[derive(Debug, thiserror::Error)]
pub enum GetDefaultDisplayResolutionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
