//! CMIF protocol operations for the NFC interface.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Domain, DomainObject, OutHandleAttr};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_out},
    proto,
    types::{
        InitializeIn, NfcDeviceHandle, NfcMifareReadBlockData, NfcMifareReadBlockParameter,
        NfcMifareWriteBlockParameter, NfcRequiredMcuVersionData, NfcStartDetectionIn, NfcTagInfo,
        SendCommandByPassThroughIn,
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

/// Initialize — pre-4.0.0 command ID layout.
pub(crate) fn initialize_legacy(
    object: &DomainObject<'_>,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
) -> Result<(), DispatchError> {
    initialize_impl(object, aruid, version_data, proto::NFC_INITIALIZE_LEGACY)
}

/// Initialize — 4.0.0+ command ID layout.
pub(crate) fn initialize(
    object: &DomainObject<'_>,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
) -> Result<(), DispatchError> {
    initialize_impl(object, aruid, version_data, proto::NFC_INITIALIZE)
}

fn initialize_impl(
    object: &DomainObject<'_>,
    aruid: u64,
    version_data: &[NfcRequiredMcuVersionData],
    cmd_id: u32,
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
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .in_buffer(version_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send_pid()
        .send()
        .map(|_| ())
}

/// Finalize — pre-4.0.0 command ID layout.
pub(crate) fn finalize_legacy(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    object
        .dispatch(proto::NFC_FINALIZE_LEGACY)
        .send()
        .map(|_| ())
}

/// Finalize — 4.0.0+ command ID layout.
pub(crate) fn finalize(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    object.dispatch(proto::NFC_FINALIZE).send().map(|_| ())
}

/// GetState (pre-4.0.0).
pub(crate) fn get_state_legacy(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::NFC_GET_STATE_LEGACY)
}

/// IsNfcEnabled (pre-4.0.0).
pub(crate) fn is_nfc_enabled_legacy(object: &DomainObject<'_>) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(object, proto::NFC_IS_NFC_ENABLED_LEGACY)?;
    Ok(val & 1 != 0)
}

/// GetState (4.0.0+).
pub(crate) fn get_state(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out(object, proto::NFC_GET_STATE)
}

/// IsNfcEnabled (4.0.0+).
pub(crate) fn is_nfc_enabled(object: &DomainObject<'_>) -> Result<bool, DispatchError> {
    let val: u8 = dispatch_out(object, proto::NFC_IS_NFC_ENABLED)?;
    Ok(val & 1 != 0)
}

/// ListDevices (4.0.0+).
pub(crate) fn list_devices(
    object: &DomainObject<'_>,
    out: &mut [NfcDeviceHandle],
) -> Result<i32, DispatchError> {
    // SAFETY: `out` is a valid `&mut` slice; viewing it as bytes for the
    // OUT buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(out.as_mut_ptr().cast::<u8>(), core::mem::size_of_val(out))
    };
    let result = object
        .dispatch(proto::NFC_LIST_DEVICES)
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

/// GetDeviceState (4.0.0+).
pub(crate) fn get_device_state(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFC_GET_DEVICE_STATE, *handle)
}

/// GetNpadId (4.0.0+).
pub(crate) fn get_npad_id(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<u32, DispatchError> {
    dispatch_in_out(object, proto::NFC_GET_NPAD_ID, *handle)
}

/// StartDetection (4.0.0+ — device handle + protocol).
pub(crate) fn start_detection(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    protocol: u32,
) -> Result<(), DispatchError> {
    let input = NfcStartDetectionIn {
        handle: *handle,
        protocol,
    };
    dispatch_in(object, proto::NFC_START_DETECTION, input)
}

/// StopDetection (4.0.0+).
pub(crate) fn stop_detection(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFC_STOP_DETECTION, *handle)
}

/// GetTagInfo (4.0.0+).
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
        .dispatch(proto::NFC_GET_TAG_INFO)
        .in_raw(in_bytes)
        .out_buffer(
            out_bytes,
            BufferAttr::FIXED_SIZE.or(BufferAttr::HIPC_POINTER),
        )
        .send()
        .map(|_| ())
}

/// AttachActivateEvent (4.0.0+).
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
        .dispatch(proto::NFC_ATTACH_ACTIVATE_EVENT)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;
    Ok(result.copy_handles[0])
}

/// AttachDeactivateEvent (4.0.0+).
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
        .dispatch(proto::NFC_ATTACH_DEACTIVATE_EVENT)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;
    Ok(result.copy_handles[0])
}

/// AttachAvailabilityChangeEvent (4.0.0+).
pub(crate) fn attach_availability_change_event(
    object: &DomainObject<'_>,
) -> Result<u32, DispatchError> {
    let result = object
        .dispatch(proto::NFC_ATTACH_AVAILABILITY_CHANGE_EVENT)
        .out_handle(0, OutHandleAttr::Copy)
        .send()?;
    Ok(result.copy_handles[0])
}

/// ReadMifare (4.0.0+).
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
        .dispatch(proto::NFC_READ_MIFARE)
        .in_raw(in_bytes)
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(param_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// WriteMifare (4.0.0+).
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
        .dispatch(proto::NFC_WRITE_MIFARE)
        .in_raw(in_bytes)
        .in_buffer(param_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// SendCommandByPassThrough (4.0.0+).
pub(crate) fn send_command_by_pass_through(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
    timeout: u64,
    cmd_buf: &[u8],
    reply_buf: &mut [u8],
) -> Result<u32, DispatchError> {
    let input = SendCommandByPassThroughIn {
        handle: *handle,
        timeout,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<SendCommandByPassThroughIn>(),
        )
    };
    let result = object
        .dispatch(proto::NFC_SEND_COMMAND_BY_PASS_THROUGH)
        .in_raw(in_bytes)
        .out_size(size_of::<u32>())
        .out_buffer(reply_buf, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(cmd_buf, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;

    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// KeepPassThroughSession (4.0.0+).
pub(crate) fn keep_pass_through_session(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFC_KEEP_PASS_THROUGH_SESSION, *handle)
}

/// ReleasePassThroughSession (4.0.0+).
pub(crate) fn release_pass_through_session(
    object: &DomainObject<'_>,
    handle: &NfcDeviceHandle,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::NFC_RELEASE_PASS_THROUGH_SESSION, *handle)
}
