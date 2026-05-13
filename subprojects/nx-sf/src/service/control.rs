//! CMIF control-request helpers used by the typed service wrappers and FFI.
//!
//! Each function corresponds to one of the standard CMIF control request IDs
//! (0..=4) and is a stateless protocol primitive — no caller state is held.
//!
//! Errors during the close-style helpers ([`close_session`], [`close_object`])
//! are deliberately swallowed: matching libnx and Horizon OS conventions, a
//! peer that has already gone away must not prevent the local side from
//! releasing its resources. Drop impls and the FFI close symbols both invoke
//! these.

use core::{mem::size_of, ptr};

use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::cmif::{self, ObjectId};

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
    session: SessionHandle,
) -> Result<u16, QueryPointerBufferSizeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to the current thread's IPC buffer.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_QUERY_POINTER_BUFFER_SIZE, 0) };

    ipc::send_sync_request(session).map_err(QueryPointerBufferSizeError::SendRequest)?;

    // SAFETY: Response is in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u16>()) }
        .map_err(QueryPointerBufferSizeError::ParseResponse)?;

    // SAFETY: Response data layout for control request 3 is a single u16.
    let size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u16>()) };
    Ok(size)
}

/// Error returned by [`query_pointer_buffer_size`].
#[derive(Debug, thiserror::Error)]
pub enum QueryPointerBufferSizeError {
    /// The kernel rejected the underlying `SendSyncRequest`.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Clones the current session via control request 2.
pub fn clone_current_object(session: SessionHandle) -> Result<SessionHandle, CloneObjectError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to the current thread's IPC buffer.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT, 0) };

    ipc::send_sync_request(session).map_err(CloneObjectError::SendRequest)?;

    // SAFETY: Response is in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CloneObjectError::ParseResponse)?;

    let raw = *resp
        .move_handles
        .first()
        .ok_or(CloneObjectError::MissingHandle)?;

    // SAFETY: The kernel returned the handle as a move handle in the response.
    Ok(unsafe { SessionHandle::from_raw(raw) })
}

/// Error returned by [`clone_current_object`].
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectError {
    /// The kernel rejected the underlying `SendSyncRequest`.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// The server returned success but no move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

/// Clones the current session with a session manager tag via control request 4.
pub fn clone_current_object_ex(
    session: SessionHandle,
    tag: u32,
) -> Result<SessionHandle, CloneObjectExError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to the current thread's IPC buffer.
    let data_ptr = unsafe {
        cmif::make_control_request(ipc_buf, CTRL_CLONE_OBJECT_EX, size_of::<u32>() as u32)
    };

    // SAFETY: data_ptr points to the payload region for the in header.
    unsafe { ptr::write_unaligned(data_ptr.cast::<u32>(), tag) };

    ipc::send_sync_request(session).map_err(CloneObjectExError::SendRequest)?;

    // SAFETY: Response is in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CloneObjectExError::ParseResponse)?;

    let raw = *resp
        .move_handles
        .first()
        .ok_or(CloneObjectExError::MissingHandle)?;

    // SAFETY: The kernel returned the handle as a move handle in the response.
    Ok(unsafe { SessionHandle::from_raw(raw) })
}

/// Error returned by [`clone_current_object_ex`].
#[derive(Debug, thiserror::Error)]
pub enum CloneObjectExError {
    /// The kernel rejected the underlying `SendSyncRequest`.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// The server returned success but no move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

/// Converts the current session to a domain via control request 0.
pub fn convert_current_object_to_domain(
    session: SessionHandle,
) -> Result<ObjectId, ConvertToDomainError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to the current thread's IPC buffer.
    unsafe { cmif::make_control_request(ipc_buf, CTRL_CONVERT_TO_DOMAIN, 0) };

    ipc::send_sync_request(session).map_err(ConvertToDomainError::SendRequest)?;

    // SAFETY: Response is in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(ConvertToDomainError::ParseResponse)?;

    // SAFETY: Response data layout for control request 0 is the new u32 object id.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    // SAFETY: The kernel always returns a non-zero object id on success here.
    Ok(unsafe { ObjectId::new_unchecked(raw) })
}

/// Error returned by [`convert_current_object_to_domain`].
#[derive(Debug, thiserror::Error)]
pub enum ConvertToDomainError {
    /// The kernel rejected the underlying `SendSyncRequest`.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Copies a domain object to a new standalone session via control request 1.
pub fn copy_from_current_domain(
    session: SessionHandle,
    object_id: ObjectId,
) -> Result<SessionHandle, CopyFromDomainError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to the current thread's IPC buffer.
    let data_ptr = unsafe {
        cmif::make_control_request(ipc_buf, CTRL_COPY_FROM_DOMAIN, size_of::<u32>() as u32)
    };

    // SAFETY: data_ptr points to the payload region for the in header.
    unsafe { ptr::write_unaligned(data_ptr.cast::<u32>(), object_id.to_raw()) };

    ipc::send_sync_request(session).map_err(CopyFromDomainError::SendRequest)?;

    // SAFETY: Response is in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(CopyFromDomainError::ParseResponse)?;

    let raw = *resp
        .move_handles
        .first()
        .ok_or(CopyFromDomainError::MissingHandle)?;

    // SAFETY: The kernel returned the handle as a move handle in the response.
    Ok(unsafe { SessionHandle::from_raw(raw) })
}

/// Error returned by [`copy_from_current_domain`].
#[derive(Debug, thiserror::Error)]
pub enum CopyFromDomainError {
    /// The kernel rejected the underlying `SendSyncRequest`.
    #[error("failed to send IPC request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// The response header did not pass CMIF validation.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// The server returned success but no move handle.
    #[error("missing move handle in response")]
    MissingHandle,
}

/// Sends a CMIF session-close request and closes the local handle.
///
/// Errors from either step are deliberately swallowed: a peer that has gone
/// away must not block the local side from releasing its kernel handle.
pub(crate) fn close_session(handle: SessionHandle) {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to the current thread's IPC buffer.
    unsafe { cmif::make_close_request(ipc_buf, None) };

    let _ = ipc::send_sync_request(handle);
    let _ = ipc::close_handle(handle);
}

/// Sends a CMIF domain-object close request on the parent session.
///
/// Does not close the session handle — only the named object inside the
/// domain. Errors are deliberately swallowed for the same reason as
/// [`close_session`].
pub(crate) fn close_object(session: SessionHandle, object_id: ObjectId) {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // SAFETY: ipc_buf points to the current thread's IPC buffer.
    unsafe { cmif::make_close_request(ipc_buf, Some(object_id)) };

    let _ = ipc::send_sync_request(session);
}
