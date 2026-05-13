//! CMIF protocol operations for the USB host stack service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{dispatch_domain_in_no_out, dispatch_domain_in_out},
    proto,
    types::{
        CreateInterfaceAvailableEventIn, CtrlXferAsyncIn, EpBatchBufferAsyncIn,
        EpCreateSmmuSpaceIn, EpPostBufferAsyncIn, EpSubmitRequestIn, OpenUsbEpIn,
        SubmitControlRequestIn, UsbEndpointDescriptor, UsbHsInterfaceFilter, UsbHsInterfaceInfo,
        UsbHsXferReport,
    },
};

// ---------------------------------------------------------------------------
// IUsbHsService — root service commands
// ---------------------------------------------------------------------------

/// BindClientProcess (2.0.0+, cmd 0). Sends process handle as copy-handle.
pub(crate) fn bind_client_process(
    service: &Session,
    proc_handle: u32,
) -> Result<(), DispatchError> {
    service
        .dispatch(proto::BIND_CLIENT_PROCESS)
        .in_handle(proc_handle)
        .send()
        .map(|_| ())
}

/// QueryAllInterfaces / QueryAvailableInterfaces.
/// In: filter. Out: HipcMapAlias buffer + i32 count.
pub(crate) fn query_interfaces_with_filter(
    service: &Session,
    cmd_id: u32,
    filter: &UsbHsInterfaceFilter,
    interfaces: *mut u8,
    interfaces_size: usize,
) -> Result<i32, DispatchError> {
    // SAFETY: `filter` and `interfaces` live until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw(
                (&raw const *filter).cast::<u8>(),
                size_of::<UsbHsInterfaceFilter>(),
            )
            .buffer(
                interfaces,
                interfaces_size,
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .out_size(size_of::<i32>())
            .send()?
    };

    // SAFETY: response payload is at least size_of::<i32>().
    let count = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) };

    Ok(count)
}

