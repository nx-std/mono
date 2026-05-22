//! CMIF protocol operations for the temperature measurement service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Gets the temperature range for a location.
///
/// Returns `(min_temperature, max_temperature)` in Celsius.
pub fn get_temperature_range(
    session: SessionHandle,
    location: u8,
) -> Result<(i32, i32), GetTemperatureRangeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_TEMPERATURE_RANGE)
        .data_value(&location)
        .build();
    req.write_to(&mut buf)
        .map_err(GetTemperatureRangeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetTemperatureRangeError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<[i32; 2]>())
        .map_err(GetTemperatureRangeError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for two i32.
    let data = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<[i32; 2]>()) };

    Ok((data[0], data[1]))
}

/// Gets the temperature for a location, in Celsius.
pub fn get_temperature(session: SessionHandle, location: u8) -> Result<i32, GetTemperatureError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_TEMPERATURE)
        .data_value(&location)
        .build();
    req.write_to(&mut buf)
        .map_err(GetTemperatureError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetTemperatureError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::GET_TEMPERATURE_MILLI_C)
        .data_value(&location)
        .build();
    req.write_to(&mut buf)
        .map_err(GetTemperatureMilliCError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(GetTemperatureMilliCError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::OPEN_SESSION)
        .data_value(&device_code)
        .build();
    req.write_to(&mut buf)
        .map_err(OpenSessionError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(OpenSessionError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, 0).map_err(OpenSessionError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(OpenSessionError::MissingHandle);
    };

    // SAFETY: handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the temperature from a session, as a float in Celsius.
pub fn session_get_temperature(session: SessionHandle) -> Result<f32, SessionGetTemperatureError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::SESSION_GET_TEMPERATURE).build();
    req.write_to(&mut buf)
        .map_err(SessionGetTemperatureError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SessionGetTemperatureError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<f32>())
        .map_err(SessionGetTemperatureError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for f32.
    let temp = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<f32>()) };

    Ok(temp)
}

/// Error returned by [`get_temperature_range`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureRangeError {
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

/// Error returned by [`get_temperature`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureError {
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

/// Error returned by [`get_temperature_milli_c`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureMilliCError {
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
    /// Response did not contain the expected move handle.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`session_get_temperature`].
#[derive(Debug, thiserror::Error)]
pub enum SessionGetTemperatureError {
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
