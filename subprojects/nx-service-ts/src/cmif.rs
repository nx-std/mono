//! CMIF protocol operations for the temperature measurement service.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto;

/// Gets the temperature range for a location.
///
/// Returns `(min_temperature, max_temperature)` in Celsius.
pub fn get_temperature_range(
    session: SessionHandle,
    location: u8,
) -> Result<(i32, i32), GetTemperatureRangeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_TEMPERATURE_RANGE)
        .data_size(size_of::<u8>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), location);
    }

    ipc::send_sync_request(session).map_err(GetTemperatureRangeError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<[i32; 2]>()) }
        .map_err(GetTemperatureRangeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for two i32.
    let data = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<[i32; 2]>()) };

    Ok((data[0], data[1]))
}

/// Gets the temperature for a location, in Celsius.
pub fn get_temperature(session: SessionHandle, location: u8) -> Result<i32, GetTemperatureError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_TEMPERATURE)
        .data_size(size_of::<u8>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), location);
    }

    ipc::send_sync_request(session).map_err(GetTemperatureError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<i32>()) }
        .map_err(GetTemperatureError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let temp = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(temp)
}

/// Gets the temperature for a location, in millicelsius.
pub fn get_temperature_milli_c(
    session: SessionHandle,
    location: u8,
) -> Result<i32, GetTemperatureMilliCError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_TEMPERATURE_MILLI_C)
        .data_size(size_of::<u8>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), location);
    }

    ipc::send_sync_request(session).map_err(GetTemperatureMilliCError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<i32>()) }
        .map_err(GetTemperatureMilliCError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let temp = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(temp)
}

/// Opens a temperature session for a device code.
///
/// Returns a session handle for the opened device.
pub fn open_session(
    session: SessionHandle,
    device_code: u32,
) -> Result<SessionHandle, OpenSessionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_SESSION)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), device_code);
    }

    ipc::send_sync_request(session).map_err(OpenSessionError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSessionError::ParseResponse)?;

    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSessionError::MissingHandle)?;

    // SAFETY: handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the temperature from a session, as a float in Celsius.
pub fn session_get_temperature(session: SessionHandle) -> Result<f32, SessionGetTemperatureError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SESSION_GET_TEMPERATURE).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(SessionGetTemperatureError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<f32>()) }
        .map_err(SessionGetTemperatureError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f32.
    let temp = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(temp)
}

/// Error returned by [`get_temperature_range`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureRangeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_temperature`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by [`get_temperature_milli_c`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureMilliCError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
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
    /// Response did not contain the expected move handle.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`session_get_temperature`].
#[derive(Debug, thiserror::Error)]
pub enum SessionGetTemperatureError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
