//! CMIF protocol operations for the UART service.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif,
    ipc::{self, Handle},
    service::{BufferAttr, DispatchError, OutHandleAttr, Session},
};

use crate::{
    proto,
    types::{BindPortEventIn, OpenPortLegacyIn, OpenPortV6In, OpenPortV7In},
};

fn dispatch_in_u32_out_bool(
    session: Handle,
    cmd_id: u32,
    value: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&value)
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInU32OutBoolError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DispatchInU32OutBoolError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(DispatchInU32OutBoolError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInU32OutBoolError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_in_two_u32s_out_bool(
    session: Handle,
    cmd_id: u32,
    val0: u32,
    val1: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    #[repr(C)]
    #[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
    struct TwoU32s {
        val0: u32,
        val1: u32,
    }

    let input = TwoU32s { val0, val1 };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(DispatchInTwoU32sOutBoolError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session)
        .map_err(DispatchInTwoU32sOutBoolError::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u8>())
        .map_err(DispatchInTwoU32sOutBoolError::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u8.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(raw & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchInTwoU32sOutBoolError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

fn dispatch_out_u64(session: Handle, cmd_id: u32) -> Result<u64, DispatchOutU64Error> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(cmd_id).build();
    req.write_to(&mut buf)
        .map_err(DispatchOutU64Error::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(DispatchOutU64Error::SendRequest)?;

    let resp = cmif::parse_response_bytes(&buf, size_of::<u64>())
        .map_err(DispatchOutU64Error::ParseResponse)?;

    // SAFETY: resp.data points to valid payload area with space for u64.
    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(value)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchOutU64Error {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Checks if a production port exists (pre-17.0.0).
pub fn has_port(session: Handle, port: u32) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::HAS_PORT, port)
}

/// Checks if a dev port exists (pre-17.0.0).
pub fn has_port_for_dev(session: Handle, port: u32) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::HAS_PORT_FOR_DEV, port)
}

/// Checks if a baud rate is supported for a production port (pre-17.0.0).
pub fn is_supported_baud_rate(
    session: Handle,
    port: u32,
    baud_rate: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(session, proto::IS_SUPPORTED_BAUD_RATE, port, baud_rate)
}

/// Checks if a baud rate is supported for a dev port (pre-17.0.0).
pub fn is_supported_baud_rate_for_dev(
    session: Handle,
    port: u32,
    baud_rate: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(
        session,
        proto::IS_SUPPORTED_BAUD_RATE_FOR_DEV,
        port,
        baud_rate,
    )
}

/// Checks if a flow control mode is supported for a production port (pre-17.0.0).
pub fn is_supported_flow_control_mode(
    session: Handle,
    port: u32,
    mode: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(session, proto::IS_SUPPORTED_FLOW_CONTROL_MODE, port, mode)
}

/// Checks if a flow control mode is supported for a dev port (pre-17.0.0).
pub fn is_supported_flow_control_mode_for_dev(
    session: Handle,
    port: u32,
    mode: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(
        session,
        proto::IS_SUPPORTED_FLOW_CONTROL_MODE_FOR_DEV,
        port,
        mode,
    )
}

/// Creates a new port session (returns IPortSession as a move handle).
pub fn create_port_session(session: Handle) -> Result<Session, CreatePortSessionError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(proto::CREATE_PORT_SESSION).build();
    req.write_to(&mut buf)
        .map_err(CreatePortSessionError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(CreatePortSessionError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, 0).map_err(CreatePortSessionError::ParseResponse)?;

    let Some(&raw_handle) = resp.move_handles.first() else {
        return Err(CreatePortSessionError::MissingHandle);
    };

    // SAFETY: the kernel returned a valid move handle for the new port
    // session; ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

/// Checks if a port event type is supported for a production port (pre-17.0.0).
pub fn is_supported_port_event(
    session: Handle,
    port: u32,
    event_type: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(session, proto::IS_SUPPORTED_PORT_EVENT, port, event_type)
}

/// Checks if a port event type is supported for a dev port (pre-17.0.0).
pub fn is_supported_port_event_for_dev(
    session: Handle,
    port: u32,
    event_type: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(
        session,
        proto::IS_SUPPORTED_PORT_EVENT_FOR_DEV,
        port,
        event_type,
    )
}

/// Checks if a device variation is supported for a production port ([7.0.0-16.1.0]).
pub fn is_supported_device_variation(
    session: Handle,
    port: u32,
    device_variation: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(
        session,
        proto::IS_SUPPORTED_DEVICE_VARIATION,
        port,
        device_variation,
    )
}

/// Checks if a device variation is supported for a dev port ([7.0.0-16.1.0]).
pub fn is_supported_device_variation_for_dev(
    session: Handle,
    port: u32,
    device_variation: u32,
) -> Result<bool, DispatchInTwoU32sOutBoolError> {
    dispatch_in_two_u32s_out_bool(
        session,
        proto::IS_SUPPORTED_DEVICE_VARIATION_FOR_DEV,
        port,
        device_variation,
    )
}

/// Opens a port using the pre-6.0.0 wire format (legacy).
#[allow(clippy::too_many_arguments)]
pub fn port_open_legacy(
    service: &Session,
    port: u32,
    baud_rate: u32,
    flow_control_mode: u32,
    send_tmem_handle: u32,
    receive_tmem_handle: u32,
    send_buffer_length: u64,
    receive_buffer_length: u64,
) -> Result<bool, OpenPortError> {
    let input = OpenPortLegacyIn {
        port,
        baud_rate,
        flow_control_mode,
        pad: 0,
        send_buffer_length,
        receive_buffer_length,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<OpenPortLegacyIn>()` bytes as a slice
    // is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<OpenPortLegacyIn>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_OPEN)
        .in_raw(in_bytes)
        .in_handle(send_tmem_handle)
        .in_handle(receive_tmem_handle)
        .out_size(size_of::<u8>())
        .send(&mut buf)
        .map_err(OpenPortError::Dispatch)?;

    let raw = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(raw & 1 != 0)
}

/// Opens a port using the 6.x wire format (adds signal inversion flags).
#[allow(clippy::too_many_arguments)]
pub fn port_open_v6(
    service: &Session,
    port: u32,
    baud_rate: u32,
    flow_control_mode: u32,
    is_invert_tx: bool,
    is_invert_rx: bool,
    is_invert_rts: bool,
    is_invert_cts: bool,
    send_tmem_handle: u32,
    receive_tmem_handle: u32,
    send_buffer_length: u64,
    receive_buffer_length: u64,
) -> Result<bool, OpenPortError> {
    let input = OpenPortV6In {
        is_invert_tx: is_invert_tx as u8,
        is_invert_rx: is_invert_rx as u8,
        is_invert_rts: is_invert_rts as u8,
        is_invert_cts: is_invert_cts as u8,
        port,
        baud_rate,
        flow_control_mode,
        send_buffer_length,
        receive_buffer_length,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<OpenPortV6In>()` bytes as a slice is
    // sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<OpenPortV6In>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_OPEN)
        .in_raw(in_bytes)
        .in_handle(send_tmem_handle)
        .in_handle(receive_tmem_handle)
        .out_size(size_of::<u8>())
        .send(&mut buf)
        .map_err(OpenPortError::Dispatch)?;

    let raw = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(raw & 1 != 0)
}

/// Opens a port using the 7.0.0+ wire format (adds device variation).
#[allow(clippy::too_many_arguments)]
pub fn port_open_v7(
    service: &Session,
    port: u32,
    baud_rate: u32,
    flow_control_mode: u32,
    device_variation: u32,
    is_invert_tx: bool,
    is_invert_rx: bool,
    is_invert_rts: bool,
    is_invert_cts: bool,
    send_tmem_handle: u32,
    receive_tmem_handle: u32,
    send_buffer_length: u64,
    receive_buffer_length: u64,
) -> Result<bool, OpenPortError> {
    let input = OpenPortV7In {
        is_invert_tx: is_invert_tx as u8,
        is_invert_rx: is_invert_rx as u8,
        is_invert_rts: is_invert_rts as u8,
        is_invert_cts: is_invert_cts as u8,
        port,
        baud_rate,
        flow_control_mode,
        device_variation,
        pad: 0,
        send_buffer_length,
        receive_buffer_length,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<OpenPortV7In>()` bytes as a slice is
    // sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<OpenPortV7In>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_OPEN)
        .in_raw(in_bytes)
        .in_handle(send_tmem_handle)
        .in_handle(receive_tmem_handle)
        .out_size(size_of::<u8>())
        .send(&mut buf)
        .map_err(OpenPortError::Dispatch)?;

    let raw = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(raw & 1 != 0)
}

/// Opens a dev port using the pre-6.0.0 wire format (legacy).
#[allow(clippy::too_many_arguments)]
pub fn port_open_for_dev_legacy(
    service: &Session,
    port: u32,
    baud_rate: u32,
    flow_control_mode: u32,
    send_tmem_handle: u32,
    receive_tmem_handle: u32,
    send_buffer_length: u64,
    receive_buffer_length: u64,
) -> Result<bool, OpenPortError> {
    let input = OpenPortLegacyIn {
        port,
        baud_rate,
        flow_control_mode,
        pad: 0,
        send_buffer_length,
        receive_buffer_length,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<OpenPortLegacyIn>()` bytes as a slice
    // is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<OpenPortLegacyIn>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_OPEN_FOR_DEV)
        .in_raw(in_bytes)
        .in_handle(send_tmem_handle)
        .in_handle(receive_tmem_handle)
        .out_size(size_of::<u8>())
        .send(&mut buf)
        .map_err(OpenPortError::Dispatch)?;

    let raw = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(raw & 1 != 0)
}

/// Opens a dev port using the 6.x wire format (adds signal inversion flags).
#[allow(clippy::too_many_arguments)]
pub fn port_open_for_dev_v6(
    service: &Session,
    port: u32,
    baud_rate: u32,
    flow_control_mode: u32,
    is_invert_tx: bool,
    is_invert_rx: bool,
    is_invert_rts: bool,
    is_invert_cts: bool,
    send_tmem_handle: u32,
    receive_tmem_handle: u32,
    send_buffer_length: u64,
    receive_buffer_length: u64,
) -> Result<bool, OpenPortError> {
    let input = OpenPortV6In {
        is_invert_tx: is_invert_tx as u8,
        is_invert_rx: is_invert_rx as u8,
        is_invert_rts: is_invert_rts as u8,
        is_invert_cts: is_invert_cts as u8,
        port,
        baud_rate,
        flow_control_mode,
        send_buffer_length,
        receive_buffer_length,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<OpenPortV6In>()` bytes as a slice is
    // sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<OpenPortV6In>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_OPEN_FOR_DEV)
        .in_raw(in_bytes)
        .in_handle(send_tmem_handle)
        .in_handle(receive_tmem_handle)
        .out_size(size_of::<u8>())
        .send(&mut buf)
        .map_err(OpenPortError::Dispatch)?;

    let raw = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(raw & 1 != 0)
}

/// Opens a dev port using the 7.0.0+ wire format (adds device variation).
#[allow(clippy::too_many_arguments)]
pub fn port_open_for_dev_v7(
    service: &Session,
    port: u32,
    baud_rate: u32,
    flow_control_mode: u32,
    device_variation: u32,
    is_invert_tx: bool,
    is_invert_rx: bool,
    is_invert_rts: bool,
    is_invert_cts: bool,
    send_tmem_handle: u32,
    receive_tmem_handle: u32,
    send_buffer_length: u64,
    receive_buffer_length: u64,
) -> Result<bool, OpenPortError> {
    let input = OpenPortV7In {
        is_invert_tx: is_invert_tx as u8,
        is_invert_rx: is_invert_rx as u8,
        is_invert_rts: is_invert_rts as u8,
        is_invert_cts: is_invert_cts as u8,
        port,
        baud_rate,
        flow_control_mode,
        device_variation,
        pad: 0,
        send_buffer_length,
        receive_buffer_length,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<OpenPortV7In>()` bytes as a slice is
    // sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<OpenPortV7In>())
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_OPEN_FOR_DEV)
        .in_raw(in_bytes)
        .in_handle(send_tmem_handle)
        .in_handle(receive_tmem_handle)
        .out_size(size_of::<u8>())
        .send(&mut buf)
        .map_err(OpenPortError::Dispatch)?;

    let raw = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    Ok(raw & 1 != 0)
}

/// Gets the number of bytes available for writing.
pub fn port_get_writable_length(session: Handle) -> Result<u64, DispatchOutU64Error> {
    dispatch_out_u64(session, proto::PORT_GET_WRITABLE_LENGTH)
}

/// Sends data through the port (HipcAutoSelect in-buffer).
pub fn port_send(service: &Session, data: &[u8]) -> Result<u64, PortSendError> {
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_SEND)
        .in_buffer(data, BufferAttr::HIPC_AUTO_SELECT)
        .out_size(size_of::<u64>())
        .send(&mut buf)
        .map_err(PortSendError::Dispatch)?;

    let bytes_written = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };
    Ok(bytes_written)
}

/// Gets the number of bytes available for reading.
pub fn port_get_readable_length(session: Handle) -> Result<u64, DispatchOutU64Error> {
    dispatch_out_u64(session, proto::PORT_GET_READABLE_LENGTH)
}

/// Receives data from the port (HipcAutoSelect out-buffer).
pub fn port_receive(service: &Session, buf: &mut [u8]) -> Result<u64, PortReceiveError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_RECEIVE)
        .out_buffer(buf, BufferAttr::HIPC_AUTO_SELECT)
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)
        .map_err(PortReceiveError::Dispatch)?;

    let bytes_read = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };
    Ok(bytes_read)
}

