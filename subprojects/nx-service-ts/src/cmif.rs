//! CMIF protocol operations for the temperature measurement service.

use nx_sf::{
    cmif,
    ipc::Handle as RawSessionHandle,
    service::{
        BorrowedSessionHandle,
        OwnedSessionHandle,
    },
};

use crate::proto;

/// Gets the temperature range for a location.
///
/// Returns `(min_temperature, max_temperature)` in Celsius.
pub fn get_temperature_range(
    session: BorrowedSessionHandle<'_>,
    location: u8,
) -> Result<(i32, i32), GetTemperatureRangeError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_TEMPERATURE_RANGE)
        .with_data_value(&location)
        .build();
    req.send(&mut buf, session)
        .map_err(GetTemperatureRangeError::SendRequest)?;

    let resp =
        cmif::parse_response::<&[i32; 2]>(&buf).map_err(GetTemperatureRangeError::ParseResponse)?;

    let data = *resp.payload;

    Ok((data[0], data[1]))
}

/// Gets the temperature for a location, in Celsius.
pub fn get_temperature(
    session: BorrowedSessionHandle<'_>,
    location: u8,
) -> Result<i32, GetTemperatureError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_TEMPERATURE)
        .with_data_value(&location)
        .build();
    req.send(&mut buf, session)
        .map_err(GetTemperatureError::SendRequest)?;

    let resp = cmif::parse_response::<&i32>(&buf).map_err(GetTemperatureError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Gets the temperature for a location, in millicelsius.
pub fn get_temperature_milli_c(
    session: BorrowedSessionHandle<'_>,
    location: u8,
) -> Result<i32, GetTemperatureMilliCError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_TEMPERATURE_MILLI_C)
        .with_data_value(&location)
        .build();
    req.send(&mut buf, session)
        .map_err(GetTemperatureMilliCError::SendRequest)?;

    let resp =
        cmif::parse_response::<&i32>(&buf).map_err(GetTemperatureMilliCError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Opens a temperature session for a device code.
///
/// Returns a session handle for the opened device.
pub fn open_session(
    session: BorrowedSessionHandle<'_>,
    device_code: u32,
) -> Result<OwnedSessionHandle, OpenSessionError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::OPEN_SESSION)
        .with_data_value(&device_code)
        .build();
    req.send(&mut buf, session)
        .map_err(OpenSessionError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenSessionError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(OpenSessionError::MissingHandle);
    };

    // SAFETY: handle is from a valid IPC response.
    Ok(OwnedSessionHandle::from_handle_unchecked(
        RawSessionHandle::from_raw_unchecked(handle),
    ))
}

/// Gets the temperature from a session, as a float in Celsius.
pub fn session_get_temperature(
    session: BorrowedSessionHandle<'_>,
) -> Result<f32, SessionGetTemperatureError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::SESSION_GET_TEMPERATURE).build();
    req.send(&mut buf, session)
        .map_err(SessionGetTemperatureError::SendRequest)?;

    let resp =
        cmif::parse_response::<&f32>(&buf).map_err(SessionGetTemperatureError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`get_temperature_range`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureRangeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_temperature`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Error returned by [`get_temperature_milli_c`].
#[derive(Debug, thiserror::Error)]
pub enum GetTemperatureMilliCError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
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
    /// Response did not contain the expected move handle.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`session_get_temperature`].
#[derive(Debug, thiserror::Error)]
pub enum SessionGetTemperatureError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}
