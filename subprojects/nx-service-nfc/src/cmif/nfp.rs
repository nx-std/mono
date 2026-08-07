//! CMIF protocol operations for the NFP (amiibo) interface.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
    DomainRef,
    OutHandleAttr,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_in_out,
        dispatch_no_io,
        dispatch_out,
    },
    proto,
    types::{
        BreakTagIn,
        DeviceHandleAppIdIn,
        InitializeIn,
        MountIn,
        NfcDeviceHandle,
        NfcRequiredMcuVersionData,
        NfpAdminInfo,
        NfpCommonInfo,
        NfpData,
        NfpModelInfo,
        NfpRegisterInfo,
        NfpRegisterInfoPrivate,
        NfpTagInfo,
        WriteNtfIn,
    },
};

/// CreateInterface — returns a domain sub-object ID. The freshly minted
/// The close obligation is handed on rather than discharged: the caller
/// re-addresses the id through the long-lived parent domain.
pub(crate) fn create_interface(domain: DomainRef<'_>) -> Result<u32, CreateInterfaceError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let mut result = domain
        .dispatch(proto::CREATE_INTERFACE)
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(CreateInterfaceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateInterfaceError::MissingObject)?;
    Ok(object.into_raw_object_id())
}

/// Error returned by [`create_interface`].
#[derive(Debug, thiserror::Error)]
pub enum CreateInterfaceError {
    #[error("failed to dispatch CreateInterface")]
    Dispatch(#[source] DispatchError),
    #[error("CreateInterface response did not include the expected sub-object")]
    MissingObject,
}

/// Initialize — sends PID + ARUID + MCU version buffer.
pub(crate) fn initialize(
    object: DomainObjectRef<'_>,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
) -> Result<(), DispatchError> {
    let input = InitializeIn { aruid, zero: 0 };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_INITIALIZE)
        .in_raw(input.as_bytes())
        .in_buffer(version_data.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Finalize.
pub(crate) fn finalize(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::NFP_FINALIZE)
}

/// ListDevices — writes device handles to buffer, returns count.
pub(crate) fn list_devices(
    object: DomainObjectRef<'_>,
    out: &mut [NfcDeviceHandle],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::NFP_LIST_DEVICES)
        .out_size(size_of::<i32>())
        .out_buffer(out.as_mut_bytes(), BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// StartDetection (device handle).
pub(crate) fn start_detection(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_START_DETECTION, *handle)
}

/// StopDetection (device handle).
pub(crate) fn stop_detection(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_STOP_DETECTION, *handle)
}

/// Mount (device handle + device type + mount target).
pub(crate) fn mount(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    device_type: u32,
    mount_target: u32,
) -> Result<(), DispatchError> {
    let input = MountIn {
        handle: *handle,
        device_type,
        mount_target,
    };
    dispatch_in(object, proto::NFP_MOUNT, input)
}

/// Unmount (device handle).
pub(crate) fn unmount(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_UNMOUNT, *handle)
}

/// OpenApplicationArea.
pub(crate) fn open_application_area(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    app_id: u32,
) -> Result<(), DispatchError> {
    let input = DeviceHandleAppIdIn {
        handle: *handle,
        app_id,
    };
    dispatch_in(object, proto::NFP_OPEN_APPLICATION_AREA, input)
}

/// GetApplicationArea — writes to buffer, returns size read.
pub(crate) fn get_application_area(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::NFP_GET_APPLICATION_AREA)
        .in_raw(handle.as_bytes())
        .out_size(size_of::<u32>())
        .out_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// SetApplicationArea.
pub(crate) fn set_application_area(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_SET_APPLICATION_AREA)
        .in_raw(handle.as_bytes())
        .in_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Flush.
pub(crate) fn flush(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_FLUSH, *handle)
}

/// Restore.
pub(crate) fn restore(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_RESTORE, *handle)
}

/// CreateApplicationArea.
pub(crate) fn create_application_area(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    app_id: u32,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let input = DeviceHandleAppIdIn {
        handle: *handle,
        app_id,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_CREATE_APPLICATION_AREA)
        .in_raw(input.as_bytes())
        .in_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// RecreateApplicationArea. [3.0.0+]
pub(crate) fn recreate_application_area(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    app_id: u32,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let input = DeviceHandleAppIdIn {
        handle: *handle,
        app_id,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_RECREATE_APPLICATION_AREA)
        .in_raw(input.as_bytes())
        .in_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetApplicationAreaSize.
pub(crate) fn get_application_area_size(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFP_GET_APPLICATION_AREA_SIZE, *handle)
}

/// GetTagInfo — writes fixed-size buffer output.
pub(crate) fn get_tag_info(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpTagInfo,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_GET_TAG_INFO)
        .in_raw(handle.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetRegisterInfo.
pub(crate) fn get_register_info(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpRegisterInfo,
) -> Result<(), DispatchError> {
    // Still hand-rolled: `NfpRegisterInfo` embeds a `nx-service-mii` type that does
    // not derive the `zerocopy` traits, so the struct cannot derive them either.
    // SAFETY: `out` is a valid `&mut NfpRegisterInfo`; viewing its bytes for
    // the OUT pointer buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut NfpRegisterInfo).cast::<u8>(),
            size_of::<NfpRegisterInfo>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_GET_REGISTER_INFO)
        .in_raw(handle.as_bytes())
        .out_buffer(
            out_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetCommonInfo.
pub(crate) fn get_common_info(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpCommonInfo,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_GET_COMMON_INFO)
        .in_raw(handle.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetModelInfo.
pub(crate) fn get_model_info(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpModelInfo,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_GET_MODEL_INFO)
        .in_raw(handle.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// AttachActivateEvent — returns a copy handle.
pub(crate) fn attach_activate_event(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::NFP_ATTACH_ACTIVATE_EVENT)
        .in_raw(handle.as_bytes())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;
    Ok(result.copy_handles[0])
}

/// AttachDeactivateEvent — returns a copy handle.
pub(crate) fn attach_deactivate_event(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::NFP_ATTACH_DEACTIVATE_EVENT)
        .in_raw(handle.as_bytes())
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;
    Ok(result.copy_handles[0])
}

/// AttachAvailabilityChangeEvent — returns a copy handle. [3.0.0+]
pub(crate) fn attach_availability_change_event(
    object: DomainObjectRef<'_>,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::NFP_ATTACH_AVAILABILITY_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;
    Ok(result.copy_handles[0])
}

/// GetState.
pub(crate) fn get_state(object: DomainObjectRef<'_>) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::NFP_GET_STATE)
}

/// GetDeviceState.
pub(crate) fn get_device_state(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFP_GET_DEVICE_STATE, *handle)
}

/// GetNpadId.
pub(crate) fn get_npad_id(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFP_GET_NPAD_ID, *handle)
}

/// Format (not for User).
pub(crate) fn format(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_FORMAT, *handle)
}

