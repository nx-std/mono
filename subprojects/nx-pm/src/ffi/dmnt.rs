//! `pm:dmnt` (debug/monitor) FFI.

use core::slice;

use nx_service_pm::{ProcessId, ProgramId};
use nx_sf::ffi::Service;
use nx_svc::raw::INVALID_HANDLE;

use super::{
    common::{
        GENERIC_ERROR, LibnxEvent, RC_INCOMPAT_SYSVER, connect_error_to_rc, dispatch_error_to_rc,
        hosversion_at_least, hosversion_before, hosversionIsAtmosphere, sm_connect_error_to_rc,
    },
    state::{DMNT, DMNT_SRV, clear_shadow, ensure_sm, write_shadow},
};

/// `pmdmntInitialize()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_initialize() -> u32 {
    {
        let guard = DMNT.read();
        if guard.is_some() {
            return 0;
        }
    }

    if let Err(err) = ensure_sm() {
        return sm_connect_error_to_rc(err);
    }

    let sm_guard = super::state::SM.read();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let svc = match nx_service_pm::connect_dmnt_cmif(sm) {
        Ok(s) => s,
        Err(e) => return connect_error_to_rc(e.0),
    };

    let mut guard = DMNT.write();
    if guard.is_some() {
        return 0;
    }
    write_shadow(&DMNT_SRV, svc.session());
    *guard = Some(svc);
    0
}

/// `pmdmntExit()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_exit() {
    let mut guard = DMNT.write();
    if guard.take().is_some() {
        clear_shadow(&DMNT_SRV);
    }
}

/// `pmdmntGetServiceSession()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_get_service_session() -> *mut Service {
    DMNT_SRV.get().cast::<Service>()
}

/// `pmdmntGetJitDebugProcessIdList(u32 *out_count, u64 *out_pids, size_t max_pids)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_get_jit_debug_process_id_list(
    out_count: *mut u32,
    out_pids: *mut u64,
    max_pids: usize,
) -> u32 {
    if out_count.is_null() || (out_pids.is_null() && max_pids != 0) {
        return GENERIC_ERROR;
    }

    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    // SAFETY: caller guarantees `out_pids` is writable for `max_pids` u64s;
    // `ProcessId` is `#[repr(transparent)]` over `u64`, so the cast is layout-safe.
    let pids = unsafe { slice::from_raw_parts_mut(out_pids.cast::<ProcessId>(), max_pids) };
    let result = if hosversion_at_least(5, 0, 0) {
        svc.get_jit_debug_process_id_list(pids)
    } else {
        svc.get_jit_debug_process_id_list_legacy(pids)
    };

    match result {
        Ok(count) => {
            // SAFETY: caller guarantees `out_count` is writable.
            unsafe { *out_count = count };
            0
        }
        Err(e) => dispatch_error_to_rc(e),
    }
}

/// `pmdmntStartProcess(u64 pid)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_start_process(pid: u64) -> u32 {
    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let pid = unsafe { ProcessId::new_unchecked(pid) };
    let result = if hosversion_at_least(5, 0, 0) {
        svc.start_process(pid)
    } else {
        svc.start_process_legacy(pid)
    };
    match result {
        Ok(()) => 0,
        Err(e) => dispatch_error_to_rc(e),
    }
}

/// `pmdmntGetProcessId(u64 *pid_out, u64 program_id)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_get_process_id(pid_out: *mut u64, program_id: u64) -> u32 {
    if pid_out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let program_id = unsafe { ProgramId::new_unchecked(program_id) };
    let result = if hosversion_at_least(5, 0, 0) {
        svc.get_process_id(program_id)
    } else {
        svc.get_process_id_legacy(program_id)
    };
    match result {
        Ok(pid) => {
            // SAFETY: caller guarantees `pid_out` is writable.
            unsafe { *pid_out = pid.to_u64() };
            0
        }
        Err(e) => dispatch_error_to_rc(e),
    }
}

/// `pmdmntHookToCreateProcess(Event *out, u64 program_id)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_hook_to_create_process(
    out: *mut LibnxEvent,
    program_id: u64,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let program_id = unsafe { ProgramId::new_unchecked(program_id) };
    let result = if hosversion_at_least(5, 0, 0) {
        svc.hook_to_create_process(program_id)
    } else {
        svc.hook_to_create_process_legacy(program_id)
    };
    match result {
        Ok(handle) => {
            // SAFETY: caller guarantees `out` is writable.
            unsafe {
                (*out).revent = handle;
                (*out).wevent = INVALID_HANDLE;
                (*out).autoclear = true;
            }
            0
        }
        Err(e) => dispatch_error_to_rc(e),
    }
}

/// `pmdmntGetApplicationProcessId(u64 *pid_out)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_get_application_process_id(pid_out: *mut u64) -> u32 {
    if pid_out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let result = if hosversion_at_least(5, 0, 0) {
        svc.get_application_process_id()
    } else {
        svc.get_application_process_id_legacy()
    };
    match result {
        Ok(pid) => {
            // SAFETY: caller guarantees `pid_out` is writable.
            unsafe { *pid_out = pid.to_u64() };
            0
        }
        Err(e) => dispatch_error_to_rc(e),
    }
}

/// `pmdmntHookToCreateApplicationProcess(Event *out)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_hook_to_create_application_process(
    out: *mut LibnxEvent,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    let result = if hosversion_at_least(5, 0, 0) {
        svc.hook_to_create_application_process()
    } else {
        svc.hook_to_create_application_process_legacy()
    };
    match result {
        Ok(handle) => {
            // SAFETY: caller guarantees `out` is writable.
            unsafe {
                (*out).revent = handle;
                (*out).wevent = INVALID_HANDLE;
                (*out).autoclear = true;
            }
            0
        }
        Err(e) => dispatch_error_to_rc(e),
    }
}

/// `pmdmntClearHook(u32 which)` — `[6.0.0+]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_clear_hook(which: u32) -> u32 {
    if hosversion_before(6, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }

    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.clear_hook(which) {
        Ok(()) => 0,
        Err(e) => dispatch_error_to_rc(e),
    }
}

/// `pmdmntGetProgramId(u64 *program_id_out, u64 pid)` — `[14.0.0+/Atmosphere]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmdmnt_get_program_id(program_id_out: *mut u64, pid: u64) -> u32 {
    if !hosversionIsAtmosphere() && hosversion_before(14, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }
    if program_id_out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = DMNT.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_program_id(unsafe { ProcessId::new_unchecked(pid) }) {
        Ok(program_id) => {
            // SAFETY: caller guarantees `program_id_out` is writable.
            unsafe { *program_id_out = program_id.to_u64() };
            0
        }
        Err(e) => dispatch_error_to_rc(e),
    }
}
