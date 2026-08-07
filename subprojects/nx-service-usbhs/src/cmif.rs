//! CMIF protocol operations for the USB host stack service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_domain_in_no_out,
        dispatch_domain_in_out,
    },
    proto,
    types::{
        CreateInterfaceAvailableEventIn,
        CtrlXferAsyncIn,
        EpBatchBufferAsyncIn,
        EpCreateSmmuSpaceIn,
        EpPostBufferAsyncIn,
        EpSubmitRequestIn,
        OpenUsbEpIn,
        SubmitControlRequestIn,
        UsbEndpointDescriptor,
        UsbHsInterface,
        UsbHsInterfaceFilter,
        UsbHsInterfaceInfo,
        UsbHsXferReport,
    },
};

/// BindClientProcess (2.0.0+, cmd 0). Sends process handle as copy-handle.
pub(crate) fn bind_client_process(
    service: &Session,
    proc_handle: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::BIND_CLIENT_PROCESS)
        .in_handle(proc_handle)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// QueryAllInterfaces / QueryAvailableInterfaces.
/// In: filter. Out: HipcMapAlias buffer + i32 count.
pub(crate) fn query_interfaces_with_filter(
    service: &Session,
    cmd_id: u32,
    filter: &UsbHsInterfaceFilter,
    interfaces: &mut [UsbHsInterface],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(filter.as_bytes())
        .out_buffer(interfaces.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<i32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<i32>())
}

/// QueryAcquiredInterfaces.
/// Out: HipcMapAlias buffer + i32 count.
pub(crate) fn query_acquired_interfaces(
    service: &Session,
    cmd_id: u32,
    interfaces: &mut [UsbHsInterface],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .out_buffer(interfaces.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<i32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<i32>())
}

