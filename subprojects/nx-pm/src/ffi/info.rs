//! `pm:info` (process info) FFI.

use nx_service_pm::{ProcessId, ResourceLimitValues};
use nx_sf::{error::ToResultCode as _, ffi::Service};

use super::{
    common::{GENERIC_ERROR, RC_INCOMPAT_SYSVER, hosversion_before, hosversionIsAtmosphere},
    state::{INFO, INFO_SRV, clear_shadow, ensure_sm, write_shadow},
};

/// `pminfoInitialize()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pminfo_initialize() -> u32 {
    {
        let guard = INFO.read();
        if guard.is_some() {
            return 0;
        }
    }

    if let Err(err) = ensure_sm() {
        return err.to_rc();
    }

    let sm_guard = super::state::SM.read();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let svc = match nx_service_pm::connect_info_cmif(sm) {
        Ok(s) => s,
        Err(err) => return err.to_rc(),
    };

    let mut guard = INFO.write();
    if guard.is_some() {
        return 0;
    }
    write_shadow(&INFO_SRV, svc.session());
    *guard = Some(svc);
    0
}

/// `pminfoExit()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pminfo_exit() {
    let mut guard = INFO.write();
    if guard.take().is_some() {
        clear_shadow(&INFO_SRV);
    }
}

/// `pminfoGetServiceSession()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pminfo_get_service_session() -> *mut Service {
    INFO_SRV.get().cast::<Service>()
}

/// `pminfoGetProgramId(u64 *program_id_out, u64 pid)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pminfo_get_program_id(program_id_out: *mut u64, pid: u64) -> u32 {
    if program_id_out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = INFO.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_program_id(unsafe { ProcessId::new_unchecked(pid) }) {
        Ok(program_id) => {
            // SAFETY: caller guarantees `program_id_out` is writable.
            unsafe { *program_id_out = program_id.to_u64() };
            0
        }
        Err(e) => e.to_rc(),
    }
}

/// `pminfoGetAppletCurrentResourceLimitValues(PmResourceLimitValues *out)` — `[14.0.0+/Atmosphere]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pminfo_get_applet_current_resource_limit_values(
    out: *mut ResourceLimitValues,
) -> u32 {
    if !hosversionIsAtmosphere() && hosversion_before(14, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = INFO.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_applet_current_resource_limit_values() {
        Ok(vals) => {
            // SAFETY: caller guarantees `out` is writable; layout matches
            // libnx's `PmResourceLimitValues` (asserted in `nx-service-pm`).
            unsafe { *out = vals };
            0
        }
        Err(e) => e.to_rc(),
    }
}

/// `pminfoGetAppletPeakResourceLimitValues(PmResourceLimitValues *out)` — `[14.0.0+/Atmosphere]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pminfo_get_applet_peak_resource_limit_values(
    out: *mut ResourceLimitValues,
) -> u32 {
    if !hosversionIsAtmosphere() && hosversion_before(14, 0, 0) {
        return RC_INCOMPAT_SYSVER;
    }
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = INFO.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_applet_peak_resource_limit_values() {
        Ok(vals) => {
            // SAFETY: caller guarantees `out` is writable.
            unsafe { *out = vals };
            0
        }
        Err(e) => e.to_rc(),
    }
}
