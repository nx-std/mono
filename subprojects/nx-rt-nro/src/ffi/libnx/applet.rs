//! Applet service FFI (NRO).
//!
//! Exposes only the applet-type-sourcing entry points — `appletInitialize`
//! and `appletGetAppletType`. The homebrew NRO reads its applet type at
//! runtime from the parsed loader configuration block.
//!
//! The kind-agnostic `applet*` accessor surface (`appletExit`,
//! `appletGetOperationMode`, the notification setters, …) is shared knowledge
//! and lives in [`nx_rt_core::ffi::libnx::applet`]; the `rt_nro_libnx_service_applet.ld`
//! fragment binds it for this link.

use nx_rt_core::error::ToResultCode as _;
use nx_svc::process::Handle as ProcessHandle;

use crate::{env, ffi::common::GENERIC_ERROR, services::applet};

/// Initializes the applet service. Returns 0 on success, error code on failure.
///
/// Corresponds to `appletInitialize()` in `applet.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_applet_initialize() -> u32 {
    // The applet type is loader-supplied: read it from the parsed environment
    // state rather than a weak global. The NRO hbl config never yields
    // `AppletType::None`, and `applet::init` resolves `Default` and
    // short-circuits `None` itself, so the dispatch stays unchanged.
    let Some(applet_type) =
        nx_service_applet::AppletType::from_raw(env::applet_type().as_raw() as i32)
    else {
        return GENERIC_ERROR;
    };

    // Get process handle
    let process_handle = env::own_process_handle()
        .map(|h| {
            // SAFETY: Handle from env::own_process_handle() is guaranteed valid.
            unsafe { ProcessHandle::from_raw(h.to_raw()) }
        })
        .unwrap_or_else(ProcessHandle::current_process);

    if let Err(err) = applet::init(applet_type, process_handle) {
        return err.to_rc();
    }

    0
}

/// Gets the current applet type.
///
/// Corresponds to `appletGetAppletType()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_applet_get_applet_type() -> i32 {
    // Report the loader-supplied applet type from the parsed environment state.
    env::applet_type().as_raw() as i32
}
