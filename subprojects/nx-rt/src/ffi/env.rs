//! Environment/loader config FFI

use core::ffi::{c_char, c_uint, c_void};

use nx_svc::{raw::INVALID_HANDLE, thread::Handle as ThreadHandle};

use crate::{
    env::{self, AccountUid, ConfigEntry, LoaderReturnFn},
    init, service_manager,
};

// ============================================================================
// Global Environment Variables
// ============================================================================

/// nxlink host address (C-compatible, network byte order)
///
/// This corresponds to `struct in_addr __nxlink_host` in libnx.
#[unsafe(no_mangle)]
pub static mut __nx_rt__nxlink_host: u32 = 0;

/// Set the nxlink host address
///
/// Called from nxlink::strip_nxlink_suffix() when the _NXLINK_ suffix is detected.
pub(crate) fn set_nxlink_host(addr: u32) {
    unsafe {
        __nx_rt__nxlink_host = addr;
    }
}

/// Get the nxlink host address
///
/// Returns None if no nxlink host was detected.
#[allow(dead_code)]
pub(crate) fn get_nxlink_host() -> Option<u32> {
    let addr = unsafe { __nx_rt__nxlink_host };
    if addr != 0 { Some(addr) } else { None }
}

/// Global applet type (C-compatible)
///
/// Default is `AppletType_Default` (0). Set during env setup.
/// Corresponds to `__nx_applet_type` in libnx applet.c.
#[unsafe(no_mangle)]
pub static mut __nx_rt__applet_type: u32 = 0;

/// Set the applet type
pub(crate) fn set_applet_type(applet_type: u32) {
    unsafe { __nx_rt__applet_type = applet_type };
}

/// Get the applet type
pub(crate) fn get_applet_type() -> u32 {
    unsafe { __nx_rt__applet_type }
}

// ============================================================================
// Environment Setup FFI
// ============================================================================

/// Parse the homebrew loader environment configuration.
///
/// Corresponds to `envSetup()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_setup(
    ctx: *const ConfigEntry,
    main_thread: u32,
    saved_lr: LoaderReturnFn,
) {
    // SAFETY: Caller (libnx CRT0) guarantees that ctx is either null (NSO mode)
    // or points to a valid ConfigEntry array terminated by EndOfList.
    // The main_thread handle is provided by the kernel/loader and is guaranteed valid.
    unsafe { env::setup(ctx, ThreadHandle::from_raw(main_thread), saved_lr) }

    // Register service overrides with the Rust service manager
    for ovr in env::service_overrides().iter().flatten() {
        let _ = service_manager::add_override(ovr.name, ovr.handle);
    }

    // Set global applet type from env config
    set_applet_type(env::applet_type().as_raw());
}

/// Initialize main thread TLS (ThreadVars and .tdata copy).
///
/// Must be called after `envSetup()` and before the allocator is initialized.
///
/// Corresponds to `newlibSetup()` in `newlib.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setup_main_thread_tls() {
    // SAFETY: Caller (libnx initialization sequence) guarantees this is called
    // exactly once during main thread startup, before any allocator use.
    unsafe { env::main_thread::setup() }
}

/// Get loader info string pointer.
///
/// Corresponds to `envGetLoaderInfo()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_loader_info() -> *const c_char {
    match env::loader_info() {
        Some((ptr, _)) => ptr.as_ptr() as *const c_char,
        None => core::ptr::null(),
    }
}

/// Get loader info size.
///
/// Corresponds to `envGetLoaderInfoSize()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_loader_info_size() -> u64 {
    match env::loader_info() {
        Some((_, size)) => size,
        None => 0,
    }
}

/// Get main thread handle.
///
/// Corresponds to `envGetMainThreadHandle()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_main_thread_handle() -> u32 {
    env::main_thread_handle().to_raw()
}

/// Returns true if running as NSO.
///
/// Corresponds to `envIsNso()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_is_nso() -> bool {
    env::is_nso()
}

/// Returns true if heap override is present.
///
/// Corresponds to `envHasHeapOverride()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_heap_override() -> bool {
    env::heap_override().is_some()
}

