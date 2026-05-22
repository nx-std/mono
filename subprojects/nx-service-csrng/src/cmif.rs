//! CMIF protocol operations for the csrng service.

use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Fills `out` with cryptographically-secure random bytes.
#[inline]
pub fn get_random_bytes(session: SessionHandle, out: &mut [u8]) -> Result<(), GetRandomBytesError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_RANDOM_BYTES)
        .add_output_buffer_raw(out.as_mut_ptr(), out.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf)
        .map_err(GetRandomBytesError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetRandomBytesError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(GetRandomBytesError::ParseResponse)?;

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
    ParseResponse(#[source] cmif::ParseError),
}
