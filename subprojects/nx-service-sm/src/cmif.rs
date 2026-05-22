//! CMIF protocol operations for Service Manager.
//!
//! This module implements SM commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::{mem::size_of, ptr};

use nx_sf::{ServiceName, cmif, ipc};
use nx_svc::ipc::Handle as SessionHandle;

use crate::proto;

/// Gets a raw service handle by name using CMIF protocol.
#[inline]
pub fn get_service_handle(
    session: SessionHandle,
    name: ServiceName,
) -> Result<SessionHandle, GetServiceError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let mut payload = [0u8; size_of::<ServiceName>()];
    // SAFETY: `payload` is exactly `size_of::<ServiceName>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<ServiceName>(), name) };
    let req = cmif::CmifRequestBuilder::new(proto::GET_SERVICE_HANDLE)
        .data(&payload)
        .build();
    req.write_to(&mut buf)
        .map_err(GetServiceError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetServiceError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetServiceError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by [`get_service_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetServiceError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

/// Registers a service with the Service Manager using CMIF protocol.
#[inline]
pub fn register_service(
    session: SessionHandle,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<SessionHandle, RegisterServiceError> {
    #[repr(C)]
    struct RegisterServiceIn {
        name: ServiceName,
        is_light: u8,
        _pad: [u8; 3],
        max_sessions: i32,
    }

    let input = RegisterServiceIn {
        name,
        is_light: u8::from(is_light),
        _pad: [0; 3],
        max_sessions,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let mut payload = [0u8; size_of::<RegisterServiceIn>()];
    // SAFETY: `payload` is exactly `size_of::<RegisterServiceIn>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<RegisterServiceIn>(), input) };
    let req = cmif::CmifRequestBuilder::new(proto::REGISTER_SERVICE)
        .data(&payload)
        .build();
    req.write_to(&mut buf)
        .map_err(RegisterServiceError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(RegisterServiceError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(RegisterServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(RegisterServiceError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by [`register_service`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterServiceError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

/// Unregisters a service from the Service Manager using CMIF protocol.
#[inline]
pub fn unregister_service(
    session: SessionHandle,
    name: ServiceName,
) -> Result<(), UnregisterServiceError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let mut payload = [0u8; size_of::<ServiceName>()];
    // SAFETY: `payload` is exactly `size_of::<ServiceName>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<ServiceName>(), name) };
    let req = cmif::CmifRequestBuilder::new(proto::UNREGISTER_SERVICE)
        .data(&payload)
        .build();
    req.write_to(&mut buf)
        .map_err(UnregisterServiceError::BuildRequest)?;
    ipc::send_sync_request(&mut buf, session).map_err(UnregisterServiceError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(UnregisterServiceError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_service`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterServiceError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Detaches the client from the Service Manager using CMIF protocol.
///
/// Only available on HOS 11.0.0-11.0.1.
#[inline]
pub fn detach_client(session: SessionHandle) -> Result<(), DetachClientError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let payload = [0u8; size_of::<u64>()];
    let req = cmif::CmifRequestBuilder::new(proto::DETACH_CLIENT)
        .data(&payload)
        .send_pid()
        .build();
    req.write_to(&mut buf)
        .map_err(DetachClientError::BuildRequest)?;
    ipc::send_sync_request(&mut buf, session).map_err(DetachClientError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DetachClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`detach_client`].
#[derive(Debug, thiserror::Error)]
pub enum DetachClientError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}

/// Registers the client with the Service Manager using CMIF protocol.
///
/// Sends the RegisterClient command (cmd 0) with PID.
#[inline]
pub fn register_client(session: SessionHandle) -> Result<(), RegisterClientError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let payload = [0u8; size_of::<u64>()];
    let req = cmif::CmifRequestBuilder::new(proto::REGISTER_CLIENT)
        .data(&payload)
        .send_pid()
        .build();
    req.write_to(&mut buf)
        .map_err(RegisterClientError::BuildRequest)?;
    ipc::send_sync_request(&mut buf, session).map_err(RegisterClientError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RegisterClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespError),
}