/// QueryAcquiredInterfaces.
/// Out: HipcMapAlias buffer + i32 count.
pub(crate) fn query_acquired_interfaces(
    service: &Session,
    cmd_id: u32,
    interfaces: *mut u8,
    interfaces_size: usize,
) -> Result<i32, DispatchError> {
    let result = service
        .dispatch(cmd_id)
        .buffer(
            interfaces,
            interfaces_size,
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .out_size(size_of::<i32>())
        .send()?;

    // SAFETY: response payload is at least size_of::<i32>().
    let count = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<i32>()) };

    Ok(count)
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

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<CreateInterfaceAvailableEventIn>(),
            )
            .send()
            .map_err(GetEventError::Dispatch)?
    };

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
    let result = service
        .dispatch(cmd_id)
        .send()
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
    info_out: *mut UsbHsInterfaceInfo,
) -> Result<u32, AcquireIfError> {
    // SAFETY: `interface_id` and `info_out` live until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::ACQUIRE_USB_IF_LEGACY)
            .in_raw((&raw const interface_id).cast::<u8>(), size_of::<i32>())
            .buffer(
                info_out.cast::<u8>(),
                size_of::<UsbHsInterfaceInfo>(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map_err(AcquireIfError::Dispatch)?
    };

    if result.move_handles.is_empty() {
        return Err(AcquireIfError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

/// AcquireUsbIf (2.0.0+, cmd 7).
/// In: i32 ID. Out: 2 HipcMapAlias buffers (pathstr area + InterfaceInfo) + domain object.
pub(crate) fn acquire_usb_if(
    service: &Session,
    interface_id: i32,
    intf_data_out: *mut u8,
    intf_data_size: usize,
    info_out: *mut UsbHsInterfaceInfo,
) -> Result<u32, AcquireIfError> {
    // SAFETY: `interface_id`, `intf_data_out`, `info_out` live until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::ACQUIRE_USB_IF)
            .in_raw((&raw const interface_id).cast::<u8>(), size_of::<i32>())
            .buffer(
                intf_data_out,
                intf_data_size,
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .buffer(
                info_out.cast::<u8>(),
                size_of::<UsbHsInterfaceInfo>(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map_err(AcquireIfError::Dispatch)?
    };

    if result.move_handles.is_empty() {
        return Err(AcquireIfError::MissingHandle);
    }

    Ok(result.move_handles[0])
}

// ---------------------------------------------------------------------------
// IClientIfSession commands
// ---------------------------------------------------------------------------

/// SetInterface (cmd 1). In: u8 id. Out: HipcMapAlias buffer (UsbHsInterfaceInfo).
pub(crate) fn if_set_interface(
    service: &Session,
    id: u8,
    info_out: *mut UsbHsInterfaceInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `id` and `info_out` live until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::IF_SET_INTERFACE)
            .in_raw((&raw const id).cast::<u8>(), size_of::<u8>())
            .buffer(
                info_out.cast::<u8>(),
                size_of::<UsbHsInterfaceInfo>(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// GetInterface (cmd 2). Out: HipcMapAlias buffer (UsbHsInterfaceInfo).
pub(crate) fn if_get_interface(
    service: &Session,
    info_out: *mut UsbHsInterfaceInfo,
) -> Result<(), DispatchError> {
    service
        .dispatch(proto::IF_GET_INTERFACE)
        .buffer(
            info_out.cast::<u8>(),
            size_of::<UsbHsInterfaceInfo>(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
        .map(|_| ())
}

/// GetAlternateInterface (cmd 3). In: u8 id. Out: HipcMapAlias buffer.
pub(crate) fn if_get_alternate_interface(
    service: &Session,
    id: u8,
    info_out: *mut UsbHsInterfaceInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `id` and `info_out` live until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::IF_GET_ALTERNATE_INTERFACE)
            .in_raw((&raw const id).cast::<u8>(), size_of::<u8>())
            .buffer(
                info_out.cast::<u8>(),
                size_of::<UsbHsInterfaceInfo>(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// GetCurrentFrame. Out: u32.
pub(crate) fn if_get_current_frame(service: &Session, cmd_id: u32) -> Result<u32, DispatchError> {
    crate::dispatch::dispatch_domain_out::<u32>(service, cmd_id)
}

/// SubmitControlRequest (pre-2.0.0). Direction determined by cmd_id (6=IN, 7=OUT).
/// In: SubmitControlRequestIn. Buffer: HipcMapAlias (IN or OUT based on direction).
/// Out: u32 transferred size.
#[allow(clippy::too_many_arguments)]
pub(crate) fn if_submit_control_request(
    service: &Session,
    cmd_id: u32,
    b_request: u8,
    bm_request_type: u8,
    w_value: u16,
    w_index: u16,
    w_length: u16,
    buffer: *mut u8,
    buffer_size: usize,
    timeout_in_ms: u32,
) -> Result<u32, DispatchError> {
    let is_in = cmd_id == proto::IF_SUBMIT_CONTROL_REQUEST_IN;
    let buf_attr = if is_in {
        BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS)
    } else {
        BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS)
    };

    let input = SubmitControlRequestIn {
        b_request,
        bm_request_type,
        w_value,
        w_index,
        w_length,
        timeout_in_ms,
    };

    // SAFETY: `input` and `buffer` live until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<SubmitControlRequestIn>(),
            )
            .buffer(buffer, buffer_size, buf_attr)
            .out_size(size_of::<u32>())
            .send()?
    };

    // SAFETY: response payload is at least size_of::<u32>().
    let transferred = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(transferred)
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
    report_out: *mut UsbHsXferReport,
) -> Result<(), DispatchError> {
    service
        .dispatch(proto::IF_GET_CTRL_XFER_REPORT)
        .buffer(
            report_out.cast::<u8>(),
            size_of::<UsbHsXferReport>(),
            BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
        )
        .send()
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

    // SAFETY: `input` lives on the stack until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw((&raw const input).cast::<u8>(), size_of::<OpenUsbEpIn>())
            .out_size(size_of::<UsbEndpointDescriptor>())
            .send()
            .map_err(OpenEpError::Dispatch)?
    };

    if result.move_handles.is_empty() {
        return Err(OpenEpError::MissingHandle);
    }

    // SAFETY: response payload is at least size_of::<UsbEndpointDescriptor>().
    let desc =
        unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<UsbEndpointDescriptor>()) };

    Ok((result.move_handles[0], desc))
}

// ---------------------------------------------------------------------------
// IClientEpSession commands
// ---------------------------------------------------------------------------

/// SubmitRequest (pre-2.0.0). Direction determined by cmd_id (0=OUT, 1=IN).
/// In: EpSubmitRequestIn. Buffer: HipcMapAlias (IN or OUT). Out: u32 transferred size.
pub(crate) fn ep_submit_request(
    service: &Session,
    cmd_id: u32,
    size: u32,
    timeout_in_ms: u32,
    buffer: *mut u8,
    buffer_size: usize,
) -> Result<u32, DispatchError> {
    let is_in = cmd_id == proto::EP_SUBMIT_REQUEST_IN;
    let buf_attr = if is_in {
        BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS)
    } else {
        BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS)
    };

    let input = EpSubmitRequestIn {
        size,
        timeout_in_ms,
    };

    // SAFETY: `input` and `buffer` live until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(cmd_id)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<EpSubmitRequestIn>(),
            )
            .buffer(buffer, buffer_size, buf_attr)
            .out_size(size_of::<u32>())
            .send()?
    };

    // SAFETY: response payload is at least size_of::<u32>().
    let transferred = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(transferred)
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
    reports: *mut u8,
    reports_size: usize,
    buf_attr: BufferAttr,
) -> Result<u32, DispatchError> {
    // SAFETY: `max_reports` and `reports` live until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::EP_GET_XFER_REPORT)
            .in_raw((&raw const max_reports).cast::<u8>(), size_of::<u32>())
            .buffer(reports, reports_size, buf_attr)
            .out_size(size_of::<u32>())
            .send()?
    };

    // SAFETY: response payload is at least size_of::<u32>().
    let count = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(count)
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
    urbs: *const u8,
    urbs_size: usize,
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

    // SAFETY: `input` and `urbs` live until `.send()` returns.
    let result = unsafe {
        service
            .dispatch(proto::EP_BATCH_BUFFER_ASYNC)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<EpBatchBufferAsyncIn>(),
            )
            .buffer(urbs.cast_mut(), urbs_size, buf_attr)
            .out_size(size_of::<u32>())
            .send()?
    };

    // SAFETY: response payload is at least size_of::<u32>().
    let xfer_id = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) };

    Ok(xfer_id)
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
    // SAFETY: `size` and `tmem_handle` live until `.send()` returns.
    unsafe {
        service
            .dispatch(proto::EP_SHARE_REPORT_RING)
            .in_raw((&raw const size).cast::<u8>(), size_of::<u64>())
            .in_handle(tmem_handle)
            .send()
            .map(|_| ())
    }
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
