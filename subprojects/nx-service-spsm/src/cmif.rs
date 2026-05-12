//! CMIF protocol operations for the spsm service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Initiates system shutdown or reboot.
pub fn shutdown(session: SessionHandle, reboot: bool) -> Result<(), ShutdownError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let in_data: u8 = u8::from(reboot);

    let fmt = cmif::RequestFormatBuilder::new(proto::SHUTDOWN)
        .data_size(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), in_data);
    }

    ipc::send_sync_request(session).map_err(ShutdownError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp =
        unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(ShutdownError::ParseResponse)?;

    Ok(())
}

/// Puts the system into an error state.
pub fn put_error_state(session: SessionHandle) -> Result<(), PutErrorStateError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::PUT_ERROR_STATE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(PutErrorStateError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(PutErrorStateError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`shutdown`].
#[derive(Debug, thiserror::Error)]
pub enum ShutdownError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`put_error_state`].
#[derive(Debug, thiserror::Error)]
pub enum PutErrorStateError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
