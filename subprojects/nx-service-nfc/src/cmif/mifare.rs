//! CMIF protocol operations for the NFC Mifare interface (nfc:mf:u).

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Domain, DomainObject, OutHandleAttr};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_out},
    proto,
    types::{
        InitializeIn, NfcDeviceHandle, NfcMifareReadBlockData, NfcMifareReadBlockParameter,
        NfcMifareWriteBlockParameter, NfcRequiredMcuVersionData, NfcTagInfo,
    },
};

/// CreateInterface — returns a domain sub-object ID. The freshly minted
/// `DomainObject` is wrapped in `ManuallyDrop` so the server-side object
/// outlives this call; the service wrapper re-opens it per request.
pub(crate) fn create_interface(domain: &Domain) -> Result<u32, CreateInterfaceError> {
    let mut result = domain
        .dispatch(proto::CREATE_INTERFACE)
        .out_objects(1)
        .send()
        .map_err(CreateInterfaceError::Dispatch)?;

    let object = result
        .take_object(0)
        .ok_or(CreateInterfaceError::MissingObject)?;
    Ok(core::mem::ManuallyDrop::new(object).object_id().to_raw())
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
    object: &DomainObject<'_>,
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
    object
        .dispatch(proto::MF_INITIALIZE)
        .in_raw(in_bytes)
        .in_buffer(version_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send_pid()
        .send()
        .map(|_| ())
}

/// Finalize.
pub(crate) fn finalize(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    object.dispatch(proto::MF_FINALIZE).send().map(|_| ())
}

/// ListDevices.
pub(crate) fn list_devices(
    object: &DomainObject<'_>,
    out: &mut [NfcDeviceHandle],
) -> Result<i32, DispatchError> {
    // SAFETY: `out` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound, and the byte slice borrows `out`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let result = object
        .dispatch(proto::MF_LIST_DEVICES)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_POINTER)
        .send()?;

    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// StartDetection.
pub(crate) fn start_detection(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::MF_START_DETECTION, *handle)
}

/// StopDetection.
pub(crate) fn stop_detection(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::MF_STOP_DETECTION, *handle)
}

/// ReadMifare.
pub(crate) fn read_mifare(
    object: &DomainObject<'_>,
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
    object
        .dispatch(proto::MF_READ_MIFARE)
        .in_raw(in_bytes)
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(param_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// WriteMifare.
pub(crate) fn write_mifare(
    object: &DomainObject<'_>,
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
    object
        .dispatch(proto::MF_WRITE_MIFARE)
        .in_raw(in_bytes)
        .in_buffer(param_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// GetTagInfo.
pub(crate) fn get_tag_info(
    object: &DomainObject<'_>,
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
    object
        .dispatch(proto::MF_GET_TAG_INFO)
        .in_raw(in_bytes)
        .out_buffer(
            out_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send()
        .map(|_| ())
}

/// AttachActivateEvent.
pub(crate) fn attach_activate_event(
    object: &DomainObject<'_>,
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
    let result = object
        .dispatch(proto::MF_ATTACH_ACTIVATE_EVENT)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;
    Ok(result.copy_handles[0])
}

/// AttachDeactivateEvent.
pub(crate) fn attach_deactivate_event(
    object: &DomainObject<'_>,
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
    let result = object
        .dispatch(proto::MF_ATTACH_DEACTIVATE_EVENT)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;
    Ok(result.copy_handles[0])
}

/// AttachAvailabilityChangeEvent.
pub(crate) fn attach_availability_change_event(
    object: &DomainObject<'_>,
) -> Result<u32, DispatchError> {
    let result = object
        .dispatch(proto::MF_ATTACH_AVAILABILITY_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;
    Ok(result.copy_handles[0])
}

/// GetState.
pub(crate) fn get_state(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::MF_GET_STATE)
}

/// GetDeviceState.
pub(crate) fn get_device_state(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::MF_GET_DEVICE_STATE, *handle)
}

/// GetNpadId.
pub(crate) fn get_npad_id(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::MF_GET_NPAD_ID, *handle)
}
