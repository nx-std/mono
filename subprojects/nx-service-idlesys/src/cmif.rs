//! CMIF protocol operations for the idle:sys service.

use nx_sf::{
    cmif,
    service::BorrowedSessionHandle,
};

use crate::proto;

/// Reports that the user is active, resetting the sleep counter.
#[inline]
pub fn report_user_is_active(
    session: BorrowedSessionHandle<'_>,
) -> Result<(), ReportUserIsActiveError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::REPORT_USER_IS_ACTIVE).build();
    req.send(&mut buf, session)
        .map_err(ReportUserIsActiveError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(ReportUserIsActiveError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`report_user_is_active`].
#[derive(Debug, thiserror::Error)]
pub enum ReportUserIsActiveError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
