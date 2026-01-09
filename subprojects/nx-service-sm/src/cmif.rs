//! CMIF protocol operations for Service Manager.
//!
//! This module implements SM commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::{mem::size_of, ptr};

use nx_sf::{ServiceName, cmif};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Gets a raw service handle by name using CMIF protocol.
#[inline]
pub fn get_service_handle(
    session: SessionHandle,
    name: ServiceName,
) -> Result<SessionHandle, GetServiceError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: proto::GET_SERVICE_HANDLE,
        context: 0,
        data_size: size_of::<ServiceName>(),
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for ServiceName.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<ServiceName>().cast_mut(), name);
    }

    ipc::send_sync_request(session).map_err(GetServiceError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetServiceError::ParseResponse)?;

    if resp.move_handles.is_empty() {
        return Err(GetServiceError::MissingHandle);
    }

    // SAFETY: Kernel returned a valid handle in the response.
    Ok(unsafe { SessionHandle::from_raw(resp.move_handles[0]) })
}

/// Error returned by [`get_service_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

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

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: proto::REGISTER_SERVICE,
        context: 0,
        data_size: size_of::<RegisterServiceIn>(),
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area.
    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<RegisterServiceIn>().cast_mut(),
            input,
        );
    }

    ipc::send_sync_request(session).map_err(RegisterServiceError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RegisterServiceError::ParseResponse)?;

    if resp.move_handles.is_empty() {
        return Err(RegisterServiceError::MissingHandle);
    }

    // SAFETY: Kernel returned a valid handle in the response.
    Ok(unsafe { SessionHandle::from_raw(resp.move_handles[0]) })
}

/// Error returned by [`register_service`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: proto::UNREGISTER_SERVICE,
        context: 0,
        data_size: size_of::<ServiceName>(),
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<ServiceName>().cast_mut(), name);
    }

    ipc::send_sync_request(session).map_err(UnregisterServiceError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(UnregisterServiceError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_service`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Detaches the client from the Service Manager using CMIF protocol.
///
/// Only available on HOS 11.0.0-11.0.1.
#[inline]
pub fn detach_client(session: SessionHandle) -> Result<(), DetachClientError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: proto::DETACH_CLIENT,
        context: 0,
        data_size: size_of::<u64>(),
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: true,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), 0u64);
    }

    ipc::send_sync_request(session).map_err(DetachClientError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DetachClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`detach_client`].
#[derive(Debug, thiserror::Error)]
pub enum DetachClientError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Registers the client with the Service Manager using CMIF protocol.
///
/// Sends the RegisterClient command (cmd 0) with PID.
#[inline]
pub fn register_client(session: SessionHandle) -> Result<(), RegisterClientError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: proto::REGISTER_CLIENT,
        context: 0,
        data_size: size_of::<u64>(),
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: true,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), 0u64);
    }

    ipc::send_sync_request(session).map_err(RegisterClientError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(RegisterClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