/// CreateInterfaceAvailableEvent. In: index + filter. Out: copy-handle.
pub(crate) fn create_interface_available_event(
    service: &Session,
    cmd_id: u32,
    index: u8,
    filter: &UsbHsInterfaceFilter,
) -> Result<u32, GetEventError> {
    let input = CreateInterfaceAvailableEventIn {
        index,
        _pad: 0,
        filter: *filter,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .send(&mut ipc_buf)
        .map_err(GetEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// DestroyInterfaceAvailableEvent. In: u8 index.
pub(crate) fn destroy_interface_available_event(
    service: &Session,
    cmd_id: u32,
    index: u8,
) -> Result<(), DispatchError> {
    dispatch_domain_in_no_out(service, cmd_id, &index)
}

/// GetInterfaceStateChangeEvent. Out: copy-handle.
pub(crate) fn get_event(service: &Session, cmd_id: u32) -> Result<u32, GetEventError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .send(&mut ipc_buf)
        .map_err(GetEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetEventError::MissingHandle);
    }

    Ok(result.copy_handles[0])
}

/// AcquireUsbIf (pre-2.0.0, cmd 6).
/// In: i32 ID. Out: 1 HipcMapAlias buffer (UsbHsInterfaceInfo) + domain object.
pub(crate) fn acquire_usb_if_legacy(
    service: &Session,
    interface_id: i32,
    info_out: &mut UsbHsInterfaceInfo,
) -> Result<u32, AcquireIfError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::ACQUIRE_USB_IF_LEGACY)
        .in_raw(interface_id.as_bytes())
        .out_buffer(info_out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map_err(AcquireIfError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(AcquireIfError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// AcquireUsbIf (2.0.0+, cmd 7).
/// In: i32 ID. Out: 2 HipcMapAlias buffers (pathstr area + InterfaceInfo) + domain object.
///
/// The server fills the two regions separately, so `intf_out` is split at the
/// end of its leading [`UsbHsInterfaceInfo`]: the tail carries `pathstr`
/// through `timestamp`, the head the interface info.
pub(crate) fn acquire_usb_if(
    service: &Session,
    interface_id: i32,
    intf_out: &mut UsbHsInterface,
) -> Result<u32, AcquireIfError> {
    let (info_bytes, data_bytes) = intf_out
        .as_mut_bytes()
        .split_at_mut(size_of::<UsbHsInterfaceInfo>());
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::ACQUIRE_USB_IF)
        .in_raw(interface_id.as_bytes())
        .out_buffer(data_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_buffer(info_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map_err(AcquireIfError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(AcquireIfError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// SetInterface (cmd 1). In: u8 id. Out: HipcMapAlias buffer (UsbHsInterfaceInfo).
pub(crate) fn if_set_interface(
    service: &Session,
    id: u8,
    info_out: &mut UsbHsInterfaceInfo,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::IF_SET_INTERFACE)
        .in_raw(id.as_bytes())
        .out_buffer(info_out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetInterface (cmd 2). Out: HipcMapAlias buffer (UsbHsInterfaceInfo).
pub(crate) fn if_get_interface(
    service: &Session,
    info_out: &mut UsbHsInterfaceInfo,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::IF_GET_INTERFACE)
        .out_buffer(info_out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetAlternateInterface (cmd 3). In: u8 id. Out: HipcMapAlias buffer.
pub(crate) fn if_get_alternate_interface(
    service: &Session,
    id: u8,
    info_out: &mut UsbHsInterfaceInfo,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::IF_GET_ALTERNATE_INTERFACE)
        .in_raw(id.as_bytes())
        .out_buffer(info_out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetCurrentFrame. Out: u32.
pub(crate) fn if_get_current_frame(service: &Session, cmd_id: u32) -> Result<u32, DispatchError> {
    crate::dispatch::dispatch_domain_out::<u32>(service, cmd_id)
}

/// SubmitControlRequest IN (pre-2.0.0, cmd 6).
/// In: SubmitControlRequestIn. Buffer: HipcMapAlias OUT, filled by the device.
/// Out: u32 transferred size.
#[allow(clippy::too_many_arguments)]
pub(crate) fn if_submit_control_request_in(
    service: &Session,
    b_request: u8,
    bm_request_type: u8,
    w_value: u16,
    w_index: u16,
    w_length: u16,
    buffer: &mut [u8],
    timeout_in_ms: u32,
) -> Result<u32, DispatchError> {
    let input = SubmitControlRequestIn {
        b_request,
        bm_request_type,
        w_value,
        w_index,
        w_length,
        timeout_in_ms,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::IF_SUBMIT_CONTROL_REQUEST_IN)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u32>())
}

/// SubmitControlRequest OUT (pre-2.0.0, cmd 7).
/// In: SubmitControlRequestIn. Buffer: HipcMapAlias IN, written to the device.
/// Out: u32 transferred size.
#[allow(clippy::too_many_arguments)]
pub(crate) fn if_submit_control_request_out(
    service: &Session,
    b_request: u8,
    bm_request_type: u8,
    w_value: u16,
    w_index: u16,
    w_length: u16,
    buffer: &[u8],
    timeout_in_ms: u32,
) -> Result<u32, DispatchError> {
    let input = SubmitControlRequestIn {
        b_request,
        bm_request_type,
        w_value,
        w_index,
        w_length,
        timeout_in_ms,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::IF_SUBMIT_CONTROL_REQUEST_OUT)
        .in_raw(input.as_bytes())
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u32>())
}

/// CtrlXferAsync (2.0.0+, cmd 5). In: CtrlXferAsyncIn.
pub(crate) fn if_ctrl_xfer_async(
    service: &Session,
    bm_request_type: u8,
    b_request: u8,
    w_value: u16,
    w_index: u16,
    w_length: u16,
    buffer: u64,
) -> Result<(), DispatchError> {
    let input = CtrlXferAsyncIn {
        bm_request_type,
        b_request,
        w_value,
        w_index,
        w_length,
        buffer,
    };

    dispatch_domain_in_no_out(service, proto::IF_CTRL_XFER_ASYNC, &input)
}

/// GetCtrlXferReport (2.0.0+, cmd 7). Out: HipcMapAlias buffer (UsbHsXferReport).
pub(crate) fn if_get_ctrl_xfer_report(
    service: &Session,
    report_out: &mut UsbHsXferReport,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::IF_GET_CTRL_XFER_REPORT)
        .out_buffer(report_out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// OpenUsbEp. In: OpenUsbEpIn. Out: UsbEndpointDescriptor + domain object.
pub(crate) fn if_open_usb_ep(
    service: &Session,
    cmd_id: u32,
    max_urb_count: u16,
    max_xfer_size: u32,
    ep_type: u32,
    ep_number: u32,
    ep_direction: u32,
) -> Result<(u32, UsbEndpointDescriptor), OpenEpError> {
    let input = OpenUsbEpIn {
        max_urb_count,
        _pad: 0,
        ep_type,
        ep_number,
        ep_direction,
        max_xfer_size,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(cmd_id)
        .in_raw(input.as_bytes())
        .out_size(size_of::<UsbEndpointDescriptor>())
        .send(&mut ipc_buf)
        .map_err(OpenEpError::Dispatch)?;

    if result.move_handles.is_empty() {
        return Err(OpenEpError::MissingHandle);
    }

    Ok((
        result.move_handles[0],
        *result.value::<UsbEndpointDescriptor>(),
    ))
}

/// SubmitRequest OUT (pre-2.0.0, cmd 0).
/// In: EpSubmitRequestIn. Buffer: HipcMapAlias IN. Out: u32 transferred size.
pub(crate) fn ep_submit_request_out(
    service: &Session,
    size: u32,
    timeout_in_ms: u32,
    buffer: &[u8],
) -> Result<u32, DispatchError> {
    let input = EpSubmitRequestIn {
        size,
        timeout_in_ms,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::EP_SUBMIT_REQUEST_OUT)
        .in_raw(input.as_bytes())
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u32>())
}

/// SubmitRequest IN (pre-2.0.0, cmd 1).
/// In: EpSubmitRequestIn. Buffer: HipcMapAlias OUT. Out: u32 transferred size.
pub(crate) fn ep_submit_request_in(
    service: &Session,
    size: u32,
    timeout_in_ms: u32,
    buffer: &mut [u8],
) -> Result<u32, DispatchError> {
    let input = EpSubmitRequestIn {
        size,
        timeout_in_ms,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::EP_SUBMIT_REQUEST_IN)
        .in_raw(input.as_bytes())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u32>())
}

/// PostBufferAsync (2.0.0+, cmd 4). In: EpPostBufferAsyncIn. Out: u32 xfer_id.
pub(crate) fn ep_post_buffer_async(
    service: &Session,
    size: u32,
    buffer: u64,
    id: u64,
) -> Result<u32, DispatchError> {
    let input = EpPostBufferAsyncIn {
        size,
        _pad: 0,
        buffer,
        id,
    };

    dispatch_domain_in_out::<EpPostBufferAsyncIn, u32>(service, proto::EP_POST_BUFFER_ASYNC, &input)
}

/// GetXferReport (2.0.0+, cmd 5). In: u32 max_reports. Out: buffer + u32 count.
/// Uses HipcMapAlias (pre-3.0.0) or HipcAutoSelect (3.0.0+) depending on `buf_attr`.
pub(crate) fn ep_get_xfer_report(
    service: &Session,
    max_reports: u32,
    reports: &mut [UsbHsXferReport],
    buf_attr: BufferAttr,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::EP_GET_XFER_REPORT)
        .in_raw(max_reports.as_bytes())
        .out_buffer(reports.as_mut_bytes(), buf_attr)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u32>())
}

/// BatchBufferAsync (2.0.0+, cmd 6). In: EpBatchBufferAsyncIn + buffer. Out: u32 xfer_id.
/// Uses HipcMapAlias (pre-3.0.0) or HipcAutoSelect (3.0.0+) depending on `buf_attr`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ep_batch_buffer_async(
    service: &Session,
    urb_count: u32,
    unk1: u32,
    unk2: u32,
    buffer: u64,
    id: u64,
    urbs: &[u32],
    buf_attr: BufferAttr,
) -> Result<u32, DispatchError> {
    let input = EpBatchBufferAsyncIn {
        urb_count,
        unk1,
        unk2,
        _pad: 0,
        buffer,
        id,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::EP_BATCH_BUFFER_ASYNC)
        .in_raw(input.as_bytes())
        .in_buffer(urbs.as_bytes(), buf_attr)
        .out_size(size_of::<u32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u32>())
}

/// CreateSmmuSpace (4.0.0+, cmd 7). In: EpCreateSmmuSpaceIn.
pub(crate) fn ep_create_smmu_space(
    service: &Session,
    size: u32,
    buffer: u64,
) -> Result<(), DispatchError> {
    let input = EpCreateSmmuSpaceIn {
        size,
        _pad: 0,
        buffer,
    };

    dispatch_domain_in_no_out(service, proto::EP_CREATE_SMMU_SPACE, &input)
}

/// ShareReportRing (4.0.0+, cmd 8). In: u64 size + copy-handle (tmem).
pub(crate) fn ep_share_report_ring(
    service: &Session,
    size: u64,
    tmem_handle: u32,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::EP_SHARE_REPORT_RING)
        .in_raw(size.as_bytes())
        .in_handle(tmem_handle)
        .send(&mut ipc_buf)
        .map(|_| ())
}

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

/// Error returned by AcquireUsbIf.
#[derive(Debug, thiserror::Error)]
pub enum AcquireIfError {
    /// IPC dispatch failed.
    #[error("failed to dispatch AcquireUsbIf")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain object handle.
    #[error("AcquireUsbIf response missing move handle")]
    MissingHandle,
}

/// Error returned by OpenUsbEp.
#[derive(Debug, thiserror::Error)]
pub enum OpenEpError {
    /// IPC dispatch failed.
    #[error("failed to dispatch OpenUsbEp")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain object handle.
    #[error("OpenUsbEp response missing move handle")]
    MissingHandle,
}
