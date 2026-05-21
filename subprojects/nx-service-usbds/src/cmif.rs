//! CMIF protocol operations for the USB device stack service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{
        dispatch_domain_in_no_out, dispatch_domain_in_out, dispatch_domain_no_io,
        dispatch_domain_out,
    },
    proto,
    types::{AppendConfigDataLegacyIn, PostBufferIn, UsbDsReportData, UsbStringDescriptor},
};

// ---------------------------------------------------------------------------
// IDsService — root service commands
// ---------------------------------------------------------------------------

/// BindDevice (pre-11.0.0, cmd 0). Takes complex_id as u32.
pub(crate) fn bind_device_legacy(service: &Session, complex_id: u32) -> Result<(), DispatchError> {
    dispatch_domain_in_no_out(service, proto::BIND_DEVICE_LEGACY, &complex_id)
}

/// SetProcessHandle (pre-11.0.0, cmd 1). Sends process handle as copy-handle.
pub(crate) fn set_process_handle_legacy(
    service: &Session,
    proc_handle: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::SET_PROCESS_HANDLE_LEGACY)
        .in_handle(proc_handle)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// BindDevice (11.0.0+, cmd 0). Takes complex_id and process handle inline.
pub(crate) fn bind_device(
    service: &Session,
    complex_id: u32,
    proc_handle: u32,
) -> Result<(), DispatchError> {
    // SAFETY: `complex_id` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const complex_id).cast::<u8>(), size_of::<u32>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::BIND_DEVICE)
        .in_raw(in_bytes)
        .in_handle(proc_handle)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetStateChangeEvent. Returns the copy-handle for the event.
pub(crate) fn get_state_change_event(service: &Session, cmd_id: u32) -> Result<u32, GetEventError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .send(&mut ipc_buf)
        .map_err(GetEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// GetState. Returns state as u32.
pub(crate) fn get_state(service: &Session, cmd_id: u32) -> Result<u32, DispatchError> {
    dispatch_domain_out::<u32>(service, cmd_id)
}

/// GetDsInterface (pre-5.0.0, cmd 2). Two input buffers, returns domain object + u8.
pub(crate) fn get_ds_interface_legacy(
    service: &Session,
    descriptor: &[u8],
    interface_name: &[u8],
) -> Result<(u32, u8), GetInterfaceError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::GET_DS_INTERFACE_LEGACY)
        .in_buffer(descriptor, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(interface_name, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)
        .map_err(GetInterfaceError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(GetInterfaceError::MissingHandle);
    }

    let intf_num = result.data[0];
    Ok((result.move_handles[0], intf_num))
}

/// RegisterInterface (5.0.0+). Takes interface number, returns domain object.
pub(crate) fn register_interface(
    service: &Session,
    cmd_id: u32,
    intf_num: u8,
) -> Result<u32, RegisterInterfaceError> {
    // SAFETY: `intf_num` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const intf_num).cast::<u8>(), size_of::<u8>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map_err(RegisterInterfaceError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(RegisterInterfaceError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// SetVidPidBcd (pre-5.0.0, cmd 5). Input buffer.
pub(crate) fn set_vid_pid_bcd(service: &Session, deviceinfo: &[u8]) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::SET_VID_PID_BCD)
        .in_buffer(deviceinfo, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// ClearDeviceData (5.0.0+).
pub(crate) fn clear_device_data(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, cmd_id)
}

/// AddUsbStringDescriptor. Returns index.
pub(crate) fn add_usb_string_descriptor(
    service: &Session,
    cmd_id: u32,
    descriptor: &UsbStringDescriptor,
) -> Result<u8, DispatchError> {
    // SAFETY: `UsbStringDescriptor` is a `#[repr(C)]` struct; viewing its
    // `size_of` bytes as a byte slice for the IN buffer is sound, and the
    // slice borrows `descriptor`.
    let desc_bytes = unsafe {
        core::slice::from_raw_parts(
            (descriptor as *const UsbStringDescriptor).cast::<u8>(),
            size_of::<UsbStringDescriptor>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .in_buffer(desc_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)?;

    Ok(result.data[0])
}

/// DeleteUsbStringDescriptor.
pub(crate) fn delete_usb_string_descriptor(
    service: &Session,
    cmd_id: u32,
    index: u8,
) -> Result<(), DispatchError> {
    dispatch_domain_in_no_out(service, cmd_id, &index)
}

/// SetUsbDeviceDescriptor. Takes speed + descriptor buffer.
pub(crate) fn set_usb_device_descriptor(
    service: &Session,
    cmd_id: u32,
    speed: u32,
    descriptor: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `speed` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const speed).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(descriptor, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetBinaryObjectStore. Input buffer.
pub(crate) fn set_binary_object_store(
    service: &Session,
    cmd_id: u32,
    bos: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(cmd_id)
        .in_buffer(bos, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Enable.
pub(crate) fn enable(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, cmd_id)
}

/// Disable.
pub(crate) fn disable(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, cmd_id)
}

/// GetSpeed (8.0.0+). Returns speed as u32.
pub(crate) fn get_speed(service: &Session, cmd_id: u32) -> Result<u32, DispatchError> {
    dispatch_domain_out::<u32>(service, cmd_id)
}

// ---------------------------------------------------------------------------
// IDsInterface commands
// ---------------------------------------------------------------------------

/// GetSetupEvent. Returns copy-handle.
pub(crate) fn intf_get_event(service: &Session, cmd_id: u32) -> Result<u32, GetEventError> {
    get_state_change_event(service, cmd_id)
}

/// GetSetupPacket. Output buffer.
pub(crate) fn intf_get_setup_packet(
    service: &Session,
    buffer: &mut [u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::INTF_GET_SETUP_PACKET)
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// EnableInterface (pre-11.0.0).
pub(crate) fn intf_enable_interface(service: &Session) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, proto::INTF_ENABLE_INTERFACE_LEGACY)
}

/// DisableInterface (pre-11.0.0).
pub(crate) fn intf_disable_interface(service: &Session) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, proto::INTF_DISABLE_INTERFACE_LEGACY)
}

/// PostBufferAsync (ctrl in/out or endpoint).
pub(crate) fn post_buffer_async(
    service: &Session,
    cmd_id: u32,
    buffer_addr: u64,
    size: u32,
) -> Result<u32, DispatchError> {
    let input = PostBufferIn {
        size,
        _pad: 0,
        buffer: buffer_addr,
    };
    dispatch_domain_in_out::<PostBufferIn, u32>(service, cmd_id, &input)
}

/// GetReportData.
pub(crate) fn get_report_data(
    service: &Session,
    cmd_id: u32,
) -> Result<UsbDsReportData, DispatchError> {
    dispatch_domain_out::<UsbDsReportData>(service, cmd_id)
}

/// StallCtrl.
pub(crate) fn stall_ctrl(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, cmd_id)
}

