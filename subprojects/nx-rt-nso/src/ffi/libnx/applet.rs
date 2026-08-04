//! Application Manager (applet) bring-up FFI (NSO).
//!
//! Redirects the `libnx` `applet*` startup entry points to `nx-rt-nso`. Unlike
//! the homebrew-NRO surface — which sources the applet type at runtime from
//! the loader configuration block — the NSO surface reports the build-time
//! [`APPLET_TYPE`](crate::applet::APPLET_TYPE) selected by the active
//! `applet-*` Cargo feature.
//!
//! Only the applet-type-sourcing entry points (`appletInitialize`,
//! `appletGetAppletType`, `__nx_applet_type`) are exposed here. The
//! kind-agnostic `applet*` accessor surface (`appletExit`,
//! `appletGetOperationMode`, …) is shared knowledge owned by
//! [`nx_rt_core::ffi::libnx::applet`]; the `rt_nso_libnx_service_applet.ld` fragment binds
//! it for an NSO link just as it does for an NRO one.

use nx_rt_core::error::ToResultCode as _;
use nx_svc::process::Handle as ProcessHandle;

use crate::{
    applet as nso_applet,
    env,
};

/// Global applet type (C-compatible).
///
/// Backing storage for libnx's `__nx_applet_type` global. For an NSO the
/// Application Manager identity is fixed at build time, so the value is the
/// constant [`APPLET_TYPE`](crate::applet::APPLET_TYPE) rather than a
/// loader-supplied one — hence an immutable `static`.
#[unsafe(no_mangle)]
pub static __nx_rt_nso__libnx_applet_type: u32 = nso_applet::APPLET_TYPE.as_raw() as u32;

/// Initializes the applet service. Returns 0 on success, an error code on
/// failure.
///
/// Corresponds to `appletInitialize()` in `applet.h`. The applet type is the
/// build-time [`APPLET_TYPE`](crate::applet::APPLET_TYPE); a `None` identity
/// (background sysmodule) skips the Application Manager handshake entirely.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_applet_initialize() -> u32 {
    // The NSO's own process handle; falls back to the current-process pseudo
    // handle when the environment state carries none.
    let process_handle = env::own_process_handle()
        .map(|h| {
            // SAFETY: a handle from `env::own_process_handle()` is valid.
            ProcessHandle::from_raw_unchecked(h.to_raw())
        })
        .unwrap_or_else(ProcessHandle::current_process);

    // The applet handshake yields a rich `ConnectError`; the shared converter
    // maps it to the libnx result code the C ABI carries.
    match nso_applet::applet_init(process_handle) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Gets the current applet type.
///
/// Corresponds to `appletGetAppletType()` in `applet.h`. Reports the
/// build-time [`APPLET_TYPE`](crate::applet::APPLET_TYPE).
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_applet_get_applet_type() -> i32 {
    nso_applet::APPLET_TYPE.as_raw()
}
