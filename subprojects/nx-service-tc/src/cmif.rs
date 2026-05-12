//! CMIF protocol operations for the temperature control service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Enables fan control.
pub fn enable_fan_control(session: SessionHandle) -> Result<(), EnableFanControlError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::ENABLE_FAN_CONTROL).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(EnableFanControlError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(EnableFanControlError::ParseResponse)?;

    Ok(())
}

/// Disables fan control.
pub fn disable_fan_control(session: SessionHandle) -> Result<(), DisableFanControlError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::DISABLE_FAN_CONTROL).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DisableFanControlError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DisableFanControlError::ParseResponse)?;

    Ok(())
}

/// Queries whether fan control is enabled.
pub fn is_fan_control_enabled(session: SessionHandle) -> Result<bool, IsFanControlEnabledError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::IS_FAN_CONTROL_ENABLED).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(IsFanControlEnabledError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(IsFanControlEnabledError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

/// Gets the skin temperature in milli-degrees Celsius.
pub fn get_skin_temperature_milli_c(
    session: SessionHandle,
) -> Result<i32, GetSkinTemperatureMilliCError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SKIN_TEMPERATURE_MILLI_C).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSkinTemperatureMilliCError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<i32>()) }
        .map_err(GetSkinTemperatureMilliCError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let temp = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(temp)
}

/// Error returned by [`enable_fan_control`].
#[derive(Debug, thiserror::Error)]
pub enum EnableFanControlError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`disable_fan_control`].
#[derive(Debug, thiserror::Error)]
pub enum DisableFanControlError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`is_fan_control_enabled`].
#[derive(Debug, thiserror::Error)]
pub enum IsFanControlEnabledError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_skin_temperature_milli_c`].
#[derive(Debug, thiserror::Error)]
pub enum GetSkinTemperatureMilliCError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
