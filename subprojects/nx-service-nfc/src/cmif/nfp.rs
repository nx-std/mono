//! CMIF protocol operations for the NFP (amiibo) interface.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Domain, DomainObject, OutHandleAttr};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io, dispatch_out},
    proto,
    types::{
        BreakTagIn, DeviceHandleAppIdIn, InitializeIn, MountIn, NfcDeviceHandle,
        NfcRequiredMcuVersionData, NfpAdminInfo, NfpCommonInfo, NfpData, NfpModelInfo,
        NfpRegisterInfo, NfpRegisterInfoPrivate, NfpTagInfo, WriteNtfIn,
    },
};

// ---------------------------------------------------------------------------
// Root domain commands
// ---------------------------------------------------------------------------

/// CreateInterface — returns a domain sub-object ID.
pub(crate) fn create_interface(domain: &Domain) -> Result<u32, CreateInterfaceError> {
    let result = domain
        .dispatch(proto::CREATE_INTERFACE)
        .out_objects(1)
        .send()
        .map_err(CreateInterfaceError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(CreateInterfaceError::MissingObject);
    }
    Ok(result.objects[0])
}

/// Error returned by [`create_interface`].
#[derive(Debug, thiserror::Error)]
pub enum CreateInterfaceError {
    #[error("failed to dispatch CreateInterface")]
    Dispatch(#[source] DispatchError),
    #[error("CreateInterface response did not include the expected sub-object")]
    MissingObject,
}

// ---------------------------------------------------------------------------
// Interface initialization / finalization
// ---------------------------------------------------------------------------

/// Initialize — sends PID + ARUID + MCU version buffer.
pub(crate) fn initialize(
    object: &DomainObject<'_>,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
) -> Result<(), DispatchError> {
    let input = InitializeIn { aruid, zero: 0 };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_INITIALIZE)
            .in_raw((&raw const input).cast::<u8>(), size_of::<InitializeIn>())
            .buffer(
                version_data.as_ptr().cast::<u8>(),
                core::mem::size_of_val(version_data),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send_pid()
            .send()
            .map(|_| ())
    }
}

/// Finalize.
pub(crate) fn finalize(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::NFP_FINALIZE)
}

// ---------------------------------------------------------------------------
// Device management
// ---------------------------------------------------------------------------

/// ListDevices — writes device handles to buffer, returns count.
pub(crate) fn list_devices(
    object: &DomainObject<'_>,
    out: &mut [NfcDeviceHandle],
) -> Result<i32, DispatchError> {
    let result = object
        .dispatch(proto::NFP_LIST_DEVICES)
        .out_size(size_of::<i32>())
        .buffer(
            out.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out),
            BufferAttr::OUT.or(BufferAttr::HIPC_POINTER),
        )
        .send()?;

    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// StartDetection (device handle).
pub(crate) fn start_detection(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_START_DETECTION, *handle)
}

/// StopDetection (device handle).
pub(crate) fn stop_detection(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_STOP_DETECTION, *handle)
}

/// Mount (device handle + device type + mount target).
pub(crate) fn mount(
    object: &DomainObject<'_>,
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
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_UNMOUNT, *handle)
}

// ---------------------------------------------------------------------------
// Application area
// ---------------------------------------------------------------------------

