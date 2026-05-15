//! CMIF protocol operations for the IRS service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};

use crate::{
    dispatch::{dispatch_in_out, dispatch_in_pid_no_out},
    proto,
    types::{
        ActivateWithFunctionLevelIn, CheckFirmwareVersionIn, GetImageTransferProcessorStateIn,
        HandleAruidIn, ImageTransferProcessorState, IrCameraHandle,
        PackedClusteringProcessorConfig, PackedFunctionLevel, PackedImageTransferProcessorConfig,
        PackedImageTransferProcessorExConfig, PackedIrLedProcessorConfig, PackedMcuVersion,
        PackedMomentProcessorConfig, PackedPointingProcessorConfig,
        PackedTeraPluginProcessorConfig, RunClusteringProcessorIn, RunImageTransferExProcessorIn,
        RunImageTransferProcessorIn, RunIrLedProcessorIn, RunMomentProcessorIn,
        RunPointingProcessorIn, RunTeraPluginProcessorIn,
    },
};

// ---------------------------------------------------------------------------
// Activation / deactivation
// ---------------------------------------------------------------------------

/// ActivateIrsensor (cmd 302).
pub(crate) fn activate_irsensor(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in_pid_no_out(service, proto::ACTIVATE_IRSENSOR, &applet_resource_user_id)
}

/// DeactivateIrsensor (cmd 303).
pub(crate) fn deactivate_irsensor(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    dispatch_in_pid_no_out(
        service,
        proto::DEACTIVATE_IRSENSOR,
        &applet_resource_user_id,
    )
}

/// GetIrsensorSharedMemoryHandle (cmd 304). Returns a copy handle for the
/// shared memory region.
pub(crate) fn get_irsensor_shared_memory_handle(
    service: &Session,
    applet_resource_user_id: u64,
) -> Result<u32, GetSharedMemoryError> {
    // SAFETY: `applet_resource_user_id` is a `Copy` value on the stack, valid
    // until `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const applet_resource_user_id).cast::<u8>(),
            size_of::<u64>(),
        )
    };
    let result = service
        .dispatch(proto::GET_IRSENSOR_SHARED_MEMORY_HANDLE)
        .in_raw(in_bytes)
        .send_pid()
        .out_handle(0, OutHandleAttr::Copy)
        .send()
        .map_err(GetSharedMemoryError::Dispatch)?;

    result
        .copy_handles
        .first()
        .copied()
        .ok_or(GetSharedMemoryError::MissingHandle)
}

/// Error returned by [`get_irsensor_shared_memory_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryError {
    #[error("dispatch failed")]
    Dispatch(#[source] DispatchError),
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

// ---------------------------------------------------------------------------
// Processor control
// ---------------------------------------------------------------------------

/// StopImageProcessor (cmd 305).
pub(crate) fn stop_image_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = HandleAruidIn {
        handle,
        pad: 0,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::STOP_IMAGE_PROCESSOR, &input)
}

/// RunMomentProcessor (cmd 306).
pub(crate) fn run_moment_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    config: &PackedMomentProcessorConfig,
) -> Result<(), DispatchError> {
    let input = RunMomentProcessorIn {
        handle,
        pad: 0,
        applet_resource_user_id,
        config: *config,
    };
    dispatch_in_pid_no_out(service, proto::RUN_MOMENT_PROCESSOR, &input)
}

/// RunClusteringProcessor (cmd 307).
pub(crate) fn run_clustering_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    config: &PackedClusteringProcessorConfig,
) -> Result<(), DispatchError> {
    let input = RunClusteringProcessorIn {
        handle,
        pad: 0,
        applet_resource_user_id,
        config: *config,
    };
    dispatch_in_pid_no_out(service, proto::RUN_CLUSTERING_PROCESSOR, &input)
}

/// RunImageTransferProcessor (cmd 308). Takes a transfer memory copy handle.
pub(crate) fn run_image_transfer_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    config: &PackedImageTransferProcessorConfig,
    transfer_memory_size: u64,
    tmem_handle: u32,
) -> Result<(), DispatchError> {
    let input = RunImageTransferProcessorIn {
        handle,
        pad: 0,
        applet_resource_user_id,
        config: *config,
        transfer_memory_size,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its `size_of::<T>()` bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<RunImageTransferProcessorIn>(),
        )
    };
    service
        .dispatch(proto::RUN_IMAGE_TRANSFER_PROCESSOR)
        .in_raw(in_bytes)
        .send_pid()
        .in_handle(tmem_handle)
        .send()
        .map(|_| ())
}

