//! CMIF protocol operations for the idle:sys service.

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Reports that the user is active, resetting the sleep counter.
#[inline]
pub fn report_user_is_active(session: SessionHandle) -> Result<(), ReportUserIsActiveError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::REPORT_USER_IS_ACTIVE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(ReportUserIsActiveError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ReportUserIsActiveError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`report_user_is_active`].
#[derive(Debug, thiserror::Error)]
pub enum ReportUserIsActiveError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