/// GetAdminInfo (not for User).
pub(crate) fn get_admin_info(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpAdminInfo,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_GET_ADMIN_INFO)
        .in_raw(handle.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetRegisterInfoPrivate (not for User).
pub(crate) fn get_register_info_private(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpRegisterInfoPrivate,
) -> Result<(), DispatchError> {
    // Still hand-rolled: `NfpRegisterInfoPrivate` embeds a `nx-service-mii` type that does
    // not derive the `zerocopy` traits, so the struct cannot derive them either.
    // SAFETY: `out` is a valid `&mut NfpRegisterInfoPrivate`; viewing its
    // bytes for the OUT pointer buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut NfpRegisterInfoPrivate).cast::<u8>(),
            size_of::<NfpRegisterInfoPrivate>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_GET_REGISTER_INFO_PRIVATE)
        .in_raw(handle.as_bytes())
        .out_buffer(
            out_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetRegisterInfoPrivate (not for User).
pub(crate) fn set_register_info_private(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    info: &NfpRegisterInfoPrivate,
) -> Result<(), DispatchError> {
    // Still hand-rolled: `NfpRegisterInfoPrivate` embeds a `nx-service-mii` type that does
    // not derive the `zerocopy` traits, so the struct cannot derive them either.
    // SAFETY: `info` is a valid reference; viewing its bytes for the IN
    // pointer buffer is sound.
    let info_bytes = unsafe {
        core::slice::from_raw_parts(
            (info as *const NfpRegisterInfoPrivate).cast::<u8>(),
            size_of::<NfpRegisterInfoPrivate>(),
        )
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_SET_REGISTER_INFO_PRIVATE)
        .in_raw(handle.as_bytes())
        .in_buffer(
            info_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// DeleteRegisterInfo (not for User).
pub(crate) fn delete_register_info(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_DELETE_REGISTER_INFO, *handle)
}

/// DeleteApplicationArea (not for User).
pub(crate) fn delete_application_area(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_DELETE_APPLICATION_AREA, *handle)
}

/// ExistsApplicationArea (not for User).
pub(crate) fn exists_application_area(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<bool, DispatchError> {
    let val: u32 = dispatch_in_out(object, proto::NFP_EXISTS_APPLICATION_AREA, *handle)?;
    Ok(val != 0)
}

/// GetAll (debug only).
pub(crate) fn get_all(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpData,
) -> Result<(), DispatchError> {
    // Still hand-rolled: `NfpData` embeds a `nx-service-mii` type that does
    // not derive the `zerocopy` traits, so the struct cannot derive them either.
    // SAFETY: `out` is a valid `&mut NfpData`; viewing its bytes for the
    // OUT pointer buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut((out as *mut NfpData).cast::<u8>(), size_of::<NfpData>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_GET_ALL)
        .in_raw(handle.as_bytes())
        .out_buffer(
            out_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// SetAll (debug only).
pub(crate) fn set_all(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    data: &NfpData,
) -> Result<(), DispatchError> {
    // Still hand-rolled: `NfpData` embeds a `nx-service-mii` type that does
    // not derive the `zerocopy` traits, so the struct cannot derive them either.
    // SAFETY: `data` is a valid reference; viewing its bytes for the IN
    // pointer buffer is sound.
    let data_bytes = unsafe {
        core::slice::from_raw_parts((data as *const NfpData).cast::<u8>(), size_of::<NfpData>())
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_SET_ALL)
        .in_raw(handle.as_bytes())
        .in_buffer(
            data_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// FlushDebug (debug only).
pub(crate) fn flush_debug(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_FLUSH_DEBUG, *handle)
}

/// BreakTag (debug only).
pub(crate) fn break_tag(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    break_type: u32,
) -> Result<(), DispatchError> {
    let input = BreakTagIn {
        handle: *handle,
        break_type,
    };
    dispatch_in(object, proto::NFP_BREAK_TAG, input)
}

/// ReadBackupData (debug only).
pub(crate) fn read_backup_data(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .dispatch(proto::NFP_READ_BACKUP_DATA)
        .in_raw(handle.as_bytes())
        .out_size(size_of::<u32>())
        .out_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// WriteBackupData (debug only).
pub(crate) fn write_backup_data(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_WRITE_BACKUP_DATA)
        .in_raw(handle.as_bytes())
        .in_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// WriteNtf (debug only).
pub(crate) fn write_ntf(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    write_type: u32,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let input = WriteNtfIn {
        handle: *handle,
        write_type,
    };
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .dispatch(proto::NFP_WRITE_NTF)
        .in_raw(input.as_bytes())
        .in_buffer(buf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}
