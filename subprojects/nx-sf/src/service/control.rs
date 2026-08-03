//! CMIF control-request helpers used by the typed service wrappers and FFI.
//!
//! Each function corresponds to one of the standard CMIF control request IDs
//! (0..=4) and is a stateless protocol primitive - no caller state is held.
//!
//! Errors during the close-style helpers ([`close_session`], [`close_object`])
//! are deliberately swallowed: matching libnx and Horizon OS conventions, a
//! peer that has already gone away must not prevent the local side from
//! releasing its resources. Drop impls and the FFI close symbols both invoke
//! these.

use nx_svc::{
    error::ResultCode,
    ipc::{self, Handle as SessionHandle},
};

use super::handle::{BorrowedSessionHandle, OwnedSessionHandle};
use crate::{
    cmif::{self, CmifCloseRequest, CmifControlRequestBuilder, ObjectId},
    error::{GENERIC_ERROR, ToResultCode},
};

/// Control request: convert session to domain.
const CTRL_CONVERT_TO_DOMAIN: u32 = 0;
/// Control request: copy domain object to a new session handle.
const CTRL_COPY_FROM_DOMAIN: u32 = 1;
/// Control request: clone the current session.
const CTRL_CLONE_OBJECT: u32 = 2;
/// Control request: query pointer-buffer size.
const CTRL_QUERY_POINTER_BUFFER_SIZE: u32 = 3;
/// Control request: clone the current session with a session manager tag.
const CTRL_CLONE_OBJECT_EX: u32 = 4;

