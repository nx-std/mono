//! CMIF protocol operations for the GPIO service.

use nx_sf::{
    cmif,
    ipc::Handle,
    service::{
        BorrowedSessionHandle,
        DispatchError,
        OutHandleAttr,
        OwnedSessionHandle,
        Session,
    },
};

use crate::proto;

fn dispatch_no_io(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<(), DispatchNoIoError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // This request carries no payload data.
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchNoIoError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchNoIoError::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchNoIoError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_in_u32(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: u32,
) -> Result<(), DispatchInU32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&value)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchInU32Error::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchInU32Error::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32Error {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_in_bool(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: bool,
) -> Result<(), DispatchInBoolError> {
    let raw: u8 = value as u8;

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&raw)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchInBoolError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchInBoolError::ParseResponse)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_out_u32(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<u32, DispatchOutU32Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // This request carries no payload data.
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchOutU32Error::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(DispatchOutU32Error::ParseResponse)?;

    Ok(*resp.payload)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutU32Error {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_out_bool(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<bool, DispatchOutBoolError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    // This request carries no payload data.
    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.send(&mut buf, session)
        .map_err(DispatchOutBoolError::SendRequest)?;

    let resp = cmif::parse_response::<&u8>(&buf).map_err(DispatchOutBoolError::ParseResponse)?;

    Ok(*resp.payload & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

fn dispatch_in_u32_out_bool(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&value)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchInU32OutBoolError::SendRequest)?;

    let resp =
        cmif::parse_response::<&u8>(&buf).map_err(DispatchInU32OutBoolError::ParseResponse)?;

    Ok(*resp.payload & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32OutBoolError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Opens a GPIO pad session by pad name.
pub fn open_session(
    session: BorrowedSessionHandle<'_>,
    pad_name: u32,
) -> Result<Session, OpenSessionError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::OPEN_SESSION)
        .with_data_value(&pad_name)
        .build();
    req.send(&mut buf, session)
        .map_err(OpenSessionError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenSessionError::ParseResponse)?;

    let Some(&raw_handle) = resp.move_handles.first() else {
        return Err(OpenSessionError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid move handle for the new pad session;
    // ownership transfers to the new `Session`.
    let handle = Handle::from_raw_unchecked(raw_handle);

    Ok(Session::new(
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // Session below becomes its sole owner.
        OwnedSessionHandle::from_handle_unchecked(handle),
        0,
    ))
}

/// Opens a GPIO pad session by device code (7.0.0+).
pub fn open_session2(
    session: BorrowedSessionHandle<'_>,
    device_code: u32,
    access_mode: u32,
) -> Result<Session, OpenSession2Error> {
    #[repr(C)]
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    struct OpenSession2In {
        device_code: u32,
        access_mode: u32,
    }

    let input = OpenSession2In {
        device_code,
        access_mode,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::OPEN_SESSION2)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(OpenSession2Error::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(OpenSession2Error::ParseResponse)?;

    let Some(&raw_handle) = resp.move_handles.first() else {
        return Err(OpenSession2Error::MissingHandle);
    };

    // SAFETY: the kernel returned a valid move handle for the new pad session;
    // ownership transfers to the new `Session`.
    let handle = Handle::from_raw_unchecked(raw_handle);

    Ok(Session::new(
        // SAFETY: The server returned a freshly opened session in this reply, so the
        // Session below becomes its sole owner.
        OwnedSessionHandle::from_handle_unchecked(handle),
        0,
    ))
}

/// Checks if a wake event is active for the given pad name (pre-7.0.0).
pub fn is_wake_event_active(
    session: BorrowedSessionHandle<'_>,
    pad_name: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::IS_WAKE_EVENT_ACTIVE, pad_name)
}

/// Checks if a wake event is active for the given device code (7.0.0+).
pub fn is_wake_event_active2(
    session: BorrowedSessionHandle<'_>,
    device_code: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::IS_WAKE_EVENT_ACTIVE2, device_code)
}

/// Sets the pad direction.
pub fn pad_set_direction(
    session: BorrowedSessionHandle<'_>,
    direction: u32,
) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_DIRECTION, direction)
}

/// Gets the pad direction.
pub fn pad_get_direction(session: BorrowedSessionHandle<'_>) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_DIRECTION)
}

/// Sets the interrupt mode.
pub fn pad_set_interrupt_mode(
    session: BorrowedSessionHandle<'_>,
    mode: u32,
) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_INTERRUPT_MODE, mode)
}

/// Gets the interrupt mode.
pub fn pad_get_interrupt_mode(
    session: BorrowedSessionHandle<'_>,
) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_INTERRUPT_MODE)
}

/// Enables or disables the interrupt.
pub fn pad_set_interrupt_enable(
    session: BorrowedSessionHandle<'_>,
    enable: bool,
) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(session, proto::PAD_SET_INTERRUPT_ENABLE, enable)
}

/// Gets whether the interrupt is enabled.
pub fn pad_get_interrupt_enable(
    session: BorrowedSessionHandle<'_>,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::PAD_GET_INTERRUPT_ENABLE)
}

/// Gets the interrupt status (pre-17.0.0).
pub fn pad_get_interrupt_status(
    session: BorrowedSessionHandle<'_>,
) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_INTERRUPT_STATUS)
}

/// Clears the interrupt status (pre-17.0.0).
pub fn pad_clear_interrupt_status(
    session: BorrowedSessionHandle<'_>,
) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::PAD_CLEAR_INTERRUPT_STATUS)
}

/// Sets the pad output value.
pub fn pad_set_value(
    session: BorrowedSessionHandle<'_>,
    value: u32,
) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_VALUE, value)
}

/// Gets the pad input value.
pub fn pad_get_value(session: BorrowedSessionHandle<'_>) -> Result<u32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_VALUE)
}

/// Binds the interrupt and returns the event handle.
pub fn pad_bind_interrupt(service: &Session) -> Result<u32, BindInterruptError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PAD_BIND_INTERRUPT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(BindInterruptError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(BindInterruptError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// Unbinds the interrupt.
pub fn pad_unbind_interrupt(session: BorrowedSessionHandle<'_>) -> Result<(), DispatchNoIoError> {
    dispatch_no_io(session, proto::PAD_UNBIND_INTERRUPT)
}

/// Enables or disables debounce.
pub fn pad_set_debounce_enabled(
    session: BorrowedSessionHandle<'_>,
    enable: bool,
) -> Result<(), DispatchInBoolError> {
    dispatch_in_bool(session, proto::PAD_SET_DEBOUNCE_ENABLED, enable)
}

/// Gets whether debounce is enabled.
pub fn pad_get_debounce_enabled(
    session: BorrowedSessionHandle<'_>,
) -> Result<bool, DispatchOutBoolError> {
    dispatch_out_bool(session, proto::PAD_GET_DEBOUNCE_ENABLED)
}

/// Sets the debounce time in milliseconds.
pub fn pad_set_debounce_time(
    session: BorrowedSessionHandle<'_>,
    ms: i32,
) -> Result<(), DispatchInU32Error> {
    dispatch_in_u32(session, proto::PAD_SET_DEBOUNCE_TIME, ms as u32)
}

/// Gets the debounce time in milliseconds.
pub fn pad_get_debounce_time(
    session: BorrowedSessionHandle<'_>,
) -> Result<i32, DispatchOutU32Error> {
    dispatch_out_u32(session, proto::PAD_GET_DEBOUNCE_TIME).map(|v| v as i32)
}

/// Error returned by [`open_session`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSessionError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by [`open_session2`].
#[derive(Debug, thiserror::Error)]
pub enum OpenSession2Error {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
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
