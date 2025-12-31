//! FFI exports for libnx environment functions

use core::ffi::{c_char, c_uint, c_void};

use crate::{
    AccountUid, ConfigEntry, LoaderReturnFn, env_get_argv, env_get_exit_func_ptr,
    env_get_heap_override_addr, env_get_heap_override_size, env_get_last_load_result,
    env_get_loader_info, env_get_loader_info_size, env_get_main_thread_handle,
    env_get_own_process_handle, env_get_random_seed, env_get_user_id_storage, env_has_argv,
    env_has_heap_override, env_has_next_load, env_has_random_seed, env_is_nso,
    env_is_syscall_hinted, env_set_exit_func_ptr, env_set_next_load, env_setup,
};

/// Parse the homebrew loader environment configuration
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envSetup(
    ctx: *const ConfigEntry,
    main_thread: u32,
    saved_lr: LoaderReturnFn,
) {
    unsafe { env_setup(ctx, main_thread, saved_lr) }
}

/// Get loader info string pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetLoaderInfo() -> *const c_char {
    env_get_loader_info()
}

/// Get loader info size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetLoaderInfoSize() -> u64 {
    env_get_loader_info_size()
}

/// Get main thread handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetMainThreadHandle() -> u32 {
    env_get_main_thread_handle()
}

/// Returns true if running as NSO
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envIsNso() -> bool {
    env_is_nso()
}

/// Returns true if heap override is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envHasHeapOverride() -> bool {
    env_has_heap_override()
}

/// Get heap override address
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetHeapOverrideAddr() -> *mut c_void {
    env_get_heap_override_addr()
}

/// Get heap override size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetHeapOverrideSize() -> u64 {
    env_get_heap_override_size()
}

/// Returns true if argv is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envHasArgv() -> bool {
    env_has_argv()
}

/// Get argv string pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetArgv() -> *const c_char {
    env_get_argv()
}

/// Returns true if the given syscall is hinted as available
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envIsSyscallHinted(svc: c_uint) -> bool {
    env_is_syscall_hinted(svc)
}

/// Get process handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetOwnProcessHandle() -> u32 {
    env_get_own_process_handle()
}

/// Get exit function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetExitFuncPtr() -> LoaderReturnFn {
    env_get_exit_func_ptr()
}

/// Set exit function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envSetExitFuncPtr(func: LoaderReturnFn) {
    env_set_exit_func_ptr(func)
}

/// Set next NRO to load (chain loading)
///
/// Returns 0 on success, non-zero on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envSetNextLoad(
    path: *const c_char,
    argv: *const c_char,
) -> u32 {
    env_set_next_load(path, argv)
}

/// Returns true if chain loading is supported
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envHasNextLoad() -> bool {
    env_has_next_load()
}

/// Get last load result
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetLastLoadResult() -> u32 {
    env_get_last_load_result()
}

/// Returns true if random seed is present
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envHasRandomSeed() -> bool {
    env_has_random_seed()
}

/// Get random seed (copies to output buffer)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetRandomSeed(out: *mut u64) {
    if out.is_null() {
        return;
    }
    let mut seed = [0u64; 2];
    env_get_random_seed(&mut seed);
    unsafe {
        *out = seed[0];
        *out.add(1) = seed[1];
    }
}

/// Get user ID storage pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_env__envGetUserIdStorage() -> *mut AccountUid {
    env_get_user_id_storage()
}