/// Queries the server's pointer-buffer size via control request 3.
pub fn query_pointer_buffer_size(
    session: BorrowedSessionHandle<'_>,
) -> Result<u16, QueryPointerBufferSizeError> {
    // SAFETY: IPC operations are serialized on this thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    CmifControlRequestBuilder::new(CTRL_QUERY_POINTER_BUFFER_SIZE)
        .build()
        .send(&mut buf, session)
        .map_err(QueryPointerBufferSizeError::SendRequest)?;

    let resp =
        cmif::parse_response::<&u16>(&buf).map_err(QueryPointerBufferSizeError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`query_pointer_buffer_size`].
#[derive(Debug, thiserror::Error)]
pub enum QueryPointerBufferSizeError {
    /// The request could not be serialized or the kernel rejected the send.
    #[error("failed to send IPC request")]
    SendRequest(#[source] cmif::SendError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for QueryPointerBufferSizeError {
    fn to_rc(self) -> ResultCode {
        match self {
            QueryPointerBufferSizeError::SendRequest(err) => err.to_rc(),
            QueryPointerBufferSizeError::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Clones the current session via control request 2.
pub fn clone_current_object(
    session: BorrowedSessionHandle<'_>,
) -> Result<OwnedSessionHandle, CloneObjectError> {
    // SAFETY: IPC operations are serialized on this thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    CmifControlRequestBuilder::new(CTRL_CLONE_OBJECT)
        .build()
        .send(&mut buf, session)
        .map_err(CloneObjectError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(CloneObjectError::ParseResponse)?;

    let raw = *resp
        .move_handles
        .first()
        .ok_or(CloneObjectError::MissingHandle)?;

    // SAFETY: The server answered with a freshly created session that this process now owns
    // and nothing else will close; adopting it here is the one place that ownership begins.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        SessionHandle::from_raw_unchecked(raw),
    ))
}

/// Error returned by [`clone_current_object`].
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectError {
    /// The request could not be serialized or the kernel rejected the send.
    #[error("failed to send IPC request")]
    SendRequest(#[source] cmif::SendError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// The server returned success but no move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

impl ToResultCode for CloneObjectError {
    fn to_rc(self) -> ResultCode {
        match self {
            CloneObjectError::SendRequest(err) => err.to_rc(),
            CloneObjectError::ParseResponse(err) => err.to_rc(),
            // The server reported success, so it named no code for a reply
            // that then arrived without the handle it promised.
            CloneObjectError::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Clones the current session with a session manager tag via control request 4.
pub fn clone_current_object_ex(
    session: BorrowedSessionHandle<'_>,
    tag: u32,
) -> Result<OwnedSessionHandle, CloneObjectExError> {
    // SAFETY: IPC operations are serialized on this thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let payload = tag.to_le_bytes();
    CmifControlRequestBuilder::new(CTRL_CLONE_OBJECT_EX)
        .data(&payload)
        .build()
        .send(&mut buf, session)
        .map_err(CloneObjectExError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(CloneObjectExError::ParseResponse)?;

    let raw = *resp
        .move_handles
        .first()
        .ok_or(CloneObjectExError::MissingHandle)?;

    // SAFETY: The server answered with a freshly created session that this process now owns
    // and nothing else will close; adopting it here is the one place that ownership begins.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        SessionHandle::from_raw_unchecked(raw),
    ))
}

/// Error returned by [`clone_current_object_ex`].
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectExError {
    /// The request could not be serialized or the kernel rejected the send.
    #[error("failed to send IPC request")]
    SendRequest(#[source] cmif::SendError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// The server returned success but no move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

impl ToResultCode for CloneObjectExError {
    fn to_rc(self) -> ResultCode {
        match self {
            CloneObjectExError::SendRequest(err) => err.to_rc(),
            CloneObjectExError::ParseResponse(err) => err.to_rc(),
            // The server reported success, so it named no code for a reply
            // that then arrived without the handle it promised.
            CloneObjectExError::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Converts the current session to a domain via control request 0.
pub fn convert_current_object_to_domain(
    session: BorrowedSessionHandle<'_>,
) -> Result<ObjectId, ConvertToDomainError> {
    // SAFETY: IPC operations are serialized on this thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    CmifControlRequestBuilder::new(CTRL_CONVERT_TO_DOMAIN)
        .build()
        .send(&mut buf, session)
        .map_err(ConvertToDomainError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(ConvertToDomainError::ParseResponse)?;

    // SAFETY: The server answers a successful ConvertToDomain with a non-zero object id.
    Ok(ObjectId::from_raw_unchecked(*resp.payload))
}

/// Error returned by [`convert_current_object_to_domain`].
#[derive(Debug, thiserror::Error)]
pub enum ConvertToDomainError {
    /// The request could not be serialized or the kernel rejected the send.
    #[error("failed to send IPC request")]
    SendRequest(#[source] cmif::SendError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ConvertToDomainError {
    fn to_rc(self) -> ResultCode {
        match self {
            ConvertToDomainError::SendRequest(err) => err.to_rc(),
            ConvertToDomainError::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Copies a domain object to a new standalone session via control request 1.
pub fn copy_from_current_domain(
    session: BorrowedSessionHandle<'_>,
    object_id: ObjectId,
) -> Result<OwnedSessionHandle, CopyFromDomainError> {
    // SAFETY: IPC operations are serialized on this thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let payload = object_id.to_raw().to_le_bytes();
    CmifControlRequestBuilder::new(CTRL_COPY_FROM_DOMAIN)
        .data(&payload)
        .build()
        .send(&mut buf, session)
        .map_err(CopyFromDomainError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(CopyFromDomainError::ParseResponse)?;

    let raw = *resp
        .move_handles
        .first()
        .ok_or(CopyFromDomainError::MissingHandle)?;

    // SAFETY: The server answered with a freshly created session that this process now owns
    // and nothing else will close; adopting it here is the one place that ownership begins.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        SessionHandle::from_raw_unchecked(raw),
    ))
}

/// Error returned by [`copy_from_current_domain`].
#[derive(Debug, thiserror::Error)]
pub enum CopyFromDomainError {
    /// The request could not be serialized or the kernel rejected the send.
    #[error("failed to send IPC request")]
    SendRequest(#[source] cmif::SendError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// The server returned success but no move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

impl ToResultCode for CopyFromDomainError {
    fn to_rc(self) -> ResultCode {
        match self {
            CopyFromDomainError::SendRequest(err) => err.to_rc(),
            CopyFromDomainError::ParseResponse(err) => err.to_rc(),
            // The server reported success, so it named no code for a reply
            // that then arrived without the handle it promised.
            CopyFromDomainError::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Sends a CMIF session-close request and closes the local handle.
///
/// Errors from either step are deliberately swallowed: a peer that has gone
/// away must not block the local side from releasing its kernel handle.
pub(crate) fn close_session(handle: BorrowedSessionHandle<'_>) {
    // SAFETY: IPC operations are serialized on this thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let _ = CmifCloseRequest::session().send(&mut buf, handle);
    let _ = ipc::close_handle(handle.to_handle());
}

/// Sends a CMIF domain-object close request on the parent session.
///
/// Does not close the session handle - only the named object inside the
/// domain. Errors are deliberately swallowed for the same reason as
/// [`close_session`].
pub(crate) fn close_object(session: BorrowedSessionHandle<'_>, object_id: ObjectId) {
    // SAFETY: IPC operations are serialized on this thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let _ = CmifCloseRequest::domain_object(object_id).send(&mut buf, session);
}
