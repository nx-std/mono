//! Kind-agnostic environment FFI.
//!
//! Redirects the `libnx` runtime entry points that every Switch executable
//! kind shares: heap initialization, main-thread TLS, the environment
//! read accessors, and the HOS-version API: to `nx-rt-core`.
//!
//! The kind-specific entry points (`envSetup`, `argvSetup`, `__system_argc` /
//! `__system_argv`, `__nxlink_host`, `__nx_applet_type`) are intentionally
//! absent: each output-kind entry crate owns them.
//!
//! Every function here is an FFI entry point invoked across the C ABI. The
//! per-function `# Safety` sections document only the additional caller
//! obligations beyond upholding that ABI.

use core::ffi::{
    CStr,
    c_char,
    c_uint,
    c_void,
};

use nx_svc::raw::INVALID_HANDLE;

use crate::{
    env::{
        self,
        AccountUid,
        LoaderReturnFn,
    },
    error::{
        LibnxError,
        ToResultCode as _,
        libnx_error,
    },
    init,
};

/// Initialize the allocator heap.
///
/// Uses the heap override from the loader config if available, otherwise
/// allocates via SVC.
///
/// Corresponds to `__libnx_initheap()` in `init.c`.
///
/// # Safety
///
/// Must be called exactly once during runtime startup, before the global
/// allocator services any allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_initheap() {
    init::setup_heap();
}

/// Initialize main thread TLS (ThreadVars and `.tdata` copy).
///
/// Must be called after `envSetup()` and before the allocator is initialized.
///
/// Corresponds to `newlibSetup()` in `newlib.c`.
///
/// # Safety
///
/// Must be called exactly once during main-thread startup, after `envSetup`
/// and before the global allocator is used.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_setup_main_thread_tls() {
    // SAFETY: Caller (libnx initialization sequence) guarantees this is called
    // exactly once during main thread startup, before any allocator use.
    unsafe { env::main_thread::setup() }
}

/// Get loader info string pointer.
///
/// Corresponds to `envGetLoaderInfo()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_loader_info() -> *const c_char {
    match env::loader_info() {
        Some((ptr, _)) => ptr.as_ptr() as *const c_char,
        None => core::ptr::null(),
    }
}

/// Get loader info size.
///
/// Corresponds to `envGetLoaderInfoSize()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_loader_info_size() -> u64 {
    match env::loader_info() {
        Some((_, size)) => size,
        None => 0,
    }
}

/// Get main thread handle.
///
/// Corresponds to `envGetMainThreadHandle()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_main_thread_handle() -> u32 {
    env::main_thread_handle().to_raw()
}

/// Returns true if running as NSO.
///
/// Corresponds to `envIsNso()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_is_nso() -> bool {
    env::is_nso()
}

/// Returns true if a heap override is present.
///
/// Corresponds to `envHasHeapOverride()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_has_heap_override() -> bool {
    env::heap_override().is_some()
}

/// Get heap override address.
///
/// Corresponds to `envGetHeapOverrideAddr()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_heap_override_addr() -> *mut c_void {
    match env::heap_override() {
        Some((addr, _)) => addr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get heap override size.
///
/// Corresponds to `envGetHeapOverrideSize()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_heap_override_size() -> u64 {
    match env::heap_override() {
        Some((_, size)) => size as u64,
        None => 0,
    }
}

/// Returns true if argv is present.
///
/// Corresponds to `envHasArgv()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_has_argv() -> bool {
    env::argv().is_some()
}

/// Get argv string pointer.
///
/// Corresponds to `envGetArgv()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_argv() -> *const c_char {
    env::argv().map_or(core::ptr::null(), |argv| argv.as_ptr() as *const c_char)
}

/// Returns true if the given syscall is hinted as available.
///
/// Corresponds to `envIsSyscallHinted()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_is_syscall_hinted(svc: c_uint) -> bool {
    env::syscall_hints().is_available(svc)
}

