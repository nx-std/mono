//! TIPC protocol operations for Service Manager.
//!
//! This module implements SM commands using the TIPC (Tiny IPC) protocol,
//! which is used on HOS 12.0.0+ and Atmosphere for certain operations.

use core::{mem::size_of, ptr};

use nx_sf::{ServiceName, cmif, tipc};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Gets a raw service handle by name using TIPC protocol.
///
/// Requires HOS 12.0.0+ or Atmosphere.
#[inline]
pub fn get_service_handle(
    session: SessionHandle,
    name: ServiceName,
) -> Result<SessionHandle, GetServiceError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcRequestBuilder::new(proto::GET_SERVICE_HANDLE)
            .data_size(size_of::<ServiceName>())
            .send(&mut buf)
            .map_err(GetServiceError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<ServiceName>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<ServiceName>(), name) };
    }

    ipc::send_sync_request(session).map_err(GetServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, 0).map_err(GetServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetServiceError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by [`get_service_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetServiceError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

/// Registers a service with the Service Manager using TIPC protocol.
#[inline]
pub fn register_service(
    session: SessionHandle,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<SessionHandle, RegisterServiceError> {
    #[repr(C)]
    struct RegisterServiceTipcIn {
        name: ServiceName,
        max_sessions: i32,
        is_light: u8,
    }

    let input = RegisterServiceTipcIn {
        name,
        max_sessions,
        is_light: u8::from(is_light),
    };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcRequestBuilder::new(proto::REGISTER_SERVICE)
            .data_size(size_of::<RegisterServiceTipcIn>())
            .send(&mut buf)
            .map_err(RegisterServiceError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<RegisterServiceTipcIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<RegisterServiceTipcIn>(), input);
        }
    }

    ipc::send_sync_request(session).map_err(RegisterServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, 0).map_err(RegisterServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(RegisterServiceError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by [`register_service`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterServiceError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

/// Unregisters a service from the Service Manager using TIPC protocol.
#[inline]
pub fn unregister_service(
    session: SessionHandle,
    name: ServiceName,
) -> Result<(), UnregisterServiceError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcRequestBuilder::new(proto::UNREGISTER_SERVICE)
            .data_size(size_of::<ServiceName>())
            .send(&mut buf)
            .map_err(UnregisterServiceError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<ServiceName>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<ServiceName>(), name) };
    }

    ipc::send_sync_request(session).map_err(UnregisterServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    tipc::parse_response(&buf, 0).map_err(UnregisterServiceError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_service`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterServiceError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

/// Detaches the client from the Service Manager using TIPC protocol.
///
/// Only available on Atmosphere.
#[inline]
pub fn detach_client(session: SessionHandle) -> Result<(), DetachClientError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        // The detach-client request carries no payload data.
        tipc::TipcRequestBuilder::new(proto::DETACH_CLIENT)
            .send_pid()
            .send(&mut buf)
            .map_err(DetachClientError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(DetachClientError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    tipc::parse_response(&buf, 0).map_err(DetachClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`detach_client`].
#[derive(Debug, thiserror::Error)]
pub enum DetachClientError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

/// Registers the client with the Service Manager using TIPC protocol.
///
/// Requires HOS 12.0.0+ or Atmosphere.
#[expect(dead_code)]
#[inline]
pub fn register_client(session: SessionHandle) -> Result<(), RegisterClientError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        // The register-client request carries no payload data.
        tipc::TipcRequestBuilder::new(proto::REGISTER_CLIENT)
            .send_pid()
            .send(&mut buf)
            .map_err(RegisterClientError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(RegisterClientError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    tipc::parse_response(&buf, 0).map_err(RegisterClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to build the TIPC request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}
