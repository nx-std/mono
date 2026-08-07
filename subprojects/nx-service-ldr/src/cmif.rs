//! CMIF protocol operations for the loader service.

use core::mem::size_of;

use nx_sf::service::{
    BufferAttr,
    DispatchError,
    OutHandleAttr,
    Session,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_no_io,
    },
    proto,
    types::{
        CreateProcessIn,
        CreateProcessLegacyIn,
        GetProgramInfoIn,
        LoaderModuleInfo,
        LoaderProgramAttributes,
        LoaderProgramInfo,
        LoaderProgramInfoV1,
        NcmProgramLocation,
        SetProgramArgumentsLegacyIn,
    },
};

/// Sets program arguments (modern, `[11.0.0+]`).
///
/// Wire input: `u64 program_id` + HipcPointer in-buffer.
pub(crate) fn set_program_arguments(
    service: &Session,
    program_id: u64,
    args: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SET_PROGRAM_ARGUMENTS)
        .in_raw(program_id.as_bytes())
        .in_buffer(args, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets program arguments (legacy, pre-11.0.0).
///
/// Wire input: `{ u32 args_size, u32 pad, u64 program_id }` + HipcPointer in-buffer.
pub(crate) fn set_program_arguments_legacy(
    service: &Session,
    program_id: u64,
    args: &[u8],
) -> Result<(), DispatchError> {
    let input = SetProgramArgumentsLegacyIn {
        args_size: args.len() as u32,
        pad: 0,
        program_id,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::SET_PROGRAM_ARGUMENTS)
        .in_raw(input.as_bytes())
        .in_buffer(args, BufferAttr::HIPC_POINTER)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Flushes all program arguments.
pub(crate) fn flush_arguments(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, proto::FLUSH_ARGUMENTS)
}

/// Gets module information for a process via `ldr:dmnt`.
pub(crate) fn get_process_module_info(
    service: &Session,
    pid: u64,
    out_modules: &mut [LoaderModuleInfo],
) -> Result<i32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::DMNT_GET_PROCESS_MODULE_INFO)
        .in_raw(pid.as_bytes())
        .out_buffer(out_modules.as_mut_bytes(), BufferAttr::HIPC_POINTER)
        .out_size(size_of::<i32>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<i32>())
}

/// Creates a process (legacy, pre-20.0.0/non-Atmosphere).
///
/// Wire input: `{ u32 flags, u32 pad, u64 pin_id }` + copy handle in (reslimit).
/// Returns a move handle (process).
pub(crate) fn create_process_legacy(
    service: &Session,
    pin_id: u64,
    flags: u32,
    reslimit_handle: u32,
) -> Result<u32, DispatchError> {
    let input = CreateProcessLegacyIn {
        flags,
        pad: 0,
        pin_id,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::PM_CREATE_PROCESS)
        .in_raw(input.as_bytes())
        .in_handle(reslimit_handle)
        .out_handle(0, OutHandleAttr::Move)
        .send(&mut ipc_buf)?;

    Ok(result.move_handles[0])
}

/// Creates a process (`[20.0.0+/Atmosphere]`).
///
/// Wire input: `{ LoaderProgramAttributes attr, u16 pad, u32 flags, u64 pin_id }`
/// + copy handle in (reslimit). Returns a move handle (process).
pub(crate) fn create_process(
    service: &Session,
    pin_id: u64,
    flags: u32,
    reslimit_handle: u32,
    attrs: &LoaderProgramAttributes,
) -> Result<u32, DispatchError> {
    let input = CreateProcessIn {
        attr: *attrs,
        pad: 0,
        flags,
        pin_id,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::PM_CREATE_PROCESS)
        .in_raw(input.as_bytes())
        .in_handle(reslimit_handle)
        .out_handle(0, OutHandleAttr::Move)
        .send(&mut ipc_buf)?;

    Ok(result.move_handles[0])
}

/// Gets program info (legacy, `[1.0.0–18.1.0]`, non-Atmosphere).
///
/// Wire input: `NcmProgramLocation`. Out: HipcPointer fixed-size buffer.
pub(crate) fn get_program_info_v1(
    service: &Session,
    loc: &NcmProgramLocation,
    out: &mut LoaderProgramInfoV1,
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::PM_GET_PROGRAM_INFO)
        .in_raw(loc.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets program info (`[19.0.0+/Atmosphere]`).
///
/// Wire input: `GetProgramInfoIn`. Out: HipcPointer fixed-size buffer.
pub(crate) fn get_program_info(
    service: &Session,
    loc: &NcmProgramLocation,
    attrs: &LoaderProgramAttributes,
    out: &mut LoaderProgramInfo,
) -> Result<(), DispatchError> {
    let input = GetProgramInfoIn {
        attr: *attrs,
        pad1: 0,
        pad2: 0,
        loc: *loc,
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    service
        .dispatch(proto::PM_GET_PROGRAM_INFO)
        .in_raw(input.as_bytes())
        .out_buffer(
            out.as_mut_bytes(),
            BufferAttr::HIPC_POINTER.or(BufferAttr::FIXED_SIZE),
        )
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Pins a program, returning a pin ID.
pub(crate) fn pin_program(
    service: &Session,
    loc: &NcmProgramLocation,
) -> Result<u64, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = service
        .dispatch(proto::PM_PIN_PROGRAM)
        .in_raw(loc.as_bytes())
        .out_size(size_of::<u64>())
        .send(&mut ipc_buf)?;

    Ok(*result.value::<u64>())
}

/// Unpins a previously pinned program.
pub(crate) fn unpin_program(service: &Session, pin_id: u64) -> Result<(), DispatchError> {
    dispatch_in(service, proto::PM_UNPIN_PROGRAM, pin_id)
}

/// Enables or disables program verification (`[10.0.0+]`).
pub(crate) fn set_enabled_program_verification(
    service: &Session,
    enabled: bool,
) -> Result<(), DispatchError> {
    let raw: u8 = enabled as u8;
    dispatch_in(service, proto::PM_SET_ENABLED_PROGRAM_VERIFICATION, raw)
}
