//! `pm:shell` FFI.

use nx_service_pm::{NcmProgramLocation, ProcessEventInfo, ProcessId, ProgramId};
use nx_sf::{error::ToResultCode as _, ffi::Service};
use nx_svc::raw::INVALID_HANDLE;

use super::{
    common::{
        GENERIC_ERROR, LibnxEvent, RC_INCOMPAT_SYSVER, hosversion_at_least, hosversion_before,
        hosversionIsAtmosphere,
    },
    state::{SHELL, SHELL_SRV, clear_shadow, ensure_sm, write_shadow},
};

/// `pmshellInitialize()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_initialize() -> u32 {
    {
        let guard = SHELL.read();
        if guard.is_some() {
            return 0;
        }
    }

    if let Err(err) = ensure_sm() {
        return err.to_rc();
    }

    let sm_guard = super::state::SM.read();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let svc = match nx_service_pm::connect_shell_cmif(sm) {
        Ok(s) => s,
        Err(err) => return err.to_rc(),
    };

    let mut guard = SHELL.write();
    if guard.is_some() {
        return 0;
    }
    write_shadow(&SHELL_SRV, svc.session());
    *guard = Some(svc);
    0
}

/// `pmshellExit()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_exit() {
    let mut guard = SHELL.write();
    if guard.take().is_some() {
        clear_shadow(&SHELL_SRV);
    }
}

/// `pmshellGetServiceSession()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_get_service_session() -> *mut Service {
    SHELL_SRV.get().cast::<Service>()
}

/// `pmshellLaunchProgram(u32 launch_flags, const NcmProgramLocation *location, u64 *pid)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_launch_program(
    launch_flags: u32,
    location: *const NcmProgramLocation,
    pid: *mut u64,
) -> u32 {
    if location.is_null() || pid.is_null() {
        return GENERIC_ERROR;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    // SAFETY: caller guarantees `location` points to a valid NcmProgramLocation.
    let loc = unsafe { &*location };
    match svc.launch_program(launch_flags, loc) {
        Ok(new_pid) => {
            // SAFETY: caller guarantees `pid` is writable.
            unsafe { *pid = new_pid.to_u64() };
            0
        }
        Err(e) => e.to_rc(),
    }
}

/// `pmshellTerminateProcess(u64 processID)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_terminate_process(process_id: u64) -> u32 {
    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.terminate_process(unsafe { ProcessId::new_unchecked(process_id) }) {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellTerminateProgram(u64 program_id)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_terminate_program(program_id: u64) -> u32 {
    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.terminate_program(unsafe { ProgramId::new_unchecked(program_id) }) {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellGetProcessEventHandle(Event *out)` — autoclear is always true.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_get_process_event_handle(out: *mut LibnxEvent) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_process_event_handle() {
        Ok(handle) => {
            // SAFETY: caller guarantees `out` is writable.
            unsafe {
                (*out).revent = handle;
                (*out).wevent = INVALID_HANDLE;
                (*out).autoclear = true;
            }
            0
        }
        Err(e) => e.to_rc(),
    }
}

/// `pmshellGetProcessEventInfo(PmProcessEventInfo *out)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_get_process_event_info(
    out: *mut ProcessEventInfo,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_process_event_info() {
        Ok(info) => {
            // SAFETY: caller guarantees `out` is writable; layout matches
            // libnx's `PmProcessEventInfo` (asserted in `nx-service-pm`).
            unsafe { *out = info };
            0
        }
        Err(e) => e.to_rc(),
    }
}

/// `pmshellCleanupProcess(u64 pid)` — pre-5.0.0 only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_cleanup_process(pid: u64) -> u32 {
    if hosversion_at_least(5, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.cleanup_process(unsafe { ProcessId::new_unchecked(pid) }) {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellClearJitDebugOccured(u64 pid)` — pre-5.0.0 only (note libnx spelling).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_clear_jit_debug_occured(pid: u64) -> u32 {
    if hosversion_at_least(5, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.clear_jit_debug_occurred(unsafe { ProcessId::new_unchecked(pid) }) {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellNotifyBootFinished()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_notify_boot_finished() -> u32 {
    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let result = if hosversion_at_least(5, 0, 0) {
        svc.notify_boot_finished()
    } else {
        svc.notify_boot_finished_legacy()
    };
    match result {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellGetApplicationProcessIdForShell(u64 *pid_out)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_get_application_process_id_for_shell(
    pid_out: *mut u64,
) -> u32 {
    if pid_out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let result = if hosversion_at_least(5, 0, 0) {
        svc.get_application_process_id_for_shell()
    } else {
        svc.get_application_process_id_for_shell_legacy()
    };
    match result {
        Ok(pid) => {
            // SAFETY: caller guarantees `pid_out` is writable.
            unsafe { *pid_out = pid.to_u64() };
            0
        }
        Err(e) => e.to_rc(),
    }
}

/// `pmshellBoostSystemMemoryResourceLimit(u64 boost_size)` — `[4.0.0+]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_boost_system_memory_resource_limit(
    boost_size: u64,
) -> u32 {
    if hosversion_before(4, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let result = if hosversion_at_least(5, 0, 0) {
        svc.boost_system_memory_resource_limit(boost_size)
    } else {
        svc.boost_system_memory_resource_limit_legacy(boost_size)
    };
    match result {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellBoostApplicationThreadResourceLimit()` — `[7.0.0+/Atmosphere]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_boost_application_thread_resource_limit() -> u32 {
    if !hosversionIsAtmosphere() && hosversion_before(7, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.boost_application_thread_resource_limit() {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellBoostSystemThreadResourceLimit()` — `[14.0.0+/Atmosphere]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_boost_system_thread_resource_limit() -> u32 {
    if !hosversionIsAtmosphere() && hosversion_before(14, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.boost_system_thread_resource_limit() {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}

/// `pmshellGetProcessId(u64 *pid_out, u64 program_id)` — `[19.0.0+/Atmosphere]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmshell_get_process_id(
    pid_out: *mut u64,
    program_id: u64,
) -> u32 {
    if !hosversionIsAtmosphere() && hosversion_before(19, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }
    if pid_out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = SHELL.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_process_id(unsafe { ProgramId::new_unchecked(program_id) }) {
        Ok(pid) => {
            // SAFETY: caller guarantees `pid_out` is writable.
            unsafe { *pid_out = pid.to_u64() };
            0
        }
        Err(e) => e.to_rc(),
    }
}
