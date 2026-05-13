//! CMIF protocol operations for the HID Bus service.

use core::ptr;

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    proto,
    types::{
        BusHandle, BusHandleResIdIn, EnableExternalDeviceIn, EnableJoyPollingIn, GetBusHandleIn,
        GetBusHandleOut, JoyPollingMode,
    },
};

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_in<T>(session: SessionHandle, cmd_id: u32, value: &T) -> Result<(), DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<T>())
        .build();

    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<T>().cast_mut(), ptr::read(value));
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_in_out<T, U>(
    session: SessionHandle,
    cmd_id: u32,
    value: &T,
) -> Result<U, DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<T>())
        .build();

    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<T>().cast_mut(), ptr::read(value));
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<U>()) }
        .map_err(DispatchError::ParseResponse)?;

    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<U>()) };

    Ok(out)
}

/// Error returned by hidbus dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// GetBusHandle (cmd 1).
pub fn get_bus_handle(
    session: SessionHandle,
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
    session: SessionHandle,
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
    session: SessionHandle,
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
    session: SessionHandle,
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
    session: SessionHandle,
    handle: BusHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(session, proto::GET_EXTERNAL_DEVICE_ID, &handle)
}

/// SendCommandAsync (cmd 7).
pub fn send_command_async(
    session: SessionHandle,
    handle: BusHandle,
    buffer: &[u8],
) -> Result<(), DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SEND_COMMAND_ASYNC)
        .data_size(size_of::<BusHandle>())
        .in_auto_buffers(1)
        .build();

    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<BusHandle>().cast_mut(), handle);
    }

    req.add_in_auto_buffer(buffer.as_ptr(), buffer.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(DispatchError::ParseResponse)?;

    Ok(())
}

/// GetSendCommandAsyncResult (cmd 8).
pub fn get_send_command_async_result(
    session: SessionHandle,
    handle: BusHandle,
    buffer: &mut [u8],
) -> Result<u32, GetSendCommandAsyncResultError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SEND_COMMAND_ASYNC_RESULT)
        .data_size(size_of::<BusHandle>())
        .out_auto_buffers(1)
        .build();

    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<BusHandle>().cast_mut(), handle);
    }

    req.add_out_auto_buffer(buffer.as_mut_ptr(), buffer.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(GetSendCommandAsyncResultError::SendRequest)?;

    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u32>()) }
        .map_err(GetSendCommandAsyncResultError::ParseResponse)?;

    let out_size = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(out_size)
}

/// Error returned by [`get_send_command_async_result`].
#[derive(Debug, thiserror::Error)]
pub enum GetSendCommandAsyncResultError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// SetEventForSendCommandAsyncResult (cmd 9).
pub fn set_event_for_send_command_async_result(
    session: SessionHandle,
    handle: BusHandle,
) -> Result<u32, SetEventError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::SET_EVENT_FOR_SEND_COMMAND_ASYNC_RESULT)
        .data_size(size_of::<BusHandle>())
        .build();

    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<BusHandle>().cast_mut(), handle);
    }

    ipc::send_sync_request(session).map_err(SetEventError::SendRequest)?;

    let resp =
        unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(SetEventError::ParseResponse)?;

    resp.copy_handles
        .first()
        .copied()
        .ok_or(SetEventError::MissingHandle)
}

/// Error returned by [`set_event_for_send_command_async_result`].
#[derive(Debug, thiserror::Error)]
pub enum SetEventError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing event handle in response")]
    MissingHandle,
}

/// GetSharedMemoryHandle (cmd 10).
pub fn get_shared_memory_handle(session: SessionHandle) -> Result<u32, GetSharedMemoryError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(proto::GET_SHARED_MEMORY_HANDLE).build();

    unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSharedMemoryError::SendRequest)?;

    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetSharedMemoryError::ParseResponse)?;

    resp.copy_handles
        .first()
        .copied()
        .ok_or(GetSharedMemoryError::MissingHandle)
}

/// Error returned by [`get_shared_memory_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

/// EnableJoyPollingReceiveMode (cmd 11).
pub fn enable_joy_polling_receive_mode(
    session: SessionHandle,
    handle: BusHandle,
    polling_mode: JoyPollingMode,
    command_buffer: &[u8],
    tmem_handle: u32,
    tmem_size: u32,
) -> Result<(), EnableJoyPollingError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let input = EnableJoyPollingIn {
        tmem_size,
        polling_mode: polling_mode as u32,
        handle,
    };

    let fmt = cmif::RequestFormatBuilder::new(proto::ENABLE_JOY_POLLING_RECEIVE_MODE)
        .data_size(size_of::<EnableJoyPollingIn>())
        .in_auto_buffers(1)
        .handles(1)
        .build();

    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<EnableJoyPollingIn>().cast_mut(),
            input,
        );
    }

    req.add_in_auto_buffer(
        command_buffer.as_ptr(),
        command_buffer.len(),
        BufferMode::Normal,
    );

    req.add_handle(tmem_handle);

    ipc::send_sync_request(session).map_err(EnableJoyPollingError::SendRequest)?;

    unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(EnableJoyPollingError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`enable_joy_polling_receive_mode`].
#[derive(Debug, thiserror::Error)]
pub enum EnableJoyPollingError {
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// DisableJoyPollingReceiveMode (cmd 12).
pub fn disable_joy_polling_receive_mode(
    session: SessionHandle,
    handle: BusHandle,
) -> Result<(), DispatchError> {
    dispatch_in(session, proto::DISABLE_JOY_POLLING_RECEIVE_MODE, &handle)
}

/// SetStatusManagerType (cmd 14).
pub fn set_status_manager_type(
    session: SessionHandle,
    manager_type: u32,
) -> Result<(), DispatchError> {
    dispatch_in(session, proto::SET_STATUS_MANAGER_TYPE, &manager_type)
}
