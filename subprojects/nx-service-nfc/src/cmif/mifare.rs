//! CMIF protocol operations for the NFC Mifare interface (nfc:mf:u).

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainObjectRef,
    DomainRef,
    OutHandleAttr,
};

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_in_out,
        dispatch_out,
    },
    proto,
    types::{
        InitializeIn,
        NfcDeviceHandle,
        NfcMifareReadBlockData,
        NfcMifareReadBlockParameter,
        NfcMifareWriteBlockParameter,
        NfcRequiredMcuVersionData,
        NfcTagInfo,
    },
};

/// CreateInterface — returns a domain sub-object ID. The freshly minted
/// The close obligation is handed on rather than discharged: the caller
/// re-addresses the id through the long-lived parent domain.
pub(crate) fn create_interface(domain: DomainRef<'_>) -> Result<u32, CreateInterfaceError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

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

/// Initialize.
pub(crate) fn initialize(
    object: DomainObjectRef<'_>,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
) -> Result<(), DispatchError> {
    let input = InitializeIn { aruid, zero: 0 };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<InitializeIn>()` bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const input).cast::<u8>(), size_of::<InitializeIn>())
    };
    // SAFETY: `version_data` is a valid slice; viewing it as bytes for the
    // IN buffer is sound.
    let version_bytes = unsafe {
        core::slice::from_raw_parts(
            version_data.as_ptr().cast::<u8>(),
            core::mem::size_of_val(version_data),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::MF_INITIALIZE)
        .in_raw(in_bytes)
        .in_buffer(version_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send_pid()
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Finalize.
pub(crate) fn finalize(object: DomainObjectRef<'_>) -> Result<(), DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::MF_FINALIZE)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// ListDevices.
pub(crate) fn list_devices(
    object: DomainObjectRef<'_>,
    out: &mut [NfcDeviceHandle],
) -> Result<i32, DispatchError> {
    // SAFETY: `out` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::MF_LIST_DEVICES)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)?;

    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// StartDetection.
pub(crate) fn start_detection(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::MF_START_DETECTION, *handle)
}

/// StopDetection.
pub(crate) fn stop_detection(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::MF_STOP_DETECTION, *handle)
}

/// ReadMifare.
pub(crate) fn read_mifare(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out_block_data: &mut [NfcMifareReadBlockData],
    read_block_parameter: &[NfcMifareReadBlockParameter],
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *handle).cast::<u8>(),
            size_of::<NfcDeviceHandle>(),
        )
    };
    // SAFETY: `out_block_data` is a valid `&mut` slice; viewing it as bytes
    // for the OUT buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_block_data.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_block_data),
        )
    };
    // SAFETY: `read_block_parameter` is a valid slice; viewing it as bytes
    // for the IN buffer is sound.
    let param_bytes = unsafe {
        core::slice::from_raw_parts(
            read_block_parameter.as_ptr().cast::<u8>(),
            core::mem::size_of_val(read_block_parameter),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::MF_READ_MIFARE)
        .in_raw(in_bytes)
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(param_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// WriteMifare.
pub(crate) fn write_mifare(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    write_block_parameter: &[NfcMifareWriteBlockParameter],
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *handle).cast::<u8>(),
            size_of::<NfcDeviceHandle>(),
        )
    };
    // SAFETY: `write_block_parameter` is a valid slice; viewing it as bytes
    // for the IN buffer is sound.
    let param_bytes = unsafe {
        core::slice::from_raw_parts(
            write_block_parameter.as_ptr().cast::<u8>(),
            core::mem::size_of_val(write_block_parameter),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::MF_WRITE_MIFARE)
        .in_raw(in_bytes)
        .in_buffer(param_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// GetTagInfo.
pub(crate) fn get_tag_info(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
    out: &mut NfcTagInfo,
) -> Result<(), DispatchError> {
    // SAFETY: `*handle` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *handle).cast::<u8>(),
            size_of::<NfcDeviceHandle>(),
        )
    };
    // SAFETY: `out` is a valid `&mut NfcTagInfo`; viewing its bytes for the
    // OUT pointer buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut NfcTagInfo).cast::<u8>(),
            size_of::<NfcTagInfo>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    object
        .dispatch(proto::MF_GET_TAG_INFO)
        .in_raw(in_bytes)
        .out_buffer(
            out_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// AttachActivateEvent.
pub(crate) fn attach_activate_event(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    // SAFETY: `*handle` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *handle).cast::<u8>(),
            size_of::<NfcDeviceHandle>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::MF_ATTACH_ACTIVATE_EVENT)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;
    Ok(result.copy_handles[0])
}

/// AttachDeactivateEvent.
pub(crate) fn attach_deactivate_event(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    // SAFETY: `*handle` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const *handle).cast::<u8>(),
            size_of::<NfcDeviceHandle>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::MF_ATTACH_DEACTIVATE_EVENT)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;
    Ok(result.copy_handles[0])
}

/// AttachAvailabilityChangeEvent.
pub(crate) fn attach_availability_change_event(
    object: DomainObjectRef<'_>,
) -> Result<u32, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::MF_ATTACH_AVAILABILITY_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)?;
    Ok(result.copy_handles[0])
}

/// GetState.
pub(crate) fn get_state(object: DomainObjectRef<'_>) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::MF_GET_STATE)
}

/// GetDeviceState.
pub(crate) fn get_device_state(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::MF_GET_DEVICE_STATE, *handle)
}

/// GetNpadId.
pub(crate) fn get_npad_id(
    object: DomainObjectRef<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::MF_GET_NPAD_ID, *handle)
}
