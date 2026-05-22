//! CMIF protocol operations for the temperature control service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle as SessionHandle},
};

use crate::proto;

/// Enables fan control.
pub fn enable_fan_control(session: SessionHandle) -> Result<(), EnableFanControlError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::ENABLE_FAN_CONTROL).build();
    req.write_to(&mut buf)
        .map_err(EnableFanControlError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(EnableFanControlError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(EnableFanControlError::ParseResponse)?;

    Ok(())
}

/// Disables fan control.
pub fn disable_fan_control(session: SessionHandle) -> Result<(), DisableFanControlError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::DISABLE_FAN_CONTROL).build();
    req.write_to(&mut buf)
        .map_err(DisableFanControlError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DisableFanControlError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(DisableFanControlError::ParseResponse)?;

    Ok(())
}

/// Queries whether fan control is enabled.
pub fn is_fan_control_enabled(session: SessionHandle) -> Result<bool, IsFanControlEnabledError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::IS_FAN_CONTROL_ENABLED).build();
    req.write_to(&mut buf)
        .map_err(IsFanControlEnabledError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(IsFanControlEnabledError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(IsFanControlEnabledError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

/// Gets the skin temperature in milli-degrees Celsius.
pub fn get_skin_temperature_milli_c(
    session: SessionHandle,
) -> Result<i32, GetSkinTemperatureMilliCError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let req = cmif::CmifRequestBuilder::new(proto::GET_SKIN_TEMPERATURE_MILLI_C).build();
    req.write_to(&mut buf)
        .map_err(GetSkinTemperatureMilliCError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(GetSkinTemperatureMilliCError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<i32>())
        .map_err(GetSkinTemperatureMilliCError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for i32.
    let temp = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<i32>()) };

    Ok(temp)
}

/// Error returned by [`enable_fan_control`].
#[derive(Debug, thiserror::Error)]
pub enum EnableFanControlError {
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

/// Error returned by [`disable_fan_control`].
#[derive(Debug, thiserror::Error)]
pub enum DisableFanControlError {
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

/// Error returned by [`is_fan_control_enabled`].
#[derive(Debug, thiserror::Error)]
pub enum IsFanControlEnabledError {
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

/// Error returned by [`get_skin_temperature_milli_c`].
#[derive(Debug, thiserror::Error)]
pub enum GetSkinTemperatureMilliCError {
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
