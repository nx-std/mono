//! FFI exports for libnx runtime functions

use core::ffi::{c_char, c_uint, c_void};

use nx_svc::{raw::INVALID_HANDLE, thread::Handle as ThreadHandle};

use crate::{
    argv,
    env::{
        self, AccountUid, ConfigEntry, LoaderReturnFn, applet_type, argv as env_argv,
        exit_func_ptr, has_next_load, heap_override, hos_version, is_nso, last_load_result,
        loader_info, main_thread_handle, own_process_handle, random_seed, service_overrides,
        set_exit_func_ptr, set_next_load, setup, syscall_hints, user_id_storage,
    },
};

// libnx C symbols
unsafe extern "C" {
    /// Register a service override handle (libnx sm.c)
    /// Note: SmServiceName is `struct { char name[8]; }` which is ABI-equivalent to u64
    fn smAddOverrideHandle(name: u64, handle: u32);

    /// Global applet type variable (libnx applet.c)
    static mut __nx_applet_type: u32;
}

/// Parse the homebrew loader environment configuration
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_setup(
    ctx: *const ConfigEntry,
    main_thread: u32,
    saved_lr: LoaderReturnFn,
) {
    // SAFETY: Caller (libnx CRT0) guarantees that ctx is either null (NSO mode)
    // or points to a valid ConfigEntry array terminated by EndOfList.
    // The main_thread handle is provided by the kernel/loader and is guaranteed valid.
    unsafe { setup(ctx, ThreadHandle::from_raw(main_thread), saved_lr) }

    // Sync parsed values with libnx globals
    for ovr in service_overrides().iter().flatten() {
        // SAFETY: smAddOverrideHandle is safe to call during init
        // Pass the raw u64 value which is ABI-equivalent to SmServiceName
        unsafe { smAddOverrideHandle(ovr.name.to_raw(), ovr.handle.to_raw()) }
    }

    // SAFETY: Single-threaded initialization, exclusive access to the global variable.
    unsafe { __nx_applet_type = applet_type().as_raw() };
}

/// Initialize main thread TLS (ThreadVars and .tdata copy)
///
/// Rust port of libnx's `newlibSetup()`. Must be called after `envSetup()`
/// and before the allocator is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setup_main_thread_tls() {
    // SAFETY: Caller (libnx initialization sequence) guarantees this is called
    // exactly once during main thread startup, before any allocator use.
    unsafe { env::main_thread::setup() }
}

/// Get loader info string pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_loader_info() -> *const c_char {
    match loader_info() {
        Some((ptr, _)) => ptr.as_ptr() as *const c_char,
        None => core::ptr::null(),
    }
}

/// Get loader info size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_loader_info_size() -> u64 {
    match loader_info() {
        Some((_, size)) => size,
        None => 0,
    }
}

/// Get main thread handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_main_thread_handle() -> u32 {
    main_thread_handle().to_raw()
}

/// Returns true if running as NSO
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_is_nso() -> bool {
    is_nso()
}

/// Returns true if heap override is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_heap_override() -> bool {
    heap_override().is_some()
}

/// Get heap override address
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_heap_override_addr() -> *mut c_void {
    match heap_override() {
        Some((addr, _)) => addr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get heap override size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_heap_override_size() -> u64 {
    match heap_override() {
        Some((_, size)) => size as u64,
        None => 0,
    }
}

/// Returns true if argv is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_argv() -> bool {
    env_argv().is_some()
}

/// Get argv string pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_argv() -> *const c_char {
    match env_argv() {
        Some(ptr) => ptr,
        None => core::ptr::null(),
    }
}

/// Returns true if the given syscall is hinted as available
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_is_syscall_hinted(svc: c_uint) -> bool {
    syscall_hints().is_available(svc)
}

/// Get process handle (returns INVALID_HANDLE if not set)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_own_process_handle() -> u32 {
    own_process_handle().map_or(INVALID_HANDLE, |h| h.to_raw())
}

/// Get exit function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_exit_func_ptr() -> LoaderReturnFn {
    exit_func_ptr()
}

/// Set exit function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_set_exit_func_ptr(func: LoaderReturnFn) {
    set_exit_func_ptr(func)
}

/// Set next NRO to load (chain loading)
///
/// Returns 0 on success, non-zero on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_set_next_load(
    path: *const c_char,
    argv: *const c_char,
) -> u32 {
    set_next_load(path, argv)
}

/// Returns true if chain loading is supported
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_next_load() -> bool {
    has_next_load()
}

/// Get last load result
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_last_load_result() -> u32 {
    last_load_result()
}

/// Returns true if random seed is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_has_random_seed() -> bool {
    random_seed().is_some()
}

/// Get random seed (copies to output buffer)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_random_seed(out: *mut u64) {
    if out.is_null() {
        return;
    }
    if let Some([seed0, seed1]) = random_seed() {
        // SAFETY: Caller guarantees out points to a valid buffer with space for 2 u64 values.
        // We verified out is non-null above.
        unsafe {
            *out = seed0;
            *out.add(1) = seed1;
        }
    }
}

/// Get user ID storage pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__env_get_user_id_storage() -> *mut AccountUid {
    match user_id_storage() {
        Some(ptr) => ptr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get the current HOS version (without Atmosphere bit).
///
/// Equivalent to libnx's `hosversionGet()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hosversion_get() -> u32 {
    hos_version::get().as_u32()
}

/// Set the HOS version.
///
/// Equivalent to libnx's `hosversionSet()`.
/// This should only be called from envSetup/appInit in C code.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hosversion_set(version: u32) {
    hos_version::set(version)
}

/// Check if running on Atmosphere.
///
/// Equivalent to libnx's `hosversionIsAtmosphere()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hosversion_is_atmosphere() -> bool {
    hos_version::is_atmosphere()
}

/// Wrapper for static argv array to implement Sync
struct StaticArgv([*mut c_char; 1]);

// SAFETY: This is a null-terminated pointer array. While the pointers themselves
// aren't Sync, the array is static and immutable (contains only null pointers).
unsafe impl Sync for StaticArgv {}

/// Empty argv - null-terminated array with zero arguments
static EMPTY_ARGV: StaticArgv = StaticArgv([core::ptr::null_mut()]);

/// FFI-exported argc (for C code compatibility)
#[unsafe(no_mangle)]
pub static mut __nx_rt__system_argc: i32 = 0;

/// FFI-exported argv (for C code compatibility)
#[unsafe(no_mangle)]
pub static mut __nx_rt__system_argv: *mut *mut c_char = EMPTY_ARGV.0.as_ptr() as *mut _;

/// Setup argv parsing
///
/// # Safety
///
/// Must be called after the global allocator is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__argv_setup() {
    unsafe { argv::setup() }
}

/// Set the C-style argc/argv globals
///
/// # Safety
///
/// Only called from argv::setup() with valid argc/argv pointers
pub(crate) unsafe fn set_system_argv(argc: i32, argv: *mut *mut c_char) {
    unsafe {
        __nx_rt__system_argc = argc;
        __nx_rt__system_argv = argv;
    }
}

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
pub(crate) fn get_nxlink_host() -> Option<u32> {
    let addr = unsafe { __nx_rt__nxlink_host };
    if addr != 0 { Some(addr) } else { None }
}
