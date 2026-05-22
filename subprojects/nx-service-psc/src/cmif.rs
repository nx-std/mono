//! CMIF protocol operations for the power state controller service.

use core::{mem::size_of, ptr};

use nx_sf::service::{BufferAttr, DispatchError, Domain, DomainObject, OutHandleAttr};

use crate::{
    dispatch::{dispatch_in, dispatch_no_io},
    proto,
    types::GetRequestOut,
};

/// Gets a PM module sub-object from the root domain service.
pub(crate) fn get_pm_module<'d>(domain: &'d Domain) -> Result<DomainObject<'d>, GetPmModuleError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut result = domain
        .dispatch(proto::GET_PM_MODULE)
        .out_objects(1)
        .send(&mut ipc_buf)
        .map_err(GetPmModuleError::Dispatch)?;

    result.take_object(0).ok_or(GetPmModuleError::MissingObject)
}

/// Initializes a PM module sub-object.
///
/// Sends the module ID and dependency list, and receives an event copy handle.
pub(crate) fn module_initialize(
    object: &DomainObject<'_>,
    module_id: u32,
    dependencies: &[u32],
) -> Result<u32, ModuleInitializeError> {
    // SAFETY: `module_id` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its `size_of::<u32>()` bytes as a slice is
    // sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const module_id).cast::<u8>(), size_of::<u32>())
    };
    // SAFETY: `dependencies` is a valid `&` slice; viewing it as a byte slice
    // for the IN buffer is sound.
    let dep_bytes = unsafe {
        core::slice::from_raw_parts(
            dependencies.as_ptr().cast::<u8>(),
            core::mem::size_of_val(dependencies),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::MODULE_INITIALIZE)
        .in_raw(in_bytes)
        .in_buffer(dep_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut ipc_buf)
        .map_err(ModuleInitializeError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(ModuleInitializeError::MissingHandle);
    }
    Ok(result.copy_handles[0])
}

/// Gets the current PM request (state and flags).
pub(crate) fn module_get_request(
    object: &DomainObject<'_>,
) -> Result<GetRequestOut, DispatchError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = object
        .dispatch(proto::MODULE_GET_REQUEST)
        .out_size(size_of::<GetRequestOut>())
        .send(&mut ipc_buf)?;

    // SAFETY: response payload is at least size_of::<GetRequestOut>() bytes.
    Ok(unsafe { ptr::read_unaligned(result.data.as_ptr().cast::<GetRequestOut>()) })
}

/// Acknowledges a PM state transition (legacy, pre-5.1.0).
pub(crate) fn module_acknowledge_legacy(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::MODULE_ACKNOWLEDGE_LEGACY)
}

/// Acknowledges a PM state transition (5.1.0+).
pub(crate) fn module_acknowledge(
    object: &DomainObject<'_>,
    state: u32,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::MODULE_ACKNOWLEDGE, state)
}

/// Finalizes the PM module.
pub(crate) fn module_finalize(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::MODULE_FINALIZE)
}

/// Error returned by [`get_pm_module`].
#[derive(Debug, thiserror::Error)]
pub enum GetPmModuleError {
    /// IPC dispatch failed.
    #[error("failed to dispatch GetPmModule")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected domain sub-object.
    #[error("GetPmModule response did not include the expected sub-object")]
    MissingObject,
}

/// Error returned by [`module_initialize`].
#[derive(Debug, thiserror::Error)]
pub enum ModuleInitializeError {
    /// IPC dispatch failed.
    #[error("failed to dispatch IPmModule::Initialize")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected event copy handle.
    #[error("IPmModule::Initialize response did not include the expected event handle")]
    MissingHandle,
}