/// OpenApplicationArea.
pub(crate) fn open_application_area(
    object: &DomainObject<'_>,
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
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    let result = unsafe {
        object
            .dispatch(proto::NFP_GET_APPLICATION_AREA)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .out_size(size_of::<u32>())
            .buffer(
                buf.as_mut_ptr(),
                buf.len(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()?
    };

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// SetApplicationArea.
pub(crate) fn set_application_area(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    buf: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_SET_APPLICATION_AREA)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                buf.as_ptr(),
                buf.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// Flush.
pub(crate) fn flush(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_FLUSH, *handle)
}

/// Restore.
pub(crate) fn restore(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_RESTORE, *handle)
}

/// CreateApplicationArea.
pub(crate) fn create_application_area(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    app_id: u32,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let input = DeviceHandleAppIdIn {
        handle: *handle,
        app_id,
    };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_CREATE_APPLICATION_AREA)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<DeviceHandleAppIdIn>(),
            )
            .buffer(
                buf.as_ptr(),
                buf.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// RecreateApplicationArea. [3.0.0+]
pub(crate) fn recreate_application_area(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    app_id: u32,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let input = DeviceHandleAppIdIn {
        handle: *handle,
        app_id,
    };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_RECREATE_APPLICATION_AREA)
            .in_raw(
                (&raw const input).cast::<u8>(),
                size_of::<DeviceHandleAppIdIn>(),
            )
            .buffer(
                buf.as_ptr(),
                buf.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// GetApplicationAreaSize.
pub(crate) fn get_application_area_size(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFP_GET_APPLICATION_AREA_SIZE, *handle)
}

// ---------------------------------------------------------------------------
// Tag / info queries
// ---------------------------------------------------------------------------

/// GetTagInfo — writes fixed-size buffer output.
pub(crate) fn get_tag_info(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpTagInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_GET_TAG_INFO)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (out as *mut NfpTagInfo).cast::<u8>(),
                size_of::<NfpTagInfo>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::OUT),
            )
            .send()
            .map(|_| ())
    }
}

/// GetRegisterInfo.
pub(crate) fn get_register_info(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpRegisterInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_GET_REGISTER_INFO)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (out as *mut NfpRegisterInfo).cast::<u8>(),
                size_of::<NfpRegisterInfo>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::OUT),
            )
            .send()
            .map(|_| ())
    }
}

/// GetCommonInfo.
pub(crate) fn get_common_info(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpCommonInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_GET_COMMON_INFO)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (out as *mut NfpCommonInfo).cast::<u8>(),
                size_of::<NfpCommonInfo>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::OUT),
            )
            .send()
            .map(|_| ())
    }
}

/// GetModelInfo.
pub(crate) fn get_model_info(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpModelInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_GET_MODEL_INFO)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (out as *mut NfpModelInfo).cast::<u8>(),
                size_of::<NfpModelInfo>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::OUT),
            )
            .send()
            .map(|_| ())
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// AttachActivateEvent — returns a copy handle.
pub(crate) fn attach_activate_event(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    let result = unsafe {
        object
            .dispatch(proto::NFP_ATTACH_ACTIVATE_EVENT)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .out_handle(0, OutHandleAttr::Copy)
            .send()?
    };
    Ok(result.copy_handles[0])
}

/// AttachDeactivateEvent — returns a copy handle.
pub(crate) fn attach_deactivate_event(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    let result = unsafe {
        object
            .dispatch(proto::NFP_ATTACH_DEACTIVATE_EVENT)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .out_handle(0, OutHandleAttr::Copy)
            .send()?
    };
    Ok(result.copy_handles[0])
}

/// AttachAvailabilityChangeEvent — returns a copy handle. [3.0.0+]
pub(crate) fn attach_availability_change_event(
    object: &DomainObject<'_>,
) -> Result<u32, DispatchError> {
    let result = object
        .dispatch(proto::NFP_ATTACH_AVAILABILITY_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;
    Ok(result.copy_handles[0])
}

// ---------------------------------------------------------------------------
// State queries
// ---------------------------------------------------------------------------

/// GetState.
pub(crate) fn get_state(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::NFP_GET_STATE)
}

/// GetDeviceState.
pub(crate) fn get_device_state(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFP_GET_DEVICE_STATE, *handle)
}

/// GetNpadId.
pub(crate) fn get_npad_id(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFP_GET_NPAD_ID, *handle)
}

// ---------------------------------------------------------------------------
// System/debug-only commands
// ---------------------------------------------------------------------------

/// Format (not for User).
pub(crate) fn format(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_FORMAT, *handle)
}