/// Get heap override address.
///
/// Corresponds to `envGetHeapOverrideAddr()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_heap_override_addr() -> *mut c_void {
    match env::heap_override() {
        Some((addr, _)) => addr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get heap override size.
///
/// Corresponds to `envGetHeapOverrideSize()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_heap_override_size() -> u64 {
    match env::heap_override() {
        Some((_, size)) => size as u64,
        None => 0,
    }
}

/// Initialize the allocator heap.
///
/// Uses heap override from loader config if available, otherwise allocates via SVC.
///
/// Corresponds to `__libnx_initheap()` in `init.c`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__initheap() {
    init::setup_heap();
}

/// Returns true if argv is present.
///
/// Corresponds to `envHasArgv()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_argv() -> bool {
    env::argv().is_some()
}

/// Get argv string pointer.
///
/// Corresponds to `envGetArgv()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_argv() -> *const c_char {
    env::argv().unwrap_or_default()
}

/// Returns true if the given syscall is hinted as available.
///
/// Corresponds to `envIsSyscallHinted()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_is_syscall_hinted(svc: c_uint) -> bool {
    env::syscall_hints().is_available(svc)
}

/// Get process handle (returns INVALID_HANDLE if not set).
///
/// Corresponds to `envGetOwnProcessHandle()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_own_process_handle() -> u32 {
    env::own_process_handle().map_or(INVALID_HANDLE, |h| h.to_raw())
}

/// Get exit function pointer.
///
/// Corresponds to `envGetExitFuncPtr()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_exit_func_ptr() -> LoaderReturnFn {
    env::exit_func_ptr()
}

/// Set exit function pointer.
///
/// Corresponds to `envSetExitFuncPtr()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_set_exit_func_ptr(func: LoaderReturnFn) {
    env::set_exit_func_ptr(func)
}

/// Set next NRO to load (chain loading).
///
/// Returns 0 on success, non-zero on error.
///
/// Corresponds to `envSetNextLoad()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_set_next_load(
    path: *const c_char,
    argv: *const c_char,
) -> u32 {
    // SAFETY: Caller guarantees path and argv are valid C strings or null
    unsafe { env::set_next_load(path, argv) }
}

/// Returns true if chain loading is supported.
///
/// Corresponds to `envHasNextLoad()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_next_load() -> bool {
    env::has_next_load()
}

/// Get last load result.
///
/// Corresponds to `envGetLastLoadResult()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_last_load_result() -> u32 {
    env::last_load_result()
}

/// Returns true if random seed is present.
///
/// Corresponds to `envHasRandomSeed()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_random_seed() -> bool {
    env::random_seed().is_some()
}

/// Get random seed (copies to output buffer).
///
/// Corresponds to `envGetRandomSeed()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_random_seed(out: *mut u64) {
    if out.is_null() {
        return;
    }
    if let Some([seed0, seed1]) = env::random_seed() {
        // SAFETY: Caller guarantees out points to a valid buffer with space for 2 u64 values.
        // We verified out is non-null above.
        unsafe {
            *out = seed0;
            *out.add(1) = seed1;
        }
    }
}

/// Get user ID storage pointer.
///
/// Corresponds to `envGetUserIdStorage()` in `env.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_user_id_storage() -> *mut AccountUid {
    match env::user_id_storage() {
        Some(ptr) => ptr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get the current HOS version (without Atmosphere bit).
///
/// Corresponds to `hosversionGet()` in `hosversion.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hosversion_get() -> u32 {
    env::hos_version::get().as_u32()
}

/// Set the HOS version.
///
/// This should only be called from envSetup/appInit in C code.
///
/// Corresponds to `hosversionSet()` in `hosversion.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hosversion_set(version: u32) {
    env::hos_version::set(version)
}

/// Check if running on Atmosphere.
///
/// Corresponds to `hosversionIsAtmosphere()` in `hosversion.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hosversion_is_atmosphere() -> bool {
    env::hos_version::is_atmosphere()
}
