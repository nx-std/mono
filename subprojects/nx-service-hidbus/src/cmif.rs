//! CMIF protocol operations for the HID Bus service.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        InputBuffer,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};

use crate::{
    proto,
    types::{
        BusHandle,
        BusHandleResIdIn,
        EnableExternalDeviceIn,
        EnableJoyPollingIn,
        GetBusHandleIn,
        GetBusHandleOut,
        JoyPollingMode,
    },
};

fn dispatch_in<T>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: &T,
) -> Result<(), DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(value)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_in_out<T, U>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: &T,
) -> Result<U, DispatchError>
where
    T: zerocopy::IntoBytes + zerocopy::Immutable,
    U: Copy + zerocopy::FromBytes + zerocopy::Immutable + zerocopy::KnownLayout,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(value)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp = cmif::parse_response::<&U>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by hidbus dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// GetBusHandle (cmd 1).
pub fn get_bus_handle(
    session: BorrowedSessionHandle<'_>,
    npad_id: u32,
    bus_type: u64,
    applet_resource_user_id: u64,
) -> Result<(bool, BusHandle), DispatchError> {
    let input = GetBusHandleIn {
        npad_id,
        pad: 0,
        bus_type,
        applet_resource_user_id,
    };

    let out: GetBusHandleOut = dispatch_in_out(session, proto::GET_BUS_HANDLE, &input)?;

    Ok((out.flag & 1 != 0, out.handle))
}

/// Initialize (cmd 3).
pub fn initialize(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = BusHandleResIdIn {
        handle,
        applet_resource_user_id,
    };

    dispatch_in(session, proto::INITIALIZE, &input)
}

/// Finalize (cmd 4).
pub fn finalize(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = BusHandleResIdIn {
        handle,
        applet_resource_user_id,
    };

    dispatch_in(session, proto::FINALIZE, &input)
}

/// EnableExternalDevice (cmd 5).
pub fn enable_external_device(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
    flag: bool,
    inval: u64,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = EnableExternalDeviceIn {
        flag: u8::from(flag),
        pad: [0; 7],
        handle,
        inval,
        applet_resource_user_id,
    };

    dispatch_in(session, proto::ENABLE_EXTERNAL_DEVICE, &input)
}

/// GetExternalDeviceId (cmd 6).
pub fn get_external_device_id(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(session, proto::GET_EXTERNAL_DEVICE_ID, &handle)
}

/// SendCommandAsync (cmd 7).
pub fn send_command_async(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
    buffer: &[u8],
) -> Result<(), DispatchError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::SEND_COMMAND_ASYNC)
        .with_data_value(&handle)
        .add_in_auto_buffer(InputBuffer::new(buffer, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

/// GetSendCommandAsyncResult (cmd 8).
pub fn get_send_command_async_result(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
    buffer: &mut [u8],
) -> Result<u32, GetSendCommandAsyncResultError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_SEND_COMMAND_ASYNC_RESULT)
        .with_data_value(&handle)
        .add_out_auto_buffer(OutputBuffer::new(buffer, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetSendCommandAsyncResultError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf)
        .map_err(GetSendCommandAsyncResultError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Error returned by [`get_send_command_async_result`].
#[derive(Debug, thiserror::Error)]
pub enum GetSendCommandAsyncResultError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// SetEventForSendCommandAsyncResult (cmd 9).
pub fn set_event_for_send_command_async_result(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
) -> Result<u32, SetEventError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::SET_EVENT_FOR_SEND_COMMAND_ASYNC_RESULT)
        .with_data_value(&handle)
        .build();
    req.send(&mut buf, session)
        .map_err(SetEventError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(SetEventError::ParseResponse)?;

    resp.copy_handles
        .first()
        .copied()
        .ok_or(SetEventError::MissingHandle)
}

/// Error returned by [`set_event_for_send_command_async_result`].
#[derive(Debug, thiserror::Error)]
pub enum SetEventError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    #[error("missing event handle in response")]
    MissingHandle,
}

/// GetSharedMemoryHandle (cmd 10).
pub fn get_shared_memory_handle(
    session: BorrowedSessionHandle<'_>,
) -> Result<u32, GetSharedMemoryError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::GET_SHARED_MEMORY_HANDLE).build();
    req.send(&mut buf, session)
        .map_err(GetSharedMemoryError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetSharedMemoryError::ParseResponse)?;

    resp.copy_handles
        .first()
        .copied()
        .ok_or(GetSharedMemoryError::MissingHandle)
}

/// Error returned by [`get_shared_memory_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

/// EnableJoyPollingReceiveMode (cmd 11).
pub fn enable_joy_polling_receive_mode(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
    polling_mode: JoyPollingMode,
    command_buffer: &[u8],
    tmem_handle: u32,
    tmem_size: u32,
) -> Result<(), EnableJoyPollingError> {
    let input = EnableJoyPollingIn {
        tmem_size,
        polling_mode: polling_mode as u32,
        handle,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(proto::ENABLE_JOY_POLLING_RECEIVE_MODE)
        .with_data_value(&input)
        .add_in_auto_buffer(InputBuffer::new(command_buffer, BufferMode::Normal))
        .add_copy_handle(tmem_handle)
        .build();
    req.send(&mut buf, session)
        .map_err(EnableJoyPollingError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(EnableJoyPollingError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`enable_joy_polling_receive_mode`].
#[derive(Debug, thiserror::Error)]
pub enum EnableJoyPollingError {
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// DisableJoyPollingReceiveMode (cmd 12).
pub fn disable_joy_polling_receive_mode(
    session: BorrowedSessionHandle<'_>,
    handle: BusHandle,
) -> Result<(), DispatchError> {
    dispatch_in(session, proto::DISABLE_JOY_POLLING_RECEIVE_MODE, &handle)
}

/// SetStatusManagerType (cmd 14).
pub fn set_status_manager_type(
    session: BorrowedSessionHandle<'_>,
    manager_type: u32,
) -> Result<(), DispatchError> {
    dispatch_in(session, proto::SET_STATUS_MANAGER_TYPE, &manager_type)
}