/// Get process handle (returns `INVALID_HANDLE` if not set).
///
/// Corresponds to `envGetOwnProcessHandle()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_own_process_handle() -> u32 {
    env::own_process_handle().map_or(INVALID_HANDLE, |h| h.to_raw())
}

/// Get exit function pointer.
///
/// Corresponds to `envGetExitFuncPtr()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_exit_func_ptr() -> LoaderReturnFn {
    env::exit_func_ptr()
}

/// Set exit function pointer.
///
/// Corresponds to `envSetExitFuncPtr()` in `env.h`.
///
/// # Safety
///
/// `func` must be a valid loader-return function pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_set_exit_func_ptr(func: LoaderReturnFn) {
    env::set_exit_func_ptr(func)
}

/// Set next NRO to load (chain loading).
///
/// Returns 0 on success, non-zero on error.
///
/// Corresponds to `envSetNextLoad()` in `env.h`.
///
/// # Safety
///
/// `path` must be a valid NUL-terminated C string, and `argv` must be one or
/// null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_set_next_load(
    path: *const c_char,
    argv: *const c_char,
) -> u32 {
    // Naming no program at all is not a request the loader can act on, and a
    // null command line is: the program is simply started without one.
    if path.is_null() {
        return libnx_error(LibnxError::BadInput);
    }

    // SAFETY: Caller guarantees a valid NUL-terminated C string for a non-null
    // pointer, and the strings outlive this call: `set_next_load` copies out of
    // them before returning.
    let path = unsafe { CStr::from_ptr(path) };
    let argv = (!argv.is_null()).then(|| unsafe { CStr::from_ptr(argv) });

    match env::set_next_load(path, argv) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Returns true if chain loading is supported.
///
/// Corresponds to `envHasNextLoad()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_has_next_load() -> bool {
    env::has_next_load()
}

/// Get last load result.
///
/// Corresponds to `envGetLastLoadResult()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_last_load_result() -> u32 {
    env::last_load_result()
}

/// Returns true if a random seed is present.
///
/// Corresponds to `envHasRandomSeed()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_has_random_seed() -> bool {
    env::random_seed().is_some()
}

/// Get random seed (copies to output buffer).
///
/// Corresponds to `envGetRandomSeed()` in `env.h`.
///
/// # Safety
///
/// `out`, when non-null, must be valid for writing two `u64` values. A null
/// `out` is tolerated and ignored.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_random_seed(out: *mut u64) {
    if out.is_null() {
        return;
    }
    if let Some([seed0, seed1]) = env::random_seed() {
        // SAFETY: Caller guarantees out points to a valid buffer with space for
        // 2 u64 values. We verified out is non-null above.
        unsafe {
            *out = seed0;
            *out.add(1) = seed1;
        }
    }
}

/// Get user ID storage pointer.
///
/// Corresponds to `envGetUserIdStorage()` in `env.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_env_get_user_id_storage() -> *mut AccountUid {
    match env::user_id_storage() {
        Some(ptr) => ptr.as_ptr(),
        None => core::ptr::null_mut(),
    }
}

/// Get the current HOS version (without the Atmosphere bit).
///
/// Corresponds to `hosversionGet()` in `hosversion.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_hosversion_get() -> u32 {
    env::hos_version::get().as_u32()
}

/// Set the HOS version.
///
/// This should only be called from `envSetup`/`appInit` in C code.
///
/// Corresponds to `hosversionSet()` in `hosversion.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. Intended to run only
/// during `envSetup` / `appInit`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_hosversion_set(version: u32) {
    env::hos_version::set(version)
}

/// Check if running on Atmosphere.
///
/// Corresponds to `hosversionIsAtmosphere()` in `hosversion.h`.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. No further precondition:
/// the runtime environment state is sound to read at any time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_hosversion_is_atmosphere() -> bool {
    env::hos_version::is_atmosphere()
}
