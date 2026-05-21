//! CMIF protocol operations for the process manager service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, OutHandleAttr, Session};
use static_assertions::const_assert_eq;

use super::{
    dispatch::{dispatch_in, dispatch_in_out, dispatch_no_io, dispatch_out},
    pm_bm::{BootMode, proto as bm_proto},
    pm_dmnt::proto as dmnt_proto,
    pm_info::{ResourceLimitValues, proto as info_proto},
    pm_shell::{NcmProgramLocation, ProcessEventInfo, proto as shell_proto},
    types::{ProcessId, ProgramId},
};

/// Input for `pm:shell` `LaunchProgram`.
///
/// Wire layout: `{ u32 launch_flags, u32 pad, NcmProgramLocation }`.
#[repr(C)]
#[derive(Clone, Copy)]
struct LaunchProgramIn {
    launch_flags: u32,
    pad: u32,
    location: NcmProgramLocation,
}

const_assert_eq!(size_of::<LaunchProgramIn>(), 0x18);

/// Gets the current boot mode.
pub(crate) fn get_boot_mode(service: &Session) -> Result<BootMode, DispatchError> {
    dispatch_out(service, bm_proto::GET_BOOT_MODE)
}

/// Sets boot mode to maintenance.
pub(crate) fn set_maintenance_boot(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, bm_proto::SET_MAINTENANCE_BOOT)
}

/// Gets the JIT debug process ID list.
///
/// Wire: out `u32 count` + HipcMapAlias out buffer of `u64` PIDs.
pub(crate) fn get_jit_debug_process_id_list(
    service: &Session,
    cmd_id: u32,
    out_pids: &mut [ProcessId],
) -> Result<u32, DispatchError> {
    // SAFETY: `out_pids` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `out_pids`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            out_pids.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(out_pids),
        )
    };
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .out_size(size_of::<u32>())
        .send(&mut buf)?;

    // SAFETY: response payload is at least size_of::<u32>() bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u32>()) })
}

/// Starts a process by PID.
pub(crate) fn start_process(
    service: &Session,
    cmd_id: u32,
    pid: ProcessId,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, pid)
}

/// Gets a process ID from a program ID.
pub(crate) fn get_process_id(
    service: &Session,
    cmd_id: u32,
    program_id: ProgramId,
) -> Result<ProcessId, DispatchError> {
    dispatch_in_out(service, cmd_id, program_id)
}

/// Hooks to be notified when a specific program creates a process.
///
/// Returns a copy-handle for the event.
pub(crate) fn hook_to_create_process(
    service: &Session,
    cmd_id: u32,
    program_id: ProgramId,
) -> Result<u32, DispatchError> {
    // SAFETY: `program_id` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const program_id).cast::<u8>(), size_of::<ProgramId>())
    };
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .in_raw(in_bytes)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)?;

    Ok(result.copy_handles[0])
}

/// Gets the application process ID.
pub(crate) fn get_application_process_id(
    service: &Session,
    cmd_id: u32,
) -> Result<ProcessId, DispatchError> {
    dispatch_out(service, cmd_id)
}

/// Hooks to be notified when the application process is created.
///
/// Returns a copy-handle for the event.
pub(crate) fn hook_to_create_application_process(
    service: &Session,
    cmd_id: u32,
) -> Result<u32, DispatchError> {
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(cmd_id)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)?;

    Ok(result.copy_handles[0])
}

/// Clears a hook.
///
/// `[6.0.0+]`
pub(crate) fn clear_hook(service: &Session, which: u32) -> Result<(), DispatchError> {
    dispatch_in(service, dmnt_proto::CLEAR_HOOK, which)
}

/// Gets a program ID from a PID.
///
/// `[14.0.0+/Atmosphere]`
pub(crate) fn dmnt_get_program_id(
    service: &Session,
    pid: ProcessId,
) -> Result<ProgramId, DispatchError> {
    dispatch_in_out(service, dmnt_proto::GET_PROGRAM_ID, pid)
}

/// Gets a program ID from a PID.
pub(crate) fn info_get_program_id(
    service: &Session,
    pid: ProcessId,
) -> Result<ProgramId, DispatchError> {
    dispatch_in_out(service, info_proto::GET_PROGRAM_ID, pid)
}

/// Gets applet current resource limit values.
///
/// `[14.0.0+/Atmosphere]`
pub(crate) fn get_applet_current_resource_limit_values(
    service: &Session,
) -> Result<ResourceLimitValues, DispatchError> {
    dispatch_out(
        service,
        info_proto::GET_APPLET_CURRENT_RESOURCE_LIMIT_VALUES,
    )
}

