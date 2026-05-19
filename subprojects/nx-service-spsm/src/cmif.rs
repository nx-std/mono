//! CMIF protocol operations for the spsm service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Initiates system shutdown or reboot.
pub fn shutdown(session: SessionHandle, reboot: bool) -> Result<(), ShutdownError> {
    let in_data: u8 = u8::from(reboot);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, proto::SHUTDOWN)
            .data_size(1)
            .send()
            .map_err(ShutdownError::BuildRequest)?;

        // SAFETY: `req.data` is exactly 1 byte.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<u8>(), in_data);
        }
    }

    ipc::send_sync_request(session).map_err(ShutdownError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(ShutdownError::ParseResponse)?;

    Ok(())
}

/// Puts the system into an error state.
pub fn put_error_state(session: SessionHandle) -> Result<(), PutErrorStateError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, proto::PUT_ERROR_STATE)
            .send()
            .map_err(PutErrorStateError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(PutErrorStateError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
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