/// GetImageTransferProcessorState (cmd 309). Returns state and fills the
/// output buffer via HipcMapAlias.
pub(crate) fn get_image_transfer_processor_state(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    buffer: &mut [u8],
) -> Result<ImageTransferProcessorState, DispatchError> {
    let input = GetImageTransferProcessorStateIn {
        handle,
        pad: 0,
        applet_resource_user_id,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<GetImageTransferProcessorStateIn>(),
        )
    };
    let result = service
        .dispatch(proto::GET_IMAGE_TRANSFER_PROCESSOR_STATE)
        .in_raw(in_bytes)
        .send_pid()
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<ImageTransferProcessorState>())
        .send()?;

    // SAFETY: response payload is at least `size_of::<ImageTransferProcessorState>()`.
    let state = unsafe {
        core::ptr::read_unaligned(result.data.as_ptr().cast::<ImageTransferProcessorState>())
    };

    Ok(state)
}

/// RunTeraPluginProcessor (cmd 310).
pub(crate) fn run_tera_plugin_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    config: &PackedTeraPluginProcessorConfig,
) -> Result<(), DispatchError> {
    let input = RunTeraPluginProcessorIn {
        handle,
        config: *config,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::RUN_TERA_PLUGIN_PROCESSOR, &input)
}

/// GetIrCameraHandle (cmd 311). No PID.
pub(crate) fn get_ir_camera_handle(
    service: &Session,
    npad_id: u32,
) -> Result<IrCameraHandle, DispatchError> {
    dispatch_in_out(service, proto::GET_IR_CAMERA_HANDLE, &npad_id)
}

/// RunPointingProcessor (cmd 312).
pub(crate) fn run_pointing_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    config: &PackedPointingProcessorConfig,
) -> Result<(), DispatchError> {
    let input = RunPointingProcessorIn {
        handle,
        config: *config,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::RUN_POINTING_PROCESSOR, &input)
}

/// SuspendImageProcessor (cmd 313).
pub(crate) fn suspend_image_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = HandleAruidIn {
        handle,
        pad: 0,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::SUSPEND_IMAGE_PROCESSOR, &input)
}

/// CheckFirmwareVersion (cmd 314). \[3.0.0+\]
pub(crate) fn check_firmware_version(
    service: &Session,
    handle: IrCameraHandle,
    version: PackedMcuVersion,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = CheckFirmwareVersionIn {
        handle,
        version,
        pad: 0,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::CHECK_FIRMWARE_VERSION, &input)
}

/// RunImageTransferExProcessor (cmd 316). \[4.0.0+\] Takes a transfer memory
/// copy handle.
pub(crate) fn run_image_transfer_ex_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    config: &PackedImageTransferProcessorExConfig,
    transfer_memory_size: u64,
    tmem_handle: u32,
) -> Result<(), DispatchError> {
    let input = RunImageTransferExProcessorIn {
        handle,
        pad: 0,
        applet_resource_user_id,
        config: *config,
        transfer_memory_size,
    };

    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<RunImageTransferExProcessorIn>(),
        )
    };
    service
        .dispatch(proto::RUN_IMAGE_TRANSFER_EX_PROCESSOR)
        .in_raw(in_bytes)
        .send_pid()
        .in_handle(tmem_handle)
        .send()
        .map(|_| ())
}

/// RunIrLedProcessor (cmd 317). \[4.0.0+\]
pub(crate) fn run_ir_led_processor(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
    config: &PackedIrLedProcessorConfig,
) -> Result<(), DispatchError> {
    let input = RunIrLedProcessorIn {
        handle,
        config: *config,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::RUN_IR_LED_PROCESSOR, &input)
}

/// StopImageProcessorAsync (cmd 318). \[4.0.0+\]
pub(crate) fn stop_image_processor_async(
    service: &Session,
    handle: IrCameraHandle,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = HandleAruidIn {
        handle,
        pad: 0,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(service, proto::STOP_IMAGE_PROCESSOR_ASYNC, &input)
}

/// ActivateIrsensorWithFunctionLevel (cmd 319). \[4.0.0+\]
pub(crate) fn activate_irsensor_with_function_level(
    service: &Session,
    level: PackedFunctionLevel,
    applet_resource_user_id: u64,
) -> Result<(), DispatchError> {
    let input = ActivateWithFunctionLevelIn {
        level,
        pad: 0,
        applet_resource_user_id,
    };
    dispatch_in_pid_no_out(
        service,
        proto::ACTIVATE_IRSENSOR_WITH_FUNCTION_LEVEL,
        &input,
    )
}
