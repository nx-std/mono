//! CMIF protocol operations for APM service.
//!
//! This module implements APM commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::mem::size_of;

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
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
    req.write_to(&mut buf)
        .map_err(OpenSessionError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(OpenSessionError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, 0).map_err(OpenSessionError::ParseResponse)?;

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
    req.write_to(&mut buf)
        .map_err(GetPerformanceModeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetPerformanceModeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
        .map_err(GetPerformanceModeError::ParseResponse)?;

    if resp.data.len() < 4 {
        return Err(GetPerformanceModeError::InvalidResponse);
    }

    let raw_mode = i32::from_le_bytes([resp.data[0], resp.data[1], resp.data[2], resp.data[3]]);

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
        .data_value(&in_data)
        .build();
    req.write_to(&mut buf)
        .map_err(SetPerformanceConfigurationError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(SetPerformanceConfigurationError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(SetPerformanceConfigurationError::ParseResponse)?;

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
        .data_value(&mode)
        .build();
    req.write_to(&mut buf)
        .map_err(GetPerformanceConfigurationError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetPerformanceConfigurationError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(GetPerformanceConfigurationError::ParseResponse)?;

    if resp.data.len() < 4 {
        return Err(GetPerformanceConfigurationError::InvalidResponse);
    }

    let config = u32::from_le_bytes([resp.data[0], resp.data[1], resp.data[2], resp.data[3]]);

    Ok(config)
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`get_performance_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetPerformanceModeError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response data was too short.
    #[error("invalid response data")]
    InvalidResponse,
    /// Invalid performance mode value.
    #[error("invalid performance mode: {0}")]
    InvalidMode(i32),
}

/// Error returned by [`set_performance_configuration`].
#[derive(Debug, thiserror::Error)]
pub enum SetPerformanceConfigurationError {
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

/// Error returned by [`get_performance_configuration`].
#[derive(Debug, thiserror::Error)]
pub enum GetPerformanceConfigurationError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// Response data was too short.
    #[error("invalid response data")]
    InvalidResponse,
}
