//! CMIF protocol operations for the csrng service.

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Fills `out` with cryptographically-secure random bytes.
#[inline]
pub fn get_random_bytes(session: SessionHandle, out: &mut [u8]) -> Result<(), GetRandomBytesError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_RANDOM_BYTES)
        .out_buffers(1)
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    req.add_out_buffer(out.as_mut_ptr(), out.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(GetRandomBytesError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetRandomBytesError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`get_random_bytes`].
#[derive(Debug, thiserror::Error)]
pub enum GetRandomBytesError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
