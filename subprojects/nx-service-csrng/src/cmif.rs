//! CMIF protocol operations for the csrng service.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};

use crate::proto;

/// Fills `out` with cryptographically-secure random bytes.
#[inline]
pub fn get_random_bytes(
    session: BorrowedSessionHandle<'_>,
    out: &mut [u8],
) -> Result<(), GetRandomBytesError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_RANDOM_BYTES)
        .add_output_buffer(OutputBuffer::new(out, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetRandomBytesError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(GetRandomBytesError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`get_random_bytes`].
#[derive(Debug, thiserror::Error)]
pub enum GetRandomBytesError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
