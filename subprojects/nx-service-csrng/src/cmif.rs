//! CMIF protocol operations for the csrng service.

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Fills `out` with cryptographically-secure random bytes.
#[inline]
pub fn get_random_bytes(session: SessionHandle, out: &mut [u8]) -> Result<(), GetRandomBytesError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        cmif::CmifBuilder::new(&mut buf, proto::GET_RANDOM_BYTES)
            .add_out_buffer(out.as_mut_ptr(), out.len(), BufferMode::Normal)
            .send()
            .map_err(GetRandomBytesError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetRandomBytesError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(GetRandomBytesError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`get_random_bytes`].
#[derive(Debug, thiserror::Error)]
pub enum GetRandomBytesError {
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
