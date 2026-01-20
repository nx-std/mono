//! CMIF protocol operations for APM service.
//!
//! This module implements APM commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto::{
    CMD_GET_PERFORMANCE_CONFIGURATION, CMD_GET_PERFORMANCE_MODE, CMD_OPEN_SESSION,
    CMD_SET_PERFORMANCE_CONFIGURATION, PerformanceMode,
};

/// Opens an APM session for performance configuration.
///
/// This is IManager command 0.
pub fn open_session(session: SessionHandle) -> Result<SessionHandle, OpenSessionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(CMD_OPEN_SESSION).build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(OpenSessionError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSessionError::ParseResponse)?;

    // Extract the move handle from response
    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSessionError::MissingHandle)?;

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the current performance mode.
///
/// This is IManager command 1.
pub fn get_performance_mode(
    session: SessionHandle,
) -> Result<PerformanceMode, GetPerformanceModeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(CMD_GET_PERFORMANCE_MODE).build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetPerformanceModeError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 4) }
        .map_err(GetPerformanceModeError::ParseResponse)?;

    // Response contains a single i32
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    #[repr(C)]
    struct InData {
        mode: u32,
        config: u32,
    }

    let in_data = InData {
        mode: mode as i32 as u32,
        config,
    };

    let fmt = cmif::RequestFormatBuilder::new(CMD_SET_PERFORMANCE_CONFIGURATION)
        .data_size(8) // Two u32 values
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for InData.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<InData>().cast_mut(), in_data);
    }

    ipc::send_sync_request(session).map_err(SetPerformanceConfigurationError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetPerformanceConfigurationError::ParseResponse)?;

    Ok(())
}

/// Gets the performance configuration for a given mode.
///
/// This is ISession command 1.
pub fn get_performance_configuration(
    session: SessionHandle,
    mode: PerformanceMode,
) -> Result<u32, GetPerformanceConfigurationError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let in_data: u32 = mode as i32 as u32;

    let fmt = cmif::RequestFormatBuilder::new(CMD_GET_PERFORMANCE_CONFIGURATION)
        .data_size(4) // One u32 input
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), in_data);
    }

    ipc::send_sync_request(session).map_err(GetPerformanceConfigurationError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 4) }
        .map_err(GetPerformanceConfigurationError::ParseResponse)?;

    // Response contains a single u32
    if resp.data.len() < 4 {
        return Err(GetPerformanceConfigurationError::InvalidResponse);
    }

    let config = u32::from_le_bytes([resp.data[0], resp.data[1], resp.data[2], resp.data[3]]);

    Ok(config)
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`get_performance_mode`].
#[derive(Debug, thiserror::Error)]
pub enum GetPerformanceModeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
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
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_performance_configuration`].
#[derive(Debug, thiserror::Error)]
pub enum GetPerformanceConfigurationError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Response data was too short.
    #[error("invalid response data")]
    InvalidResponse,
}