/// GetDsEndpoint (pre-5.0.0, cmd 0). Input buffer, returns domain object.
pub(crate) fn intf_get_ds_endpoint(
    service: &Session,
    descriptor: &[u8],
) -> Result<u32, RegisterEndpointError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::INTF_REGISTER_ENDPOINT)
        .in_buffer(descriptor, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)
        .map_err(RegisterEndpointError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(RegisterEndpointError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// RegisterEndpoint (5.0.0+, cmd 0). Takes endpoint address, returns domain object.
pub(crate) fn intf_register_endpoint(
    service: &Session,
    endpoint_address: u8,
) -> Result<u32, RegisterEndpointError> {
    // SAFETY: `endpoint_address` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const endpoint_address).cast::<u8>(), size_of::<u8>())
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(proto::INTF_REGISTER_ENDPOINT)
        .in_raw(in_bytes)
        .send(&mut ipc_buf)
        .map_err(RegisterEndpointError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(RegisterEndpointError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// AppendConfigurationData (pre-11.0.0, cmd 12). Takes intf_num + speed + buffer.
pub(crate) fn intf_append_configuration_data_legacy(
    service: &Session,
    intf_num: u8,
    speed: u32,
    buffer: &[u8],
) -> Result<(), DispatchError> {
    let input = AppendConfigDataLegacyIn {
        intf_num,
        _pad: [0; 3],
        speed,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<AppendConfigDataLegacyIn>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::INTF_APPEND_CONFIGURATION_DATA_LEGACY)
        .in_raw(in_bytes)
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// AppendConfigurationData (11.0.0+, cmd 10). Takes speed + buffer.
pub(crate) fn intf_append_configuration_data(
    service: &Session,
    speed: u32,
    buffer: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `speed` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const speed).cast::<u8>(), size_of::<u32>()) };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    service
        .dispatch(proto::INTF_APPEND_CONFIGURATION_DATA)
        .in_raw(in_bytes)
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// IDsEndpoint commands
// ---------------------------------------------------------------------------

/// Endpoint Cancel (cmd 1).
pub(crate) fn ep_cancel(service: &Session) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, proto::EP_CANCEL)
}

/// Endpoint Stall (cmd 4).
pub(crate) fn ep_stall(service: &Session) -> Result<(), DispatchError> {
    dispatch_domain_no_io(service, proto::EP_STALL)
}

/// Endpoint SetZlt (cmd 5).
pub(crate) fn ep_set_zlt(service: &Session, zlt: bool) -> Result<(), DispatchError> {
    let val: u8 = u8::from(zlt);
    dispatch_domain_in_no_out(service, proto::EP_SET_ZLT, &val)
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned by event acquisition operations.
#[derive(Debug, thiserror::Error)]
pub enum GetEventError {
    /// IPC dispatch failed.
    #[error("failed to dispatch event request")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected copy handle.
    #[error("event response missing copy handle")]
    MissingHandle,
}

/// Error returned by GetDsInterface / RegisterInterface.
#[derive(Debug, thiserror::Error)]
pub enum GetInterfaceError {
    /// IPC dispatch failed.
    #[error("failed to dispatch GetDsInterface")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain object handle.
    #[error("GetDsInterface response missing move handle")]
    MissingHandle,
}

/// Error returned by RegisterInterface (5.0.0+).
#[derive(Debug, thiserror::Error)]
pub enum RegisterInterfaceError {
    /// IPC dispatch failed.
    #[error("failed to dispatch RegisterInterface")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain object handle.
    #[error("RegisterInterface response missing move handle")]
    MissingHandle,
}

/// Error returned by endpoint registration.
#[derive(Debug, thiserror::Error)]
pub enum RegisterEndpointError {
    /// IPC dispatch failed.
    #[error("failed to dispatch RegisterEndpoint")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain object handle.
    #[error("RegisterEndpoint response missing move handle")]
    MissingHandle,
}
