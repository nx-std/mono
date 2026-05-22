//! CMIF protocol operations for the idle:sys service.

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Reports that the user is active, resetting the sleep counter.
#[inline]
pub fn report_user_is_active(session: SessionHandle) -> Result<(), ReportUserIsActiveError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::REPORT_USER_IS_ACTIVE).build();
    req.write_to(&mut buf)
        .map_err(ReportUserIsActiveError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(ReportUserIsActiveError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(ReportUserIsActiveError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`report_user_is_active`].
#[derive(Debug, thiserror::Error)]
pub enum ReportUserIsActiveError {
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