/// Gets applet peak resource limit values.
///
/// `[14.0.0+/Atmosphere]`
pub(crate) fn get_applet_peak_resource_limit_values(
    service: &Session,
) -> Result<ResourceLimitValues, DispatchError> {
    dispatch_out(service, info_proto::GET_APPLET_PEAK_RESOURCE_LIMIT_VALUES)
}

/// Launches a program.
///
/// Wire input: `{ u32 launch_flags, u32 pad, NcmProgramLocation }`.
/// Returns the launched process ID.
pub(crate) fn launch_program(
    service: &Session,
    launch_flags: u32,
    location: &NcmProgramLocation,
) -> Result<ProcessId, DispatchError> {
    let input = LaunchProgramIn {
        launch_flags,
        pad: 0,
        location: *location,
    };

    dispatch_in_out(service, shell_proto::LAUNCH_PROGRAM, input)
}

/// Terminates a process by PID.
pub(crate) fn terminate_process(service: &Session, pid: ProcessId) -> Result<(), DispatchError> {
    dispatch_in(service, shell_proto::TERMINATE_PROCESS, pid)
}

/// Terminates a program by program ID.
pub(crate) fn terminate_program(
    service: &Session,
    program_id: ProgramId,
) -> Result<(), DispatchError> {
    dispatch_in(service, shell_proto::TERMINATE_PROGRAM, program_id)
}

/// Gets the process event handle.
///
/// Returns a copy-handle for the event.
pub(crate) fn get_process_event_handle(service: &Session) -> Result<u32, DispatchError> {
    // SAFETY: one IpcBuffer token per thread; IPC is serialized per thread.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let result = service
        .dispatch(shell_proto::GET_PROCESS_EVENT_HANDLE)
        .out_handle(0, OutHandleAttr::Copy)
        .send(&mut buf)?;

    Ok(result.copy_handles[0])
}

/// Gets the process event info.
pub(crate) fn get_process_event_info(service: &Session) -> Result<ProcessEventInfo, DispatchError> {
    dispatch_out(service, shell_proto::GET_PROCESS_EVENT_INFO)
}

/// Cleans up a process (pre-5.0.0 only).
pub(crate) fn cleanup_process(service: &Session, pid: ProcessId) -> Result<(), DispatchError> {
    dispatch_in(service, shell_proto::CLEANUP_PROCESS_LEGACY, pid)
}

/// Clears JIT debug occurred flag (pre-5.0.0 only).
pub(crate) fn clear_jit_debug_occurred(
    service: &Session,
    pid: ProcessId,
) -> Result<(), DispatchError> {
    dispatch_in(service, shell_proto::CLEAR_JIT_DEBUG_OCCURRED_LEGACY, pid)
}

/// Notifies the system that boot has finished.
pub(crate) fn notify_boot_finished(service: &Session, cmd_id: u32) -> Result<(), DispatchError> {
    dispatch_no_io(service, cmd_id)
}

/// Gets the application process ID for shell.
pub(crate) fn get_application_process_id_for_shell(
    service: &Session,
    cmd_id: u32,
) -> Result<ProcessId, DispatchError> {
    dispatch_out(service, cmd_id)
}

/// Boosts system memory resource limit.
///
/// `[4.0.0+]`
pub(crate) fn boost_system_memory_resource_limit(
    service: &Session,
    cmd_id: u32,
    boost_size: u64,
) -> Result<(), DispatchError> {
    dispatch_in(service, cmd_id, boost_size)
}

/// Boosts application thread resource limit.
///
/// `[7.0.0+/Atmosphere]`
pub(crate) fn boost_application_thread_resource_limit(
    service: &Session,
) -> Result<(), DispatchError> {
    dispatch_no_io(
        service,
        shell_proto::BOOST_APPLICATION_THREAD_RESOURCE_LIMIT,
    )
}

/// Boosts system thread resource limit.
///
/// `[14.0.0+/Atmosphere]`
pub(crate) fn boost_system_thread_resource_limit(service: &Session) -> Result<(), DispatchError> {
    dispatch_no_io(service, shell_proto::BOOST_SYSTEM_THREAD_RESOURCE_LIMIT)
}

/// Gets a process ID from a program ID.
///
/// `[19.0.0+/Atmosphere]`
pub(crate) fn shell_get_process_id(
    service: &Session,
    program_id: ProgramId,
) -> Result<ProcessId, DispatchError> {
    dispatch_in_out(service, shell_proto::GET_PROCESS_ID, program_id)
}