/// GetAdminInfo (not for User).
pub(crate) fn get_admin_info(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpAdminInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_GET_ADMIN_INFO)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (out as *mut NfpAdminInfo).cast::<u8>(),
                size_of::<NfpAdminInfo>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::OUT),
            )
            .send()
            .map(|_| ())
    }
}

/// GetRegisterInfoPrivate (not for User).
pub(crate) fn get_register_info_private(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpRegisterInfoPrivate,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_GET_REGISTER_INFO_PRIVATE)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (out as *mut NfpRegisterInfoPrivate).cast::<u8>(),
                size_of::<NfpRegisterInfoPrivate>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::OUT),
            )
            .send()
            .map(|_| ())
    }
}

/// SetRegisterInfoPrivate (not for User).
pub(crate) fn set_register_info_private(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    info: &NfpRegisterInfoPrivate,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_SET_REGISTER_INFO_PRIVATE)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (info as *const NfpRegisterInfoPrivate).cast::<u8>(),
                size_of::<NfpRegisterInfoPrivate>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::IN),
            )
            .send()
            .map(|_| ())
    }
}

/// DeleteRegisterInfo (not for User).
pub(crate) fn delete_register_info(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_DELETE_REGISTER_INFO, *handle)
}

/// DeleteApplicationArea (not for User).
pub(crate) fn delete_application_area(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_DELETE_APPLICATION_AREA, *handle)
}

/// ExistsApplicationArea (not for User).
pub(crate) fn exists_application_area(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<bool, DispatchError> {
    let val: u32 = dispatch_in_out(object, proto::NFP_EXISTS_APPLICATION_AREA, *handle)?;
    Ok(val != 0)
}

// ---------------------------------------------------------------------------
// Debug-only commands
// ---------------------------------------------------------------------------

/// GetAll (debug only).
pub(crate) fn get_all(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfpData,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_GET_ALL)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (out as *mut NfpData).cast::<u8>(),
                size_of::<NfpData>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::OUT),
            )
            .send()
            .map(|_| ())
    }
}

/// SetAll (debug only).
pub(crate) fn set_all(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    data: &NfpData,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_SET_ALL)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                (data as *const NfpData).cast::<u8>(),
                size_of::<NfpData>(),
                BufferAttr::FIXED_SIZE
                    .or(BufferAttr::HIPC_POINTER)
                    .or(BufferAttr::IN),
            )
            .send()
            .map(|_| ())
    }
}

/// FlushDebug (debug only).
pub(crate) fn flush_debug(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFP_FLUSH_DEBUG, *handle)
}

/// BreakTag (debug only).
pub(crate) fn break_tag(
    object: &DomainObject<'_>,
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
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    buf: &mut [u8],
) -> Result<u32, DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    let result = unsafe {
        object
            .dispatch(proto::NFP_READ_BACKUP_DATA)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .out_size(size_of::<u32>())
            .buffer(
                buf.as_mut_ptr(),
                buf.len(),
                BufferAttr::OUT.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()?
    };

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// WriteBackupData (debug only).
pub(crate) fn write_backup_data(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    buf: &[u8],
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_WRITE_BACKUP_DATA)
            .in_raw(
                (&raw const *handle).cast::<u8>(),
                size_of::<NfcDeviceHandle>(),
            )
            .buffer(
                buf.as_ptr(),
                buf.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}

/// WriteNtf (debug only).
pub(crate) fn write_ntf(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    write_type: u32,
    buf: &[u8],
) -> Result<(), DispatchError> {
    let input = WriteNtfIn {
        handle: *handle,
        write_type,
    };
    // SAFETY: `input` lives on the stack until `.send()` returns.
    unsafe {
        object
            .dispatch(proto::NFP_WRITE_NTF)
            .in_raw((&raw const input).cast::<u8>(), size_of::<WriteNtfIn>())
            .buffer(
                buf.as_ptr(),
                buf.len(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            )
            .send()
            .map(|_| ())
    }
}
