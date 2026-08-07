//! TIPC protocol operations for Service Manager.
//!
//! This module implements SM commands using the TIPC (Tiny IPC) protocol,
//! which is used on HOS 12.0.0+ and Atmosphere for certain operations.

use nx_sf::{
    ServiceName,
    error::{
        GENERIC_ERROR,
        ToResultCode,
    },
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
    tipc,
};
use nx_svc::error::ResultCode;
use zerocopy::IntoBytes as _;

use crate::proto;

/// Gets a raw service handle by name using TIPC protocol.
///
/// Requires HOS 12.0.0+ or Atmosphere.
#[inline]
pub fn get_service_handle(
    session: BorrowedSessionHandle<'_>,
    name: ServiceName,
) -> Result<OwnedSessionHandle, GetServiceError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = tipc::TipcRequestBuilder::new(proto::GET_SERVICE_HANDLE)
        .with_data(name.as_bytes_raw())
        .build();
    req.send(&mut buf, session)
        .map_err(GetServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let resp = tipc::parse_response::<()>(&buf).map_err(GetServiceError::ParseResponse)?;

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
    SendRequest(#[source] tipc::SendError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
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

/// Registers a service with the Service Manager using TIPC protocol.
#[inline]
pub fn register_service(
    session: BorrowedSessionHandle<'_>,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> Result<OwnedSessionHandle, RegisterServiceError> {
    // `is_light` ends the struct one byte into its final 4-byte word, so the encoder used to
    // copy three trailing padding bytes it had never written, publishing whatever the stack
    // held to `sm`. `_pad` gives those bytes a name and a zero, which is also what lets
    // `IntoBytes` accept the type at all.
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct RegisterServiceTipcIn {
        name: ServiceName,
        max_sessions: i32,
        is_light: u8,
        _pad: [u8; 3],
    }

    let input = RegisterServiceTipcIn {
        name,
        max_sessions,
        is_light: u8::from(is_light),
        _pad: [0; 3],
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = tipc::TipcRequestBuilder::new(proto::REGISTER_SERVICE)
        .with_data(input.as_bytes())
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let resp = tipc::parse_response::<()>(&buf).map_err(RegisterServiceError::ParseResponse)?;

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
    SendRequest(#[source] tipc::SendError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
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

/// Unregisters a service from the Service Manager using TIPC protocol.
#[inline]
pub fn unregister_service(
    session: BorrowedSessionHandle<'_>,
    name: ServiceName,
) -> Result<(), UnregisterServiceError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = tipc::TipcRequestBuilder::new(proto::UNREGISTER_SERVICE)
        .with_data(name.as_bytes_raw())
        .build();
    req.send(&mut buf, session)
        .map_err(UnregisterServiceError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    tipc::parse_response::<()>(&buf).map_err(UnregisterServiceError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`unregister_service`].
#[derive(Debug, thiserror::Error)]
pub enum UnregisterServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

impl ToResultCode for UnregisterServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Detaches the client from the Service Manager using TIPC protocol.
///
/// Only available on Atmosphere.
#[inline]
pub fn detach_client(session: BorrowedSessionHandle<'_>) -> Result<(), DetachClientError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    // The detach-client request carries no payload data.
    tipc::TipcRequestBuilder::new(proto::DETACH_CLIENT)
        .with_send_pid()
        .build()
        .send(&mut buf, session)
        .map_err(DetachClientError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    tipc::parse_response::<()>(&buf).map_err(DetachClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`detach_client`].
#[derive(Debug, thiserror::Error)]
pub enum DetachClientError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

impl ToResultCode for DetachClientError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Registers the client with the Service Manager using TIPC protocol.
///
/// Requires HOS 12.0.0+ or Atmosphere.
#[inline]
pub fn register_client(session: BorrowedSessionHandle<'_>) -> Result<(), RegisterClientError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    // The register-client request carries no payload data.
    tipc::TipcRequestBuilder::new(proto::REGISTER_CLIENT)
        .with_send_pid()
        .build()
        .send(&mut buf, session)
        .map_err(RegisterClientError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    tipc::parse_response::<()>(&buf).map_err(RegisterClientError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    /// Failed to parse the TIPC response.
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

impl ToResultCode for RegisterClientError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
