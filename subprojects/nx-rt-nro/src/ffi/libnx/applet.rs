//! Applet service FFI (NRO).
//!
//! Exposes only the applet-type-sourcing entry points: `appletInitialize`
//! and `appletGetAppletType`. The homebrew NRO reads its applet type at
//! runtime from the parsed loader configuration block.
//!
//! The kind-agnostic `applet*` accessor surface (`appletExit`,
//! `appletGetOperationMode`, the notification setters, …) is shared knowledge
//! and lives in [`nx_rt_core::ffi::libnx::applet`]; the `rt_nro_libnx_service_applet.ld`
//! fragment binds it for this link.

use nx_rt_core::error::ToResultCode as _;

use crate::{
    env,
    services::applet,
};

/// Initializes the applet service. Returns 0 on success, error code on failure.
///
/// Corresponds to `appletInitialize()` in `applet.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_applet_initialize() -> u32 {
    // The applet type is loader-supplied, so reading it and dispatching on it
    // is the NRO's own step rather than the shared handshake's. It lives in
    // the manager beside the session it opens, because the startup sequence
    // performs the same step and the two must not drift.
    match applet::init_from_env() {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
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
    env::applet_type().as_raw()
}
