//! CMIF protocol operations for the GPIO service.

use core::ptr;

use nx_sf::{
    cmif,
    service::{DispatchError, OutHandleAttr, Session},
};
use nx_svc::ipc::{self, Handle};

use crate::proto;

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_no_io(session: Handle, cmd_id: u32) -> Result<(), DispatchNoIoError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchNoIoError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(DispatchNoIoError::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchNoIoError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_in_u32(session: Handle, cmd_id: u32, value: u32) -> Result<(), DispatchInU32Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), value);
    }

    ipc::send_sync_request(session).map_err(DispatchInU32Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DispatchInU32Error::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32Error {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_in_bool(session: Handle, cmd_id: u32, value: bool) -> Result<(), DispatchInBoolError> {
    let raw: u8 = value as u8;
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<u8>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u8.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), raw);
    }

    ipc::send_sync_request(session).map_err(DispatchInBoolError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(DispatchInBoolError::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_out_u32(session: Handle, cmd_id: u32) -> Result<u32, DispatchOutU32Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutU32Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(DispatchOutU32Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u32.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(value)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutU32Error {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_out_bool(session: Handle, cmd_id: u32) -> Result<bool, DispatchOutBoolError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id).build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchOutBoolError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(DispatchOutBoolError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

fn dispatch_in_u32_out_bool(
    session: Handle,
    cmd_id: u32,
    value: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), value);
    }

    ipc::send_sync_request(session).map_err(DispatchInU32OutBoolError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u8>()) }
        .map_err(DispatchInU32OutBoolError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32OutBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// Manager commands
// ---------------------------------------------------------------------------

/// Opens a GPIO pad session by pad name.
pub fn open_session(session: Handle, pad_name: u32) -> Result<Session, OpenSessionError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_SESSION)
        .data_size(size_of::<u32>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for u32.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), pad_name);
    }

    ipc::send_sync_request(session).map_err(OpenSessionError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSessionError::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSessionError::MissingHandle)?;

    // SAFETY: the kernel returned a valid move handle for the new pad session;
    // ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

/// Opens a GPIO pad session by device code (7.0.0+).
pub fn open_session2(
    session: Handle,
    device_code: u32,
    access_mode: u32,
) -> Result<Session, OpenSession2Error> {
    #[repr(C)]
    struct OpenSession2In {
        device_code: u32,
        access_mode: u32,
    }

    let input = OpenSession2In {
        device_code,
        access_mode,
    };

    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::OPEN_SESSION2)
        .data_size(size_of::<OpenSession2In>())
        .build();

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // SAFETY: req.data points to valid payload area with space for OpenSession2In.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<OpenSession2In>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(OpenSession2Error::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(OpenSession2Error::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(OpenSession2Error::MissingHandle)?;

    // SAFETY: the kernel returned a valid move handle for the new pad session;
    // ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

/// Checks if a wake event is active for the given pad name (pre-7.0.0).
pub fn is_wake_event_active(
    session: Handle,
    pad_name: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::IS_WAKE_EVENT_ACTIVE, pad_name)
}

/// Checks if a wake event is active for the given device code (7.0.0+).
pub fn is_wake_event_active2(
    session: Handle,
    device_code: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::IS_WAKE_EVENT_ACTIVE2, device_code)
}

// ---------------------------------------------------------------------------
// Pad session commands
// ---------------------------------------------------------------------------

/// Sets the pad direction.
pub fn pad_set_direction(session: Handle, direction: u32) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_DIRECTION, direction)
}

/// Gets the pad direction.
pub fn pad_get_direction(session: Handle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_DIRECTION)
}

/// Sets the interrupt mode.
pub fn pad_set_interrupt_mode(session: Handle, mode: u32) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_INTERRUPT_MODE, mode)
}

/// Gets the interrupt mode.
pub fn pad_get_interrupt_mode(session: Handle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_INTERRUPT_MODE)
}

/// Enables or disables the interrupt.
pub fn pad_set_interrupt_enable(session: Handle, enable: bool) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(session, proto::PAD_SET_INTERRUPT_ENABLE, enable)
}

/// Gets whether the interrupt is enabled.
pub fn pad_get_interrupt_enable(session: Handle) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::PAD_GET_INTERRUPT_ENABLE)
}

/// Gets the interrupt status (pre-17.0.0).
pub fn pad_get_interrupt_status(session: Handle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_INTERRUPT_STATUS)
}

/// Clears the interrupt status (pre-17.0.0).
pub fn pad_clear_interrupt_status(session: Handle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::PAD_CLEAR_INTERRUPT_STATUS)
}

/// Sets the pad output value.
pub fn pad_set_value(session: Handle, value: u32) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_VALUE, value)
}

/// Gets the pad input value.
pub fn pad_get_value(session: Handle) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_VALUE)
}

/// Binds the interrupt and returns the event handle.
pub fn pad_bind_interrupt(service: &Session) -> Result<u32, BindInterruptError> {
    let result = service
        .dispatch(proto::PAD_BIND_INTERRUPT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(BindInterruptError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(BindInterruptError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// Unbinds the interrupt.
pub fn pad_unbind_interrupt(session: Handle) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::PAD_UNBIND_INTERRUPT)
}

/// Enables or disables debounce.
pub fn pad_set_debounce_enabled(session: Handle, enable: bool) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(session, proto::PAD_SET_DEBOUNCE_ENABLED, enable)
}

/// Gets whether debounce is enabled.
pub fn pad_get_debounce_enabled(session: Handle) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::PAD_GET_DEBOUNCE_ENABLED)
}

/// Sets the debounce time in milliseconds.
pub fn pad_set_debounce_time(session: Handle, ms: i32) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_DEBOUNCE_TIME, ms as u32)
}

/// Gets the debounce time in milliseconds.
pub fn pad_get_debounce_time(session: Handle) -> Result<i32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_DEBOUNCE_TIME).map(|v| v as i32)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`open_session2`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSession2Error {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`pad_bind_interrupt`].
#[derive(Debug, thiserror::Error)]
pub enum BindInterruptError {
    #[error("failed to dispatch bind interrupt")]
    Dispatch(#[source] DispatchError),
    #[error("missing event handle in response")]
    MissingHandle,
}
