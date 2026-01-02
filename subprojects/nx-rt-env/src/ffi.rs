//! FFI exports for libnx environment functions

use core::ffi::{c_char, c_uint, c_void};

use nx_svc::thread::Handle as ThreadHandle;

use crate::{
    AccountUid, ConfigEntry, LoaderReturnFn, argv, exit_func_ptr, has_next_load, heap_override,
    hos_version, is_nso, last_load_result, loader_info, main_thread_handle, own_process_handle,
    random_seed, set_exit_func_ptr, set_next_load, setup, syscall_hints, user_id_storage,
};

/// Parse the homebrew loader environment configuration
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_setup(
    ctx: *const ConfigEntry,
    main_thread: u32,
    saved_lr: LoaderReturnFn,
) {
    // SAFETY: Caller (libnx CRT0) guarantees that ctx is either null (NSO mode)
    // or points to a valid ConfigEntry array terminated by EndOfList.
    // The main_thread handle is provided by the kernel/loader and is guaranteed valid.
    unsafe { setup(ctx, ThreadHandle::from_raw(main_thread), saved_lr) }
}

/// Initialize main thread TLS (ThreadVars and .tdata copy)
///
/// Rust port of libnx's `newlibSetup()`. Must be called after `envSetup()`
/// and before the allocator is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__setup_main_thread_tls() {
    // SAFETY: Caller (libnx initialization sequence) guarantees this is called
    // exactly once during main thread startup, before any allocator use.
    unsafe { crate::main_thread::setup() }
}

/// Get loader info string pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_loader_info() -> *const c_char {
    match loader_info() {
        Some((ptr, _)) => ptr.as_ptr() as *const c_char,
        None => core::ptr::null(),
    }
}

/// Get loader info size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_loader_info_size() -> u64 {
    match loader_info() {
        Some((_, size)) => size,
        None => 0,
    }
}

/// Get main thread handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_main_thread_handle() -> u32 {
    main_thread_handle().to_raw()
}

/// Returns true if running as NSO
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_is_nso() -> bool {
    is_nso()
}

/// Returns true if heap override is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_has_heap_override() -> bool {
    heap_override().is_some()
}

/// Get heap override address
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_heap_override_addr() -> *mut c_void {
    match heap_override() {
        Some((addr, _)) => addr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get heap override size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_heap_override_size() -> u64 {
    match heap_override() {
        Some((_, size)) => size as u64,
        None => 0,
    }
}

/// Returns true if argv is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_has_argv() -> bool {
    argv().is_some()
}

/// Get argv string pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_argv() -> *const c_char {
    match argv() {
        Some(ptr) => ptr,
        None => core::ptr::null(),
    }
}

/// Returns true if the given syscall is hinted as available
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_is_syscall_hinted(svc: c_uint) -> bool {
    syscall_hints().is_available(svc)
}

/// Get process handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_own_process_handle() -> u32 {
    own_process_handle().to_raw()
}

/// Get exit function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_exit_func_ptr() -> LoaderReturnFn {
    exit_func_ptr()
}

/// Set exit function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_set_exit_func_ptr(func: LoaderReturnFn) {
    set_exit_func_ptr(func)
}

/// Set next NRO to load (chain loading)
///
/// Returns 0 on success, non-zero on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_set_next_load(
    path: *const c_char,
    argv: *const c_char,
) -> u32 {
    set_next_load(path, argv)
}

/// Returns true if chain loading is supported
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_has_next_load() -> bool {
    has_next_load()
}

/// Get last load result
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_last_load_result() -> u32 {
    last_load_result()
}

/// Returns true if random seed is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_has_random_seed() -> bool {
    random_seed().is_some()
}

/// Get random seed (copies to output buffer)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__env_get_random_seed(out: *mut u64) {
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
pub unsafe extern "C" fn __nx_rt_env__env_get_user_id_storage() -> *mut AccountUid {
    match user_id_storage() {
        Some(ptr) => ptr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get the current HOS version (without Atmosphere bit).
///
/// Equivalent to libnx's `hosversionGet()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__hosversion_get() -> u32 {
    hos_version::get().as_u32()
}

/// Set the HOS version.
///
/// Equivalent to libnx's `hosversionSet()`.
/// This should only be called from envSetup/appInit in C code.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__hosversion_set(version: u32) {
    hos_version::set(version)
}

/// Check if running on Atmosphere.
///
/// Equivalent to libnx's `hosversionIsAtmosphere()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__hosversion_is_atmosphere() -> bool {
    hos_version::is_atmosphere()
}
