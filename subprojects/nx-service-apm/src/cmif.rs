//! CMIF protocol operations for APM service.
//!
//! This module implements APM commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use nx_sf::{
    cmif,
    error::{GENERIC_ERROR, ResultCode, ToResultCode},
    ipc::Handle as SessionHandle,
};

use crate::proto::{
    CMD_GET_PERFORMANCE_CONFIGURATION, CMD_GET_PERFORMANCE_MODE, CMD_OPEN_SESSION,
    CMD_SET_PERFORMANCE_CONFIGURATION, PerformanceMode,
};

/// Opens an APM session for performance configuration.
///
/// This is IManager command 0.
pub fn open_session(session: SessionHandle) -> Result<SessionHandle, OpenSessionError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(CMD_OPEN_SESSION).build();
    req.send(&mut buf, session)
        .map_err(OpenSessionError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenSessionError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(OpenSessionError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid session handle in the response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the current performance mode.
///
/// This is IManager command 1.
pub fn get_performance_mode(
    session: SessionHandle,
) -> Result<PerformanceMode, GetPerformanceModeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(CMD_GET_PERFORMANCE_MODE).build();
    req.send(&mut buf, session)
        .map_err(GetPerformanceModeError::SendRequest)?;

    let resp =
        cmif::parse_response::<&i32>(&buf).map_err(GetPerformanceModeError::ParseResponse)?;
    let raw_mode = *resp.payload;

    PerformanceMode::from_raw(raw_mode).ok_or(GetPerformanceModeError::InvalidMode(raw_mode))
}

/// Sets the performance configuration for a given mode.
///
/// This is ISession command 0.
pub fn set_performance_configuration(
    session: SessionHandle,
    mode: PerformanceMode,
    config: u32,
) -> Result<(), SetPerformanceConfigurationError> {
    #[repr(C)]
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    struct InData {
        mode: PerformanceMode,
        config: u32,
    }

    let in_data = InData { mode, config };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(CMD_SET_PERFORMANCE_CONFIGURATION)
        .with_data_value(&in_data)
        .build();
    req.send(&mut buf, session)
        .map_err(SetPerformanceConfigurationError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetPerformanceConfigurationError::ParseResponse)?;

    Ok(())
}

/// Gets the performance configuration for a given mode.
///
/// This is ISession command 1.
pub fn get_performance_configuration(
    session: SessionHandle,
    mode: PerformanceMode,
) -> Result<u32, GetPerformanceConfigurationError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(CMD_GET_PERFORMANCE_CONFIGURATION)
        .with_data_value(&mode)
        .build();
    req.send(&mut buf, session)
        .map_err(GetPerformanceConfigurationError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf)
        .map_err(GetPerformanceConfigurationError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

impl ToResultCode for OpenSessionError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`get_performance_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetPerformanceModeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Invalid performance mode value.
    #[error("invalid performance mode: {0}")]
    InvalidMode(i32),
}

impl ToResultCode for GetPerformanceModeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::InvalidMode(_) => GENERIC_ERROR,
        }
    }
}

/// Error returned by [`set_performance_configuration`].
#[derive(Debug, thiserror::Error)]
pub enum SetPerformanceConfigurationError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetPerformanceConfigurationError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by [`get_performance_configuration`].
#[derive(Debug, thiserror::Error)]
pub enum GetPerformanceConfigurationError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for GetPerformanceConfigurationError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
