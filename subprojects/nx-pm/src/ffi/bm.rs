//! `pm:bm` (boot mode) FFI.

use nx_sf::{
    error::ToResultCode as _,
    ffi::Service,
};

use super::{
    common::GENERIC_ERROR,
    state::{
        BM,
        BM_SRV,
        clear_shadow,
        ensure_sm,
        write_shadow,
    },
};

/// `pmbmInitialize()` — opens a session to `pm:bm`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmbm_initialize() -> u32 {
    {
        let guard = BM.read();
        if guard.is_some() {
            return 0;
        }
    }

    if let Err(err) = ensure_sm() {
        return err.to_rc();
    }

    let sm_guard = super::state::SM.read();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    let svc = match nx_service_pm::connect_bm_cmif(sm) {
        Ok(s) => s,
        Err(err) => return err.to_rc(),
    };

    let mut guard = BM.write();
    if guard.is_some() {
        // Lost the race; drop the duplicate session.
        return 0;
    }
    write_shadow(&BM_SRV, svc.session());
    *guard = Some(svc);
    0
}

/// `pmbmExit()` — closes the `pm:bm` session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmbm_exit() {
    let mut guard = BM.write();
    if guard.take().is_some() {
        clear_shadow(&BM_SRV);
    }
}

/// `pmbmGetServiceSession()` — returns the libnx `Service*` for `pm:bm`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmbm_get_service_session() -> *mut Service {
    BM_SRV.get().cast::<Service>()
}

/// `pmbmGetBootMode(PmBootMode *out)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmbm_get_boot_mode(out: *mut u32) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let guard = BM.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.get_boot_mode() {
        Ok(mode) => {
            // `BootMode` is `#[repr(u32)]`, sized to match libnx's `PmBootMode`.
            // SAFETY: caller guarantees `out` is writable for `u32`.
            unsafe { *out = mode as u32 };
            0
        }
        Err(e) => e.to_rc(),
    }
}

/// `pmbmSetMaintenanceBoot()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_pm__pmbm_set_maintenance_boot() -> u32 {
    let guard = BM.read();
    let Some(svc) = guard.as_ref() else {
        return GENERIC_ERROR;
    };

    match svc.set_maintenance_boot() {
        Ok(()) => 0,
        Err(e) => e.to_rc(),
    }
}