/// Binds a port event and returns (success, event_handle).
pub fn port_bind_port_event(
    service: &Session,
    port_event_type: u32,
    threshold: i64,
) -> Result<(bool, u32), BindPortEventError> {
    let input = BindPortEventIn {
        port_event_type,
        pad: 0,
        threshold,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<BindPortEventIn>()` bytes as a slice
    // is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<BindPortEventIn>(),
        )
    };
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::PORT_BIND_PORT_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)
        .map_err(BindPortEventError::Dispatch)?;

    let raw = unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<u8>()) };
    let success = raw & 1 != 0;

    let Some(&event_handle) = result.copy_handles.first() else {
        return Err(BindPortEventError::MissingHandle);
    };

    Ok((success, event_handle))
}

/// Unbinds a port event.
pub fn port_unbind_port_event(
    session: Handle,
    port_event_type: u32,
) -> Result<bool, DispatchInU32OutBoolError> {
    dispatch_in_u32_out_bool(session, proto::PORT_UNBIND_PORT_EVENT, port_event_type)
}

/// Error returned by [`create_port_session`].
#[derive(Debug, thiserror::Error)]
pub enum CreatePortSessionError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by the open port commands.
#[derive(Debug, thiserror::Error)]
pub enum OpenPortError {
    #[error("failed to dispatch open port")]
    Dispatch(#[source] DispatchError),
}

/// Error returned by [`port_send`].
#[derive(Debug, thiserror::Error)]
pub enum PortSendError {
    #[error("failed to dispatch send")]
    Dispatch(#[source] DispatchError),
}

/// Error returned by [`port_receive`].
#[derive(Debug, thiserror::Error)]
pub enum PortReceiveError {
    #[error("failed to dispatch receive")]
    Dispatch(#[source] DispatchError),
}

/// Error returned by [`port_bind_port_event`].
#[derive(Debug, thiserror::Error)]
pub enum BindPortEventError {
    #[error("failed to dispatch bind port event")]
    Dispatch(#[source] DispatchError),
    #[error("missing event handle in response")]
    MissingHandle,
}
