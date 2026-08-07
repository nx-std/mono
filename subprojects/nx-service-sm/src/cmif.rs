//! CMIF protocol operations for Service Manager.
//!
//! This module implements SM commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use nx_sf::{
    ServiceName,
    cmif,
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
};
use nx_svc::error::ResultCode;
use zerocopy::IntoBytes as _;

use crate::proto;

/// Gets a raw service handle by name using CMIF protocol.
#[inline]
pub fn get_service_handle(
    session: BorrowedSessionHandle<'_>,
    name: ServiceName,
) -> Result<OwnedSessionHandle, GetServiceError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_SERVICE_HANDLE)
        .with_data(name.as_bytes_raw())
        .build();
    req.send(&mut buf, session)
        .map_err(GetServiceError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetServiceError::MissingHandle);
    };

    // SAFETY: `sm` answered with a freshly opened session that nothing else closes, so the
    // caller becomes its sole owner.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(handle),
    ))
}

/// Error returned by [`get_service_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

impl ToResultCode for GetServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // The server reported success, so it named no code for a reply
            // that then arrived without the handle it promised.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Registers a service with the Service Manager using CMIF protocol.
#[inline]
pub fn register_service(
    session: BorrowedSessionHandle<'_>,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<OwnedSessionHandle, RegisterServiceError> {
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::REGISTER_SERVICE)
        .with_data(input.as_bytes())
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterServiceError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(RegisterServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(RegisterServiceError::MissingHandle);
    };

    // SAFETY: `sm` answered with a freshly opened session that nothing else closes, so the
    // caller becomes its sole owner.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(handle),
    ))
}

/// Error returned by [`register_service`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Response did not contain the expected handle.
    #[error("missing handle in response")]
    MissingHandle,
}

impl ToResultCode for RegisterServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // The server reported success, so it named no code for a reply
            // that then arrived without the handle it promised.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Unregisters a service from the Service Manager using CMIF protocol.
#[inline]
pub fn unregister_service(
    session: BorrowedSessionHandle<'_>,
    name: ServiceName,
) -> Result<(), UnregisterServiceError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::UNREGISTER_SERVICE)
        .with_data(name.as_bytes_raw())
        .build();
    req.send(&mut buf, session)
        .map_err(UnregisterServiceError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(UnregisterServiceError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_service`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for UnregisterServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Detaches the client from the Service Manager using CMIF protocol.
///
/// Only available on HOS 11.0.0-11.0.1.
#[inline]
pub fn detach_client(session: BorrowedSessionHandle<'_>) -> Result<(), DetachClientError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let payload = [0u8; size_of::<u64>()];
    let req = cmif::CmifRequestBuilder::new(proto::DETACH_CLIENT)
        .with_data(&payload)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(DetachClientError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DetachClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`detach_client`].
#[derive(Debug, thiserror::Error)]
pub enum DetachClientError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for DetachClientError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Registers the client with the Service Manager using CMIF protocol.
///
/// Sends the RegisterClient command (cmd 0) with PID.
#[inline]
pub fn register_client(session: BorrowedSessionHandle<'_>) -> Result<(), RegisterClientError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let payload = [0u8; size_of::<u64>()];
    let req = cmif::CmifRequestBuilder::new(proto::REGISTER_CLIENT)
        .with_data(&payload)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterClientError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(RegisterClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for RegisterClientError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
