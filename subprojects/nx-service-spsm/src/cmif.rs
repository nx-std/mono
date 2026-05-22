//! CMIF protocol operations for the spsm service.

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Initiates system shutdown or reboot.
pub fn shutdown(session: SessionHandle, reboot: bool) -> Result<(), ShutdownError> {
    let in_data: u8 = u8::from(reboot);

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SHUTDOWN)
        .with_data_value(&in_data)
        .build();
    req.write_to(&mut buf)
        .map_err(ShutdownError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(ShutdownError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(ShutdownError::ParseResponse)?;

    Ok(())
}

/// Puts the system into an error state.
pub fn put_error_state(session: SessionHandle) -> Result<(), PutErrorStateError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::PUT_ERROR_STATE).build();
    req.write_to(&mut buf)
        .map_err(PutErrorStateError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(PutErrorStateError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(PutErrorStateError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`shutdown`].
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
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

/// Error returned by [`put_error_state`].
#[derive(Debug, thiserror::Error)]
pub enum PutErrorStateError {
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
