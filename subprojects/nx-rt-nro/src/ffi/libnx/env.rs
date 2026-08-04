//! Environment/loader config FFI

use core::ptr::NonNull;

use nx_svc::thread::Handle as ThreadHandle;

use crate::env::{
    self,
    ConfigEntry,
    LoaderReturnFn,
};

/// nxlink host address (C-compatible, network byte order)
///
/// This corresponds to `struct in_addr __nxlink_host` in libnx.
#[unsafe(no_mangle)]
pub static mut __nx_rt_nro__libnx_nxlink_host: u32 = 0;

/// Set the nxlink host address
///
/// Called from nxlink::strip_nxlink_suffix() when the _NXLINK_ suffix is detected.
pub(crate) fn set_nxlink_host(addr: u32) {
    unsafe {
        __nx_rt_nro__libnx_nxlink_host = addr;
    }
}

/// Global applet type (C-compatible).
///
/// Backing storage for libnx's `__nx_applet_type` global, kept in sync with
/// the parsed environment state for C consumers that read it directly. The
/// Rust applet-init dispatch never reads this back — it sources the applet
/// type from [`env::applet_type`], the loader-supplied runtime value.
#[unsafe(no_mangle)]
pub static mut __nx_rt_nro__libnx_applet_type: u32 = 0;

/// Publishes the parsed applet type to the libnx-facing global.
pub(crate) fn set_applet_type(applet_type: u32) {
    unsafe { __nx_rt_nro__libnx_applet_type = applet_type };
}

/// Parse the homebrew loader environment configuration.
///
/// Corresponds to `envSetup()` in `env.h`.
///
/// # Safety
///
/// `ctx` must point to a valid `ConfigEntry` array terminated by `EndOfList`,
/// or be null. `main_thread` must be the kernel-supplied main-thread handle
/// and `saved_lr` the loader-return function, both as passed by the crt0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_env_setup(
    ctx: *const ConfigEntry,
    main_thread: u32,
    saved_lr: LoaderReturnFn,
) {
    // The homebrew loader always supplies a non-null configuration block for
    // an NRO; a null `ctx` would be a malformed launch with nothing to parse.
    let Some(ctx) = NonNull::new(ctx.cast_mut()) else {
        return;
    };

    // SAFETY: Caller (libnx CRT0) guarantees `ctx` points to a valid
    // ConfigEntry array terminated by EndOfList, and `main_thread` is the
    // kernel-supplied main-thread handle.
    unsafe { env::setup(ctx, ThreadHandle::from_raw_unchecked(main_thread), saved_lr) }

    // Publish the parsed applet type to the global libnx-facing variable.
    set_applet_type(env::applet_type().as_raw());
}
