//! FFI exports for libnx runtime functions

use core::{
    cell::UnsafeCell,
    ffi::{c_char, c_uint, c_void},
    mem::MaybeUninit,
};

use nx_service_applet::aruid::NO_ARUID;
use nx_service_nv::fd::Fd;
use nx_sf::{ServiceName, cmif, service::Service, tipc};
use nx_svc::{
    error::ToRawResultCode, process::Handle as ProcessHandle, raw::INVALID_HANDLE,
    thread::Handle as ThreadHandle,
};

use crate::{
    apm_manager, applet_manager, argv,
    env::{self, AccountUid, ConfigEntry, LoaderReturnFn},
    init, service_manager, service_registry, vi_manager,
};

/// Generic error code for FFI when no specific result code is available.
const GENERIC_ERROR: u32 = 0xFFFF;

/// Static buffer for SM FFI session access. Updated on `initialize()` and `exit()`.
static SM_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for NV FFI session access. Updated on `nv_initialize()` and `nv_exit()`.
static NV_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI IApplicationDisplayService FFI session access.
static VI_FFI_APPLICATION_DISPLAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI IHOSBinderDriverRelay FFI session access.
static VI_FFI_BINDER_RELAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI ISystemDisplayService FFI session access.
static VI_FFI_SYSTEM_DISPLAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI IManagerDisplayService FFI session access.
static VI_FFI_MANAGER_DISPLAY: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Static buffer for VI IHOSBinderDriverIndirect FFI session access.
static VI_FFI_BINDER_INDIRECT: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

// libnx C symbols
unsafe extern "C" {
    /// Global applet type variable (libnx applet.c)
    static mut __nx_applet_type: u32;
}

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

    // SAFETY: Single-threaded initialization, exclusive access to the global variable.
    unsafe { __nx_applet_type = env::applet_type().as_raw() };
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
    env::argv().unwrap_or_else(|| core::ptr::null())
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
    env::set_next_load(path, argv)
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

/// Setup argv parsing.
///
/// Must be called after the global allocator is initialized.
///
/// Corresponds to `argvSetup()` in `argv.c`.
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

/// Initializes SM connection. Returns 0 on success, error code on failure.
///
/// Corresponds to `smInitialize()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_initialize() -> u32 {
    if let Err(err) = service_manager::initialize() {
        return sm_initialize_error_to_rc(err);
    }
    set_sm_ffi_session();
    0
}

/// Closes SM connection.
///
/// Corresponds to `smExit()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_exit() {
    service_manager::exit();
    clear_sm_ffi_session();
}

/// Gets a service with override support. Returns 0 on success, error code on failure.
///
/// Corresponds to `smGetServiceWrapper()` in `sm.h`.
///
/// # Safety
///
/// `service_out` must point to valid, writable memory for a Service struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_get_service_wrapper(
    service_out: *mut Service,
    name: ServiceName,
) -> u32 {
    if service_out.is_null() {
        return GENERIC_ERROR;
    }

    let srv = match service_manager::get_service(name) {
        Ok(srv) => srv,
        Err(err) => return sm_get_service_error_to_rc(err),
    };

    // SAFETY: Caller guarantees service_out points to valid memory.
    unsafe { *service_out = srv };
    0
}

/// Gets a service directly from SM. Returns 0 on success, error code on failure.
///
/// Corresponds to `smGetServiceOriginal()` in `sm.h`.
///
/// # Safety
///
/// `handle_out` must point to valid, writable memory for a u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_get_service_original(
    handle_out: *mut u32,
    name: ServiceName,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match service_manager::get_service_handle(name) {
        Ok(handle) => handle,
        Err(err) => return sm_get_service_error_to_rc(err),
    };

    // SAFETY: Caller guarantees handle_out points to valid memory.
    unsafe { *handle_out = handle.to_raw() };
    0
}

/// Gets an override handle for a service. Returns the handle or INVALID_HANDLE if none.
///
/// Corresponds to `smGetServiceOverride()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_get_service_override(name: ServiceName) -> u32 {
    service_manager::get_override(name)
        .map(|h| h.to_raw())
        .unwrap_or(INVALID_HANDLE)
}

/// Adds a service override.
///
/// Corresponds to `smAddOverrideHandle()` in `sm.h`.
///
/// # Safety
///
/// `handle` must be a valid handle value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_add_override_handle(name: ServiceName, handle: u32) {
    // SAFETY: Caller guarantees handle is valid.
    let handle = unsafe { nx_svc::ipc::Handle::from_raw(handle) };
    // Ignore error (matches libnx behavior)
    let _ = service_manager::add_override(name, handle);
}

/// Registers a service (auto-selects protocol). Returns 0 on success, error code on failure.
///
/// Corresponds to `smRegisterService()` in `sm.h`.
///
/// # Safety
///
/// `handle_out` must point to valid, writable memory for a u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_register_service(
    handle_out: *mut u32,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match service_manager::register_service(name, is_light, max_sessions) {
        Ok(handle) => handle,
        Err(err) => return sm_register_service_error_to_rc(err),
    };

    // SAFETY: Caller guarantees handle_out points to valid memory.
    unsafe { *handle_out = handle.to_raw() };
    0
}

/// Registers a service via CMIF. Returns 0 on success, error code on failure.
///
/// Corresponds to `smRegisterServiceCmif()` in `sm.h`.
///
/// # Safety
///
/// `handle_out` must point to valid, writable memory for a u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_register_service_cmif(
    handle_out: *mut u32,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match service_manager::register_service_cmif(name, is_light, max_sessions) {
        Ok(handle) => handle,
        Err(err) => return sm_register_service_error_to_rc(err),
    };

    // SAFETY: Caller guarantees handle_out points to valid memory.
    unsafe { *handle_out = handle.to_raw() };
    0
}

/// Registers a service via TIPC. Returns 0 on success, error code on failure.
///
/// Corresponds to `smRegisterServiceTipc()` in `sm.h`.
///
/// # Safety
///
/// `handle_out` must point to valid, writable memory for a u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_register_service_tipc(
    handle_out: *mut u32,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match service_manager::register_service_tipc(name, is_light, max_sessions) {
        Ok(handle) => handle,
        Err(err) => return sm_register_service_error_to_rc(err),
    };

    // SAFETY: Caller guarantees handle_out points to valid memory.
    unsafe { *handle_out = handle.to_raw() };
    0
}

/// Unregisters a service (auto-selects protocol). Returns 0 on success, error code on failure.
///
/// Corresponds to `smUnregisterService()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_unregister_service(name: ServiceName) -> u32 {
    if let Err(err) = service_manager::unregister_service(name) {
        return sm_unregister_service_error_to_rc(err);
    }

    0
}

/// Unregisters a service via CMIF. Returns 0 on success, error code on failure.
///
/// Corresponds to `smUnregisterServiceCmif()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_unregister_service_cmif(name: ServiceName) -> u32 {
    if let Err(err) = service_manager::unregister_service_cmif(name) {
        return sm_unregister_service_error_to_rc(err);
    }

    0
}

/// Unregisters a service via TIPC. Returns 0 on success, error code on failure.
///
/// Corresponds to `smUnregisterServiceTipc()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_unregister_service_tipc(name: ServiceName) -> u32 {
    if let Err(err) = service_manager::unregister_service_tipc(name) {
        return sm_unregister_service_error_to_rc(err);
    }

    0
}

/// Detaches the client (auto-selects protocol). Returns 0 on success, error code on failure.
///
/// Corresponds to `smDetachClient()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_detach_client() -> u32 {
    if let Err(err) = service_manager::detach_client() {
        return sm_detach_client_error_to_rc(err);
    }

    0
}

/// Detaches via CMIF. Returns 0 on success, error code on failure.
///
/// Corresponds to `smDetachClientCmif()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_detach_client_cmif() -> u32 {
    if let Err(err) = service_manager::detach_client_cmif() {
        return sm_detach_client_error_to_rc(err);
    }

    0
}

/// Detaches via TIPC. Returns 0 on success, error code on failure.
///
/// Corresponds to `smDetachClientTipc()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_detach_client_tipc() -> u32 {
    if let Err(err) = service_manager::detach_client_tipc() {
        return sm_detach_client_error_to_rc(err);
    }

    0
}

/// Gets the SM service session pointer.
///
/// Corresponds to `smGetServiceSession()` in `sm.h`.
///
/// # Safety
///
/// SM must be initialized. The returned pointer points to a static buffer
/// that is updated on initialization and cleared on exit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__sm_get_service_session() -> *mut Service {
    SM_FFI_SESSION.get().cast::<Service>()
}

/// Initializes set:sys connection. Returns 0 on success, error code on failure.
///
/// Corresponds to `setsysInitialize()` in `set.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_initialize() -> u32 {
    if let Err(err) = service_registry::setsys_init() {
        return setsys_connect_error_to_rc(err);
    }
    0
}

/// Closes set:sys connection.
///
/// Corresponds to `setsysExit()` in `set.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_exit() {
    service_registry::setsys_exit();
}

/// Gets the set:sys service session pointer.
///
/// Corresponds to `setsysGetServiceSession()` in `set.h`.
///
/// # Safety
///
/// set:sys must be initialized. The returned pointer points to the Arc-stored
/// session and remains valid as long as the service session is alive.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_get_service_session() -> *mut Service {
    let Some(setsys) = service_registry::setsys_get() else {
        return core::ptr::null_mut();
    };

    // Cast Arc<SetSysService> to *mut Service (SetSysService is repr(transparent))
    alloc::sync::Arc::as_ptr(&setsys) as *mut Service
}

/// Gets the system firmware version. Returns 0 on success, error code on failure.
///
/// Corresponds to `setsysGetFirmwareVersion()` in `set.h`.
///
/// # Safety
///
/// `out` must point to valid, writable memory for a FirmwareVersion struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__setsys_get_firmware_version(
    out: *mut nx_service_set::FirmwareVersion,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(setsys) = service_registry::setsys_get() else {
        return GENERIC_ERROR;
    };

    let fw = match setsys.get_firmware_version_cmif() {
        Ok(fw) => fw,
        Err(err) => return setsys_get_firmware_version_error_to_rc(err),
    };

    // SAFETY: Caller guarantees out points to valid memory.
    unsafe { *out = fw };
    0
}

/// Initializes the APM service. Returns 0 on success, error code on failure.
///
/// Corresponds to `apmInitialize()` in `apm.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_initialize() -> u32 {
    if let Err(err) = apm_manager::init() {
        return apm_connect_error_to_rc(err);
    }
    0
}

/// Closes the APM service connection.
///
/// Corresponds to `apmExit()` in `apm.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_exit() {
    apm_manager::exit();
}

/// Gets the current performance mode.
///
/// Corresponds to `apmGetPerformanceMode()` in `apm.h`.
///
/// # Safety
///
/// Caller guarantees out points to valid i32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_performance_mode(out: *mut i32) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = apm_manager::get_service() else {
        return GENERIC_ERROR;
    };

    match service.get_performance_mode() {
        Ok(mode) => {
            // SAFETY: Caller guarantees out points to valid memory.
            unsafe { *out = mode as i32 };
            0
        }
        Err(err) => apm_get_performance_mode_error_to_rc(err),
    }
}

/// Sets the performance configuration for a mode.
///
/// Corresponds to `apmSetPerformanceConfiguration()` in `apm.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_set_performance_configuration(mode: i32, config: u32) -> u32 {
    let Some(perf_mode) = nx_service_apm::PerformanceMode::from_raw(mode) else {
        return GENERIC_ERROR;
    };

    let Some(session) = apm_manager::get_session() else {
        return GENERIC_ERROR;
    };

    match session.set_performance_configuration(perf_mode, config) {
        Ok(()) => 0,
        Err(err) => apm_set_performance_configuration_error_to_rc(err),
    }
}

/// Gets the performance configuration for a mode.
///
/// Corresponds to `apmGetPerformanceConfiguration()` in `apm.h`.
///
/// # Safety
///
/// Caller guarantees out points to valid u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_performance_configuration(
    mode: i32,
    out: *mut u32,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(perf_mode) = nx_service_apm::PerformanceMode::from_raw(mode) else {
        return GENERIC_ERROR;
    };

    let Some(session) = apm_manager::get_session() else {
        return GENERIC_ERROR;
    };

    match session.get_performance_configuration(perf_mode) {
        Ok(config) => {
            // SAFETY: Caller guarantees out points to valid memory.
            unsafe { *out = config };
            0
        }
        Err(err) => apm_get_performance_configuration_error_to_rc(err),
    }
}

/// Gets the APM service session for C interop.
///
/// Corresponds to `apmGetServiceSession()` in `apm.h`.
///
/// # Safety
///
/// Returns a pointer to the service session or null if not initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_service_session() -> *mut Service {
    let Some(service) = apm_manager::get_service() else {
        return core::ptr::null_mut();
    };

    // SAFETY: Session handle is valid while service is initialized.
    // Lifetime safety guaranteed by singleton management.
    unsafe { core::mem::transmute(&*service as *const _ as *mut Service) }
}

/// Gets the APM ISession for C interop.
///
/// Corresponds to `apmGetServiceSession_Session()` in `apm.h`.
///
/// # Safety
///
/// Returns a pointer to the session or null if not initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__apm_get_service_session_session() -> *mut Service {
    let Some(session) = apm_manager::get_session() else {
        return core::ptr::null_mut();
    };

    // SAFETY: Session handle is valid while APM is initialized.
    // Lifetime safety guaranteed by singleton management.
    unsafe { core::mem::transmute(&*session as *const _ as *mut Service) }
}

/// Initializes the applet service. Returns 0 on success, error code on failure.
///
/// Corresponds to `appletInitialize()` in `applet.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_initialize() -> u32 {
    // Get applet type from global variable (set by envSetup)
    let raw_type = unsafe { __nx_applet_type };

    let Some(applet_type) = nx_service_applet::AppletType::from_raw(raw_type as i32) else {
        return GENERIC_ERROR;
    };

    // Skip initialization for AppletType::None
    if matches!(applet_type, nx_service_applet::AppletType::None) {
        return 0;
    }

    // Resolve Default to Application
    let applet_type = if matches!(applet_type, nx_service_applet::AppletType::Default) {
        nx_service_applet::AppletType::Application
    } else {
        applet_type
    };

    // Get process handle
    let process_handle = env::own_process_handle()
        .map(|h| {
            // SAFETY: Handle from env::own_process_handle() is guaranteed valid.
            unsafe { ProcessHandle::from_raw(h.to_raw()) }
        })
        .unwrap_or_else(ProcessHandle::current_process);

    if let Err(err) = applet_manager::init(applet_type, process_handle) {
        return applet_connect_error_to_rc(err);
    }

    0
}

/// Closes the applet service connection.
///
/// Corresponds to `appletExit()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_exit() {
    applet_manager::exit();
}

/// Gets the current applet type.
///
/// Corresponds to `appletGetAppletType()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_applet_type() -> i32 {
    // Return from the global variable
    unsafe { __nx_applet_type as i32 }
}

/// Gets the current operation mode (handheld/docked).
///
/// Corresponds to `appletGetOperationMode()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_operation_mode() -> u8 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return nx_service_applet::AppletOperationMode::Handheld as u8;
    };

    csg.get_operation_mode()
        .unwrap_or(nx_service_applet::AppletOperationMode::Handheld) as u8
}

/// Gets the current performance mode.
///
/// Corresponds to `appletGetPerformanceMode()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_performance_mode() -> u32 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return 0;
    };

    csg.get_performance_mode().unwrap_or(0)
}

/// Gets the current focus state.
///
/// Corresponds to `appletGetFocusState()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_focus_state() -> u8 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return nx_service_applet::AppletFocusState::InFocus as u8;
    };

    csg.get_current_focus_state()
        .map(|s| s as u8)
        .unwrap_or(nx_service_applet::AppletFocusState::InFocus as u8)
}

/// Gets the message event handle.
///
/// Corresponds to part of `appletGetMessageEventHandle()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_message_event_handle() -> u32 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return INVALID_HANDLE;
    };

    match csg.get_event_handle() {
        Ok(handle) => handle.to_raw(),
        Err(_) => INVALID_HANDLE,
    }
}

/// Sets the focus handling mode.
///
/// Corresponds to `appletSetFocusHandlingMode()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_set_focus_handling_mode(mode: u32) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
        return GENERIC_ERROR;
    };

    let mode = match mode {
        0 => nx_service_applet::AppletFocusHandlingMode::SuspendHomeSleep,
        1 => nx_service_applet::AppletFocusHandlingMode::NoSuspend,
        2 => nx_service_applet::AppletFocusHandlingMode::SuspendHomeSleepNotify,
        3 => nx_service_applet::AppletFocusHandlingMode::AlwaysSuspend,
        _ => return GENERIC_ERROR,
    };

    if let Err(nx_service_applet::SetFocusHandlingModeError::Dispatch(e)) =
        sc.set_focus_handling_mode(mode)
    {
        return dispatch_error_to_rc(e);
    }

    0
}

/// Sets whether to suspend when out of focus.
///
/// Corresponds to `appletSetOutOfFocusSuspendingEnabled()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_set_out_of_focus_suspending_enabled(enabled: bool) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(nx_service_applet::SetOutOfFocusSuspendingEnabledError::Dispatch(e)) =
        sc.set_out_of_focus_suspending_enabled(enabled)
    {
        return dispatch_error_to_rc(e);
    }

    0
}

/// Receives a message from the applet message queue.
///
/// Corresponds to `appletReceiveMessage()` in `applet.h`.
///
/// # Safety
///
/// `msg` must point to valid, writable memory for a u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_receive_message(msg: *mut u32) -> u32 {
    if msg.is_null() {
        return GENERIC_ERROR;
    }

    let Some(csg) = applet_manager::get_common_state_getter() else {
        return GENERIC_ERROR;
    };

    match csg.receive_message() {
        Ok(Some(message)) => {
            // SAFETY: Caller guarantees msg points to valid memory.
            unsafe { *msg = message as u32 };
            0
        }
        Ok(None) => {
            // No message available - return 0 with no message written
            // This matches libnx behavior where the queue may be empty
            0
        }
        Err(nx_service_applet::ReceiveMessageError::Dispatch(e)) => dispatch_error_to_rc(e),
        Err(nx_service_applet::ReceiveMessageError::InvalidResponse) => GENERIC_ERROR,
    }
}

/// Sets operation mode change notification.
///
/// Corresponds to `appletSetOperationModeChangedNotification()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_set_operation_mode_changed_notification(
    enabled: bool,
) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(nx_service_applet::SetOperationModeChangedNotificationError::Dispatch(e)) =
        sc.set_operation_mode_changed_notification(enabled)
    {
        return dispatch_error_to_rc(e);
    }

    0
}

/// Sets performance mode change notification.
///
/// Corresponds to `appletSetPerformanceModeChangedNotification()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_set_performance_mode_changed_notification(
    enabled: bool,
) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(nx_service_applet::SetPerformanceModeChangedNotificationError::Dispatch(e)) =
        sc.set_performance_mode_changed_notification(enabled)
    {
        return dispatch_error_to_rc(e);
    }

    0
}

/// Gets the applet resource user ID.
///
/// Corresponds to `appletGetAppletResourceUserId()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_applet_resource_user_id() -> u64 {
    applet_manager::get_applet_resource_user_id()
        .map(|a| a.to_raw())
        .unwrap_or(NO_ARUID)
}

/// Acquires foreground rights.
///
/// Corresponds to `appletAcquireForegroundRights()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_acquire_foreground_rights() -> u32 {
    let Some(wc) = applet_manager::get_window_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(nx_service_applet::AcquireForegroundRightsError::Dispatch(e)) =
        wc.acquire_foreground_rights()
    {
        return dispatch_error_to_rc(e);
    }

    0
}

/// Creates a managed display layer.
///
/// Corresponds to `appletCreateManagedDisplayLayer()` in `applet.h`.
///
/// # Safety
///
/// `out` must be a valid pointer to write the layer ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_create_managed_display_layer(out: *mut u64) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
        return GENERIC_ERROR;
    };

    match sc.create_managed_display_layer() {
        Ok(layer_id) => {
            if !out.is_null() {
                unsafe { *out = layer_id };
            }
            0
        }
        Err(_) => GENERIC_ERROR,
    }
}

fn sm_initialize_error_to_rc(err: service_manager::InitializeError) -> u32 {
    match err.0 {
        nx_service_sm::ConnectError::Connect(e) => e.to_rc(),
        nx_service_sm::ConnectError::RegisterClient(e) => match e {
            nx_service_sm::RegisterClientCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::RegisterClientCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn sm_get_service_error_to_rc(err: service_manager::GetServiceError) -> u32 {
    match err.0 {
        nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
        nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
    }
}

fn sm_register_service_error_to_rc(err: service_manager::RegisterServiceError) -> u32 {
    match err {
        service_manager::RegisterServiceError::Cmif(e) => match e {
            nx_service_sm::RegisterServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::RegisterServiceCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::RegisterServiceCmifError::MissingHandle => GENERIC_ERROR,
        },
        service_manager::RegisterServiceError::Tipc(e) => match e {
            nx_service_sm::RegisterServiceTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::RegisterServiceTipcError::ParseResponse(e) => match e {
                tipc::ParseResponseError::EmptyResponse => GENERIC_ERROR,
                tipc::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::RegisterServiceTipcError::MissingHandle => GENERIC_ERROR,
        },
    }
}

fn sm_unregister_service_error_to_rc(err: service_manager::UnregisterServiceError) -> u32 {
    match err {
        service_manager::UnregisterServiceError::Cmif(e) => match e {
            nx_service_sm::UnregisterServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::UnregisterServiceCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
        service_manager::UnregisterServiceError::Tipc(e) => match e {
            nx_service_sm::UnregisterServiceTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::UnregisterServiceTipcError::ParseResponse(e) => match e {
                tipc::ParseResponseError::EmptyResponse => GENERIC_ERROR,
                tipc::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn sm_detach_client_error_to_rc(err: service_manager::DetachClientError) -> u32 {
    match err {
        // LibnxError_IncompatSysVer = 0x64 (100) in module 345
        // MAKERESULT(Module_Libnx, LibnxError_IncompatSysVer) = 0x8A564
        service_manager::DetachClientError::IncompatibleVersion => 0x8A564,
        service_manager::DetachClientError::Cmif(e) => match e {
            nx_service_sm::DetachClientCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::DetachClientCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
        service_manager::DetachClientError::Tipc(e) => match e {
            nx_service_sm::DetachClientTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::DetachClientTipcError::ParseResponse(e) => match e {
                tipc::ParseResponseError::EmptyResponse => GENERIC_ERROR,
                tipc::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn setsys_connect_error_to_rc(err: service_registry::SetsysConnectError) -> u32 {
    match err {
        service_registry::SetsysConnectError::Cmif(e) => match e.0 {
            nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
        },
        service_registry::SetsysConnectError::Tipc(e) => match e.0 {
            nx_service_sm::GetServiceTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::GetServiceTipcError::ParseResponse(e) => match e {
                tipc::ParseResponseError::EmptyResponse => GENERIC_ERROR,
                tipc::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::GetServiceTipcError::MissingHandle => GENERIC_ERROR,
        },
    }
}

fn setsys_get_firmware_version_error_to_rc(
    err: nx_service_set::GetFirmwareVersionCmifError,
) -> u32 {
    match err {
        nx_service_set::GetFirmwareVersionCmifError::SendRequest(e) => e.to_rc(),
        nx_service_set::GetFirmwareVersionCmifError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn apm_connect_error_to_rc(err: apm_manager::ConnectError) -> u32 {
    match err {
        apm_manager::ConnectError::Connect(e) => match e {
            nx_service_apm::ConnectError::GetService(e) => match e {
                nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
                nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
            },
        },
        apm_manager::ConnectError::OpenSession(e) => match e {
            nx_service_apm::OpenSessionError::SendRequest(e) => e.to_rc(),
            nx_service_apm::OpenSessionError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_apm::OpenSessionError::MissingHandle => GENERIC_ERROR,
        },
    }
}

fn apm_get_performance_mode_error_to_rc(err: nx_service_apm::GetPerformanceModeError) -> u32 {
    match err {
        nx_service_apm::GetPerformanceModeError::SendRequest(e) => e.to_rc(),
        nx_service_apm::GetPerformanceModeError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_apm::GetPerformanceModeError::InvalidResponse => GENERIC_ERROR,
        nx_service_apm::GetPerformanceModeError::InvalidMode(_) => GENERIC_ERROR,
    }
}

fn apm_set_performance_configuration_error_to_rc(
    err: nx_service_apm::SetPerformanceConfigurationError,
) -> u32 {
    match err {
        nx_service_apm::SetPerformanceConfigurationError::SendRequest(e) => e.to_rc(),
        nx_service_apm::SetPerformanceConfigurationError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn apm_get_performance_configuration_error_to_rc(
    err: nx_service_apm::GetPerformanceConfigurationError,
) -> u32 {
    match err {
        nx_service_apm::GetPerformanceConfigurationError::SendRequest(e) => e.to_rc(),
        nx_service_apm::GetPerformanceConfigurationError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_apm::GetPerformanceConfigurationError::InvalidResponse => GENERIC_ERROR,
    }
}

/// Converts a `DispatchError` to a raw result code.
fn dispatch_error_to_rc(err: nx_sf::service::DispatchError) -> u32 {
    match err {
        nx_sf::service::DispatchError::SendRequest(e) => e.to_rc(),
        nx_sf::service::DispatchError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

/// Converts a `ConvertToDomainError` to a raw result code.
fn convert_to_domain_error_to_rc(err: nx_sf::service::ConvertToDomainError) -> u32 {
    match err {
        nx_sf::service::ConvertToDomainError::SendRequest(e) => e.to_rc(),
        nx_sf::service::ConvertToDomainError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn applet_connect_error_to_rc(err: applet_manager::ConnectError) -> u32 {
    match err {
        applet_manager::ConnectError::Connect(e) => match e {
            nx_service_applet::ConnectError::GetService(e) => match e {
                nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
                nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
            },
            nx_service_applet::ConnectError::ConvertToDomain(e) => {
                convert_to_domain_error_to_rc(e.0)
            }
        },
        applet_manager::ConnectError::OpenProxy(e) => match e {
            nx_service_applet::OpenProxyError::InvalidAppletType => GENERIC_ERROR,
            nx_service_applet::OpenProxyError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::OpenProxyError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetCommonStateGetter(e) => match e {
            nx_service_applet::GetCommonStateGetterError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetCommonStateGetterError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetSelfController(e) => match e {
            nx_service_applet::GetSelfControllerError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetSelfControllerError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetWindowController(e) => match e {
            nx_service_applet::GetWindowControllerError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetWindowControllerError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetApplicationFunctions(e) => match e {
            nx_service_applet::GetApplicationFunctionsError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetApplicationFunctionsError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetEventHandle(e) => match e {
            nx_service_applet::GetEventHandleError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetEventHandleError::MissingHandle => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetFocusState(e) => match e {
            nx_service_applet::GetCurrentFocusStateError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetCurrentFocusStateError::InvalidResponse => GENERIC_ERROR,
            nx_service_applet::GetCurrentFocusStateError::InvalidValue(_) => GENERIC_ERROR,
        },
        applet_manager::ConnectError::WaitSynchronization(e) => e.to_rc(),
        applet_manager::ConnectError::AcquireForegroundRights(e) => match e {
            nx_service_applet::AcquireForegroundRightsError::Dispatch(e) => dispatch_error_to_rc(e),
        },
        applet_manager::ConnectError::SetFocusHandlingMode(e) => match e {
            nx_service_applet::SetFocusHandlingModeError::Dispatch(e) => dispatch_error_to_rc(e),
        },
        applet_manager::ConnectError::NotifyRunning(e) => match e {
            nx_service_applet::NotifyRunningError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::NotifyRunningError::InvalidResponse => GENERIC_ERROR,
        },
        applet_manager::ConnectError::SetOperationModeNotification(e) => match e {
            nx_service_applet::SetOperationModeChangedNotificationError::Dispatch(e) => {
                dispatch_error_to_rc(e)
            }
        },
        applet_manager::ConnectError::SetPerformanceModeNotification(e) => match e {
            nx_service_applet::SetPerformanceModeChangedNotificationError::Dispatch(e) => {
                dispatch_error_to_rc(e)
            }
        },
    }
}

fn set_sm_ffi_session() {
    let guard = service_manager::sm_session();
    if let Some(sm) = guard.as_ref() {
        // Copy the underlying Service to FFI buffer
        // SAFETY: Called only during initialization, single-threaded access.
        unsafe {
            let service = Service {
                session: sm.session(),
                own_handle: 1,
                object_id: 0,
                pointer_buffer_size: 0,
            };
            SM_FFI_SESSION.get().cast::<Service>().write(service);
        }
    }
}

fn clear_sm_ffi_session() {
    // SAFETY: Called only during exit.
    unsafe {
        SM_FFI_SESSION.get().write(MaybeUninit::zeroed());
    }
}

//==============================================================================
// HID service FFI exports
//==============================================================================

/// Initializes the HID service.
///
/// Corresponds to `hidInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize() -> u32 {
    match crate::hid_manager::init() {
        Ok(()) => 0,
        Err(err) => {
            // Convert error to result code
            match err {
                crate::hid_manager::ConnectError::Connect(conn_err) => match conn_err {
                    nx_service_hid::ConnectError::GetService(sm_err) => match sm_err {
                        nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                        _ => GENERIC_ERROR,
                    },
                    nx_service_hid::ConnectError::CreateAppletResource(ar_err) => match ar_err {
                        nx_service_hid::CreateAppletResourceError::SendRequest(e) => e.to_rc(),
                        nx_service_hid::CreateAppletResourceError::ParseResponse(e) => match e {
                            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                            cmif::ParseResponseError::ServiceError(code) => code,
                        },
                        nx_service_hid::CreateAppletResourceError::MissingHandle => GENERIC_ERROR,
                    },
                    nx_service_hid::ConnectError::GetSharedMemoryHandle(sh_err) => match sh_err {
                        nx_service_hid::GetSharedMemoryHandleError::SendRequest(e) => e.to_rc(),
                        nx_service_hid::GetSharedMemoryHandleError::ParseResponse(e) => match e {
                            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                            cmif::ParseResponseError::ServiceError(code) => code,
                        },
                        nx_service_hid::GetSharedMemoryHandleError::MissingHandle => GENERIC_ERROR,
                    },
                    nx_service_hid::ConnectError::MapSharedMemory(_) => GENERIC_ERROR,
                    nx_service_hid::ConnectError::NullPointer => GENERIC_ERROR,
                },
            }
        }
    }
}

/// Exits the HID service.
///
/// Corresponds to `hidExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_exit() {
    crate::hid_manager::exit();
}

/// Gets the shared memory address for HID.
///
/// Corresponds to `hidGetSharedmemAddr()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_get_sharedmem_addr() -> *const c_void {
    match crate::hid_manager::get_service() {
        Some(service) => service.shared_memory() as *const _ as *const c_void,
        None => core::ptr::null(),
    }
}

/// Initializes Npad (controller) support.
///
/// Corresponds to `hidInitializeNpad()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_npad() {
    if let Some(service) = crate::hid_manager::get_service() {
        // Ignore errors - libnx diagAborts on failure, but we'll just return
        let _ = service.activate_npad();
    }
}

/// Sets the supported Npad style set.
///
/// Corresponds to `hidSetSupportedNpadStyleSet()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_set_supported_npad_style_set(style_set: u32) -> u32 {
    match crate::hid_manager::get_service() {
        Some(service) => match service.set_supported_npad_style_set(style_set) {
            Ok(()) => 0,
            Err(err) => match err {
                nx_service_hid::SetSupportedNpadStyleSetError::SendRequest(e) => e.to_rc(),
                nx_service_hid::SetSupportedNpadStyleSetError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
            },
        },
        None => GENERIC_ERROR,
    }
}

/// Sets the supported Npad ID types.
///
/// Corresponds to `hidSetSupportedNpadIdType()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_set_supported_npad_id_type(
    ids: *const u32,
    count: usize,
) -> u32 {
    if ids.is_null() {
        return GENERIC_ERROR;
    }

    // SAFETY: Caller guarantees ids points to a valid array of count elements.
    let ids_slice = unsafe { core::slice::from_raw_parts(ids, count) };

    match crate::hid_manager::get_service() {
        Some(service) => match service.set_supported_npad_id_type(ids_slice) {
            Ok(()) => 0,
            Err(err) => match err {
                nx_service_hid::SetSupportedNpadIdTypeError::SendRequest(e) => e.to_rc(),
                nx_service_hid::SetSupportedNpadIdTypeError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
            },
        },
        None => GENERIC_ERROR,
    }
}

/// Initializes touch screen support.
///
/// Corresponds to `hidInitializeTouchScreen()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_touch_screen() {
    if let Some(service) = crate::hid_manager::get_service() {
        let _ = service.activate_touch_screen();
    }
}

/// Initializes mouse support.
///
/// Corresponds to `hidInitializeMouse()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_mouse() {
    if let Some(service) = crate::hid_manager::get_service() {
        let _ = service.activate_mouse();
    }
}

/// Initializes keyboard support.
///
/// Corresponds to `hidInitializeKeyboard()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_keyboard() {
    if let Some(service) = crate::hid_manager::get_service() {
        let _ = service.activate_keyboard();
    }
}

/// Initializes gesture recognition support.
///
/// This is not in libnx but provides access to the activate_gesture command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_gesture() {
    if let Some(service) = crate::hid_manager::get_service() {
        let _ = service.activate_gesture();
    }
}

// =============================================================================
// Time service FFI
// =============================================================================

/// Initializes the Time service.
///
/// Corresponds to `timeInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__time_initialize() -> u32 {
    match crate::time_manager::init() {
        Ok(()) => 0,
        Err(err) => {
            // Convert error to result code
            match err {
                crate::time_manager::ConnectError::Connect(conn_err) => match conn_err {
                    nx_service_time::ConnectError::GetService(sm_err) => match sm_err {
                        nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                        _ => GENERIC_ERROR,
                    },
                    nx_service_time::ConnectError::GetUserSystemClock(clock_err) => match clock_err
                    {
                        nx_service_time::GetSystemClockError::SendRequest(e) => e.to_rc(),
                        nx_service_time::GetSystemClockError::ParseResponse(e) => match e {
                            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                            cmif::ParseResponseError::ServiceError(code) => code,
                        },
                        nx_service_time::GetSystemClockError::MissingHandle => GENERIC_ERROR,
                    },
                    nx_service_time::ConnectError::GetSteadyClock(steady_err) => match steady_err {
                        nx_service_time::GetSteadyClockError::SendRequest(e) => e.to_rc(),
                        nx_service_time::GetSteadyClockError::ParseResponse(e) => match e {
                            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                            cmif::ParseResponseError::ServiceError(code) => code,
                        },
                        nx_service_time::GetSteadyClockError::MissingHandle => GENERIC_ERROR,
                    },
                    nx_service_time::ConnectError::GetTimeZoneService(tz_err) => match tz_err {
                        nx_service_time::GetTimeZoneServiceError::SendRequest(e) => e.to_rc(),
                        nx_service_time::GetTimeZoneServiceError::ParseResponse(e) => match e {
                            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                            cmif::ParseResponseError::ServiceError(code) => code,
                        },
                        nx_service_time::GetTimeZoneServiceError::MissingHandle => GENERIC_ERROR,
                    },
                },
            }
        }
    }
}

/// Exits the Time service.
///
/// Corresponds to `timeExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__time_exit() {
    crate::time_manager::exit();
}

/// Gets the current time from the specified clock type.
///
/// Corresponds to `timeGetCurrentTime()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__time_get_current_time(
    clock_type: u32,
    timestamp: *mut u64,
) -> u32 {
    if timestamp.is_null() {
        return GENERIC_ERROR;
    }

    let time_type = match clock_type {
        0 => nx_service_time::TimeType::UserSystemClock,
        1 => nx_service_time::TimeType::NetworkSystemClock,
        2 => nx_service_time::TimeType::LocalSystemClock,
        _ => return GENERIC_ERROR,
    };

    match crate::time_manager::get_service() {
        Some(service) => match service.get_current_time(time_type) {
            Ok(time) => {
                *timestamp = time;
                0
            }
            Err(err) => match err {
                nx_service_time::GetCurrentTimeError::SendRequest(e) => e.to_rc(),
                nx_service_time::GetCurrentTimeError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
                nx_service_time::GetCurrentTimeError::NetworkClockUnavailable => GENERIC_ERROR,
                nx_service_time::GetCurrentTimeError::LocalClockNotSupported => GENERIC_ERROR,
                nx_service_time::GetCurrentTimeError::SourceIdMismatch => GENERIC_ERROR,
            },
        },
        None => GENERIC_ERROR,
    }
}

/// Converts a POSIX timestamp to calendar time using the device's timezone.
///
/// Corresponds to `timeToCalendarTimeWithMyRule()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__time_to_calendar_time_with_my_rule(
    timestamp: u64,
    caltime: *mut nx_service_time::TimeCalendarTime,
    info: *mut nx_service_time::TimeCalendarAdditionalInfo,
) -> u32 {
    if caltime.is_null() || info.is_null() {
        return GENERIC_ERROR;
    }

    match crate::time_manager::get_service() {
        Some(service) => match service.to_calendar_time_with_my_rule(timestamp) {
            Ok((cal, inf)) => {
                *caltime = cal;
                *info = inf;
                0
            }
            Err(err) => match err {
                nx_service_time::ToCalendarTimeError::SendRequest(e) => e.to_rc(),
                nx_service_time::ToCalendarTimeError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
            },
        },
        None => GENERIC_ERROR,
    }
}

// =============================================================================
// NV (NVIDIA Driver) service FFI
// =============================================================================

/// Initializes the NV service.
///
/// Corresponds to `nvInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_initialize() -> u32 {
    // Build config from global settings
    let config = crate::nv_manager::make_config();

    // Check if this is the first initialization
    let was_initialized = crate::nv_manager::is_initialized();

    match crate::nv_manager::init(config) {
        Ok(()) => {
            // Only update FFI session buffer on first actual initialization
            if !was_initialized {
                if let Some(service_ref) = crate::nv_manager::get_service() {
                    let service = Service {
                        session: service_ref.session(),
                        own_handle: 1,
                        object_id: 0,
                        pointer_buffer_size: 0,
                    };
                    // SAFETY: Called only during first initialization.
                    unsafe { NV_FFI_SESSION.get().cast::<Service>().write(service) };
                }
            }
            0
        }
        Err(err) => nv_connect_error_to_rc(err),
    }
}

/// Exits the NV service.
///
/// Corresponds to `nvExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_exit() {
    // Check if this exit will actually close the service (ref_count will become 0)
    // We need to clear the FFI session AFTER the service is closed, not before
    let was_initialized = crate::nv_manager::is_initialized();
    crate::nv_manager::exit();
    let still_initialized = crate::nv_manager::is_initialized();

    // Only clear the FFI session buffer if the service was actually closed
    if was_initialized && !still_initialized {
        // SAFETY: Called only during exit, after service is closed.
        unsafe { NV_FFI_SESSION.get().write(MaybeUninit::zeroed()) };
    }
}

/// Opens an NV device by path.
///
/// Corresponds to `nvOpen()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_open(fd: *mut u32, devicepath: *const c_char) -> u32 {
    if fd.is_null() || devicepath.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = crate::nv_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // Convert C string to Rust string
    let path_cstr = unsafe { core::ffi::CStr::from_ptr(devicepath) };
    let path_str = match path_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return GENERIC_ERROR,
    };

    match service.open(path_str) {
        Ok(opened_fd) => {
            unsafe { *fd = opened_fd.to_raw() };
            0
        }
        Err(err) => nv_open_error_to_rc(err),
    }
}

/// Performs an ioctl operation.
///
/// Corresponds to `nvIoctl()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_ioctl(fd: u32, request: u32, argp: *mut c_void) -> u32 {
    let Some(service) = crate::nv_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let bufsize = nx_service_nv::nv_ioc_size(request);
    if argp.is_null() && bufsize > 0 {
        return GENERIC_ERROR;
    }

    // SAFETY: Caller guarantees argp points to valid buffer of at least bufsize bytes.
    let argp_slice = if bufsize > 0 {
        unsafe { core::slice::from_raw_parts_mut(argp as *mut u8, bufsize) }
    } else {
        &mut []
    };

    // SAFETY: fd is provided by the C caller who obtained it from nvOpen.
    match service.ioctl(unsafe { Fd::new_unchecked(fd) }, request, argp_slice) {
        Ok(()) => 0,
        Err(err) => nv_ioctl_error_to_rc(err),
    }
}

/// Performs an ioctl2 operation with extra input buffer.
///
/// Corresponds to `nvIoctl2()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_ioctl2(
    fd: u32,
    request: u32,
    argp: *mut c_void,
    inbuf: *const c_void,
    inbuf_size: usize,
) -> u32 {
    let Some(service) = crate::nv_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let bufsize = nx_service_nv::nv_ioc_size(request);
    if argp.is_null() && bufsize > 0 {
        return GENERIC_ERROR;
    }
    if inbuf.is_null() && inbuf_size > 0 {
        return GENERIC_ERROR;
    }

    // SAFETY: Caller guarantees buffers point to valid memory.
    let argp_slice = if bufsize > 0 {
        unsafe { core::slice::from_raw_parts_mut(argp as *mut u8, bufsize) }
    } else {
        &mut []
    };

    let inbuf_slice = if inbuf_size > 0 {
        unsafe { core::slice::from_raw_parts(inbuf as *const u8, inbuf_size) }
    } else {
        &[]
    };

    // SAFETY: fd is provided by the C caller who obtained it from nvOpen.
    match service.ioctl2(
        unsafe { Fd::new_unchecked(fd) },
        request,
        argp_slice,
        inbuf_slice,
    ) {
        Ok(()) => 0,
        Err(err) => nv_ioctl2_error_to_rc(err),
    }
}

/// Performs an ioctl3 operation with extra output buffer.
///
/// Corresponds to `nvIoctl3()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_ioctl3(
    fd: u32,
    request: u32,
    argp: *mut c_void,
    outbuf: *mut c_void,
    outbuf_size: usize,
) -> u32 {
    let Some(service) = crate::nv_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let bufsize = nx_service_nv::nv_ioc_size(request);
    if argp.is_null() && bufsize > 0 {
        return GENERIC_ERROR;
    }
    if outbuf.is_null() && outbuf_size > 0 {
        return GENERIC_ERROR;
    }

    // SAFETY: Caller guarantees buffers point to valid memory.
    let argp_slice = if bufsize > 0 {
        unsafe { core::slice::from_raw_parts_mut(argp as *mut u8, bufsize) }
    } else {
        &mut []
    };

    let outbuf_slice = if outbuf_size > 0 {
        unsafe { core::slice::from_raw_parts_mut(outbuf as *mut u8, outbuf_size) }
    } else {
        &mut []
    };

    // SAFETY: fd is provided by the C caller who obtained it from nvOpen.
    match service.ioctl3(
        unsafe { Fd::new_unchecked(fd) },
        request,
        argp_slice,
        outbuf_slice,
    ) {
        Ok(()) => 0,
        Err(err) => nv_ioctl3_error_to_rc(err),
    }
}

/// Closes an NV device.
///
/// Corresponds to `nvClose()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_close(fd: u32) -> u32 {
    let Some(service) = crate::nv_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // SAFETY: fd is provided by the C caller who obtained it from nvOpen.
    match service.close_fd(unsafe { Fd::new_unchecked(fd) }) {
        Ok(()) => 0,
        Err(err) => nv_close_error_to_rc(err),
    }
}

/// Queries an event for a device.
///
/// Corresponds to `nvQueryEvent()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_query_event(
    fd: u32,
    event_id: u32,
    event_out: *mut u32,
) -> u32 {
    if event_out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = crate::nv_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // SAFETY: fd is provided by the C caller who obtained it from nvOpen.
    match service.query_event(unsafe { Fd::new_unchecked(fd) }, event_id) {
        Ok(handle) => {
            unsafe { *event_out = handle };
            0
        }
        Err(err) => nv_query_event_error_to_rc(err),
    }
}

/// Converts a raw NV error code to a result code.
///
/// Corresponds to `nvConvertError()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_convert_error(rc: i32) -> u32 {
    if rc == 0 {
        return 0;
    }
    nv_error_to_result_code(rc as u32)
}

/// Gets the NV service session.
///
/// Corresponds to `nvGetServiceSession()` in libnx.
///
/// # Safety
///
/// NV must be initialized. The returned pointer points to a static buffer
/// that is updated on initialization and cleared on exit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__nv_get_service_session() -> *mut Service {
    NV_FFI_SESSION.get().cast::<Service>()
}

fn nv_connect_error_to_rc(err: crate::nv_manager::ConnectError) -> u32 {
    match err {
        crate::nv_manager::ConnectError::Connect(e) => match e {
            nx_service_nv::ConnectError::GetService(sm_err) => match sm_err {
                nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
                nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
            },
            nx_service_nv::ConnectError::CreateTransferMemory(_) => GENERIC_ERROR,
            nx_service_nv::ConnectError::Initialize(e) => match e {
                nx_service_nv::InitializeError::SendRequest(e) => e.to_rc(),
                nx_service_nv::InitializeError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
            },
            nx_service_nv::ConnectError::CloseTransferMemHandle(_) => GENERIC_ERROR,
            nx_service_nv::ConnectError::CloneSession(_) => GENERIC_ERROR,
        },
    }
}

/// Converts an NV error code to a libnx-compatible result code.
///
/// Uses Module_LibnxNvidia (346) as the module.
fn nv_error_to_result_code(code: u32) -> u32 {
    const MODULE_LIBNX_NVIDIA: u32 = 346;

    // Map raw NV error codes to libnx error descriptors
    let desc: u32 = match code {
        0x1 => 1,      // NotImplemented
        0x2 => 2,      // NotSupported
        0x3 => 3,      // NotInitialized
        0x4 => 4,      // BadParameter
        0x5 => 5,      // Timeout
        0x6 => 6,      // InsufficientMemory
        0x7 => 7,      // ReadOnlyAttribute
        0x8 => 8,      // InvalidState
        0x9 => 9,      // InvalidAddress
        0xA => 10,     // InvalidSize
        0xB => 11,     // BadValue
        0xD => 12,     // AlreadyAllocated
        0xE => 13,     // Busy
        0xF => 14,     // ResourceError
        0x10 => 15,    // CountMismatch
        0x1000 => 16,  // SharedMemoryTooSmall
        0x30003 => 17, // FileOperationFailed
        0x3000F => 18, // IoctlFailed
        _ => 19,       // Unknown
    };

    // MAKERESULT(module, description) = ((module & 0x1FF) | ((description & 0x1FFF) << 9))
    (MODULE_LIBNX_NVIDIA & 0x1FF) | ((desc & 0x1FFF) << 9)
}

fn nv_open_error_to_rc(err: nx_service_nv::OpenError) -> u32 {
    match err {
        nx_service_nv::OpenError::SendRequest(e) => e.to_rc(),
        nx_service_nv::OpenError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_nv::OpenError::NvError(nv_err) => nv_error_to_result_code(nv_err.to_raw()),
    }
}

fn nv_ioctl_error_to_rc(err: nx_service_nv::IoctlError) -> u32 {
    match err {
        nx_service_nv::IoctlError::SendRequest(e) => e.to_rc(),
        nx_service_nv::IoctlError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_nv::IoctlError::NvError(nv_err) => nv_error_to_result_code(nv_err.to_raw()),
    }
}

fn nv_ioctl2_error_to_rc(err: nx_service_nv::Ioctl2Error) -> u32 {
    match err {
        nx_service_nv::Ioctl2Error::SendRequest(e) => e.to_rc(),
        nx_service_nv::Ioctl2Error::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_nv::Ioctl2Error::NvError(nv_err) => nv_error_to_result_code(nv_err.to_raw()),
    }
}

fn nv_ioctl3_error_to_rc(err: nx_service_nv::Ioctl3Error) -> u32 {
    match err {
        nx_service_nv::Ioctl3Error::SendRequest(e) => e.to_rc(),
        nx_service_nv::Ioctl3Error::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_nv::Ioctl3Error::NvError(nv_err) => nv_error_to_result_code(nv_err.to_raw()),
    }
}

fn nv_close_error_to_rc(err: nx_service_nv::CloseError) -> u32 {
    match err {
        nx_service_nv::CloseError::SendRequest(e) => e.to_rc(),
        nx_service_nv::CloseError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_nv::CloseError::NvError(nv_err) => nv_error_to_result_code(nv_err.to_raw()),
    }
}

fn nv_query_event_error_to_rc(err: nx_service_nv::QueryEventError) -> u32 {
    match err {
        nx_service_nv::QueryEventError::SendRequest(e) => e.to_rc(),
        nx_service_nv::QueryEventError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_nv::QueryEventError::NvError(nv_err) => nv_error_to_result_code(nv_err.to_raw()),
        nx_service_nv::QueryEventError::MissingHandle => GENERIC_ERROR,
    }
}

// =============================================================================
// VI (Visual Interface) service FFI
// =============================================================================

/// C-compatible display structure matching libnx ViDisplay.
#[repr(C)]
pub struct ViDisplay {
    /// Display ID.
    pub display_id: u64,
    /// Display name (64 bytes, null-terminated).
    pub display_name: [u8; 0x40],
    /// Whether the display is initialized.
    pub initialized: bool,
}

/// C-compatible layer structure matching libnx ViLayer.
#[repr(C)]
pub struct ViLayer {
    /// Layer ID.
    pub layer_id: u64,
    /// IGraphicBufferProducer binder object ID.
    pub igbp_binder_obj_id: u32,
    /// Flags: bit 0 = initialized, bit 1 = stray_layer
    flags: u8,
}

impl ViLayer {
    /// Returns whether the layer is initialized.
    #[inline]
    fn is_initialized(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Returns whether this is a stray layer.
    #[inline]
    fn is_stray_layer(&self) -> bool {
        self.flags & 0x02 != 0
    }
}

/// Initializes the VI service.
///
/// Corresponds to `viInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_initialize(service_type: i32) -> u32 {
    let vi_service_type = match nx_service_vi::types::ViServiceType::from_raw(service_type) {
        Some(st) => st,
        None => return GENERIC_ERROR,
    };

    // Check if this is the first initialization
    let was_initialized = vi_manager::is_initialized();

    match vi_manager::init(vi_service_type) {
        Ok(()) => {
            // Only update FFI session buffers on first actual initialization
            if !was_initialized {
                set_vi_ffi_sessions();
            }
            0
        }
        Err(err) => vi_connect_error_to_rc(err),
    }
}

/// Exits the VI service.
///
/// Corresponds to `viExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_exit() {
    let was_initialized = vi_manager::is_initialized();
    vi_manager::exit();
    let still_initialized = vi_manager::is_initialized();

    // Only clear FFI session buffers if the service was actually closed
    if was_initialized && !still_initialized {
        clear_vi_ffi_sessions();
    }
}

/// Gets the IApplicationDisplayService session pointer.
///
/// Corresponds to `viGetSession_IApplicationDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_application_display() -> *mut Service {
    VI_FFI_APPLICATION_DISPLAY.get().cast::<Service>()
}

/// Gets the IHOSBinderDriverRelay session pointer.
///
/// Corresponds to `viGetSession_IHOSBinderDriverRelay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_binder_relay() -> *mut Service {
    VI_FFI_BINDER_RELAY.get().cast::<Service>()
}

/// Gets the ISystemDisplayService session pointer.
///
/// Corresponds to `viGetSession_ISystemDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_system_display() -> *mut Service {
    VI_FFI_SYSTEM_DISPLAY.get().cast::<Service>()
}

/// Gets the IManagerDisplayService session pointer.
///
/// Corresponds to `viGetSession_IManagerDisplayService()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_manager_display() -> *mut Service {
    VI_FFI_MANAGER_DISPLAY.get().cast::<Service>()
}

/// Gets the IHOSBinderDriverIndirect session pointer.
///
/// Corresponds to `viGetSession_IHOSBinderDriverIndirect()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_session_binder_indirect() -> *mut Service {
    VI_FFI_BINDER_INDIRECT.get().cast::<Service>()
}

/// Opens a display by name.
///
/// Corresponds to `viOpenDisplay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_open_display(
    name: *const c_char,
    display: *mut ViDisplay,
) -> u32 {
    if name.is_null() || display.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // Zero-initialize the display struct
    unsafe { core::ptr::write_bytes(display, 0, 1) };

    // Copy display name from C string
    let display_ref = unsafe { &mut *display };
    let name_cstr = unsafe { core::ffi::CStr::from_ptr(name) };
    let name_bytes = name_cstr.to_bytes();
    let copy_len = name_bytes.len().min(0x3F);
    display_ref.display_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

    // Create DisplayName from the bytes
    let vi_display_name =
        nx_service_vi::DisplayName::from_ascii(name_cstr.to_str().unwrap_or("Default"));

    match service.open_display(&vi_display_name) {
        Ok(display_id) => {
            display_ref.display_id = display_id.to_raw();
            display_ref.initialized = true;
            0
        }
        Err(err) => vi_open_display_error_to_rc(err),
    }
}

/// Closes a display.
///
/// Corresponds to `viCloseDisplay()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_close_display(display: *mut ViDisplay) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &mut *display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.close_display(display_id) {
        Ok(()) => {
            // Zero-initialize the struct on success
            unsafe { core::ptr::write_bytes(display, 0, 1) };
            0
        }
        Err(err) => vi_close_display_error_to_rc(err),
    }
}

/// Gets display resolution.
///
/// Corresponds to `viGetDisplayResolution()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_display_resolution(
    display: *const ViDisplay,
    width: *mut i32,
    height: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_display_resolution(display_id) {
        Ok(res) => {
            if !width.is_null() {
                unsafe { *width = res.width as i32 };
            }
            if !height.is_null() {
                unsafe { *height = res.height as i32 };
            }
            0
        }
        Err(err) => vi_get_display_resolution_error_to_rc(err),
    }
}

/// Gets display logical resolution.
///
/// Corresponds to `viGetDisplayLogicalResolution()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_display_logical_resolution(
    display: *const ViDisplay,
    width: *mut i32,
    height: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_display_logical_resolution(display_id) {
        Ok(res) => {
            if !width.is_null() {
                unsafe { *width = res.width };
            }
            if !height.is_null() {
                unsafe { *height = res.height };
            }
            0
        }
        Err(err) => vi_get_display_logical_resolution_error_to_rc(err),
    }
}

/// Sets display magnification (3.0.0+).
///
/// Corresponds to `viSetDisplayMagnification()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_display_magnification(
    display: *const ViDisplay,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.set_display_magnification(display_id, x, y, width, height) {
        Ok(()) => 0,
        Err(err) => vi_set_display_magnification_error_to_rc(err),
    }
}

/// Gets display vsync event handle.
///
/// Corresponds to `viGetDisplayVsyncEvent()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_display_vsync_event(
    display: *const ViDisplay,
    event_handle_out: *mut u32,
) -> u32 {
    if display.is_null() || event_handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_display_vsync_event(display_id) {
        Ok(handle) => {
            unsafe { *event_handle_out = handle };
            0
        }
        Err(err) => vi_get_display_vsync_event_error_to_rc(err),
    }
}

/// Sets display power state.
///
/// Corresponds to `viSetDisplayPowerState()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_display_power_state(
    display: *const ViDisplay,
    power_state: u32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    let state = match power_state {
        0 => nx_service_vi::ViPowerState::Off,
        1 => nx_service_vi::ViPowerState::NotScanning,
        2 => nx_service_vi::ViPowerState::On,
        _ => return GENERIC_ERROR,
    };

    match service.set_display_power_state(display_id, state) {
        Ok(()) => 0,
        Err(err) => vi_set_display_power_state_error_to_rc(err),
    }
}

/// Sets display alpha.
///
/// Corresponds to `viSetDisplayAlpha()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_display_alpha(
    display: *const ViDisplay,
    alpha: f32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.set_display_alpha(display_id, alpha) {
        Ok(()) => 0,
        Err(err) => vi_set_display_alpha_error_to_rc(err),
    }
}

/// Gets Z-order count minimum.
///
/// Corresponds to `viGetZOrderCountMin()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_z_order_count_min(
    display: *const ViDisplay,
    z: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_z_order_count_min(display_id) {
        Ok(min_z) => {
            if !z.is_null() {
                unsafe { *z = min_z };
            }
            0
        }
        Err(err) => vi_get_z_order_count_min_error_to_rc(err),
    }
}

/// Gets Z-order count maximum.
///
/// Corresponds to `viGetZOrderCountMax()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_z_order_count_max(
    display: *const ViDisplay,
    z: *mut i32,
) -> u32 {
    if display.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    match service.get_z_order_count_max(display_id) {
        Ok(max_z) => {
            if !z.is_null() {
                unsafe { *z = max_z };
            }
            0
        }
        Err(err) => vi_get_z_order_count_max_error_to_rc(err),
    }
}

/// Creates a layer (uses stray layer or managed layer depending on context).
///
/// Corresponds to `viCreateLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_create_layer(
    display: *const ViDisplay,
    layer: *mut ViLayer,
) -> u32 {
    if display.is_null() || layer.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // Zero-initialize the layer struct
    unsafe { core::ptr::write_bytes(layer, 0, 1) };
    let layer_ref = unsafe { &mut *layer };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    // Create stray layer (simplified - libnx has more complex logic)
    match service.create_stray_layer(nx_service_vi::ViLayerFlags::Default, display_id) {
        Ok(output) => {
            layer_ref.layer_id = output.layer_id.to_raw();
            // Parse parcel to get binder object ID
            layer_ref.igbp_binder_obj_id =
                parse_native_window_binder_id(&output.native_window).unwrap_or(0);
            layer_ref.flags = 0x03; // initialized (0x01) | stray_layer (0x02)
            0
        }
        Err(err) => vi_create_stray_layer_error_to_rc(err),
    }
}

/// Creates a managed layer.
///
/// Corresponds to `viCreateManagedLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_create_managed_layer(
    display: *const ViDisplay,
    layer_flags: u32,
    aruid: u64,
    layer_id_out: *mut u64,
) -> u32 {
    if display.is_null() || layer_id_out.is_null() {
        return GENERIC_ERROR;
    }

    let display_ref = unsafe { &*display };

    if !display_ref.initialized {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let display_id = nx_service_vi::DisplayId::new(display_ref.display_id);

    let flags = if layer_flags == 1 {
        nx_service_vi::ViLayerFlags::Default
    } else {
        nx_service_vi::ViLayerFlags::Default
    };

    match service.create_managed_layer(flags, display_id, aruid) {
        Ok(layer_id) => {
            unsafe { *layer_id_out = layer_id.to_raw() };
            0
        }
        Err(err) => vi_create_managed_layer_error_to_rc(err),
    }
}

/// Destroys a managed layer.
///
/// Corresponds to `viDestroyManagedLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_destroy_managed_layer(layer: *mut ViLayer) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.destroy_managed_layer(layer_id) {
        Ok(()) => {
            // Zero-initialize the struct on success
            unsafe { core::ptr::write_bytes(layer, 0, 1) };
            0
        }
        Err(err) => vi_destroy_managed_layer_error_to_rc(err),
    }
}

/// Closes a layer.
///
/// Corresponds to `viCloseLayer()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_close_layer(layer: *mut ViLayer) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    let rc = if layer_ref.is_stray_layer() {
        match service.destroy_stray_layer(layer_id) {
            Ok(()) => 0,
            Err(err) => vi_destroy_stray_layer_error_to_rc(err),
        }
    } else {
        match service.close_layer(layer_id) {
            Ok(()) => 0,
            Err(err) => vi_close_layer_error_to_rc(err),
        }
    };

    if rc == 0 {
        // Zero-initialize the struct on success
        unsafe { core::ptr::write_bytes(layer, 0, 1) };
    }
    rc
}

/// Sets layer size.
///
/// Corresponds to `viSetLayerSize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_size(
    layer: *const ViLayer,
    width: i32,
    height: i32,
) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_size(layer_id, width, height) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_size_error_to_rc(err),
    }
}

/// Sets layer Z-order.
///
/// Corresponds to `viSetLayerZ()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_z(layer: *const ViLayer, z: i32) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_z(layer_id, z) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_z_error_to_rc(err),
    }
}

/// Sets layer position.
///
/// Corresponds to `viSetLayerPosition()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_position(
    layer: *const ViLayer,
    x: f32,
    y: f32,
) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    match service.set_layer_position(layer_id, x, y) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_position_error_to_rc(err),
    }
}

/// Sets layer scaling mode.
///
/// Corresponds to `viSetLayerScalingMode()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_layer_scaling_mode(
    layer: *const ViLayer,
    scaling_mode: u32,
) -> u32 {
    if layer.is_null() {
        return GENERIC_ERROR;
    }

    let layer_ref = unsafe { &*layer };

    if !layer_ref.is_initialized() {
        return libnx_error(LibnxError::NotInitialized);
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    let layer_id = nx_service_vi::LayerId::new(layer_ref.layer_id);

    let mode = match scaling_mode {
        0 => nx_service_vi::ViScalingMode::None,
        2 => nx_service_vi::ViScalingMode::FitToLayer,
        4 => nx_service_vi::ViScalingMode::PreserveAspectRatio,
        _ => return GENERIC_ERROR,
    };

    match service.set_layer_scaling_mode(layer_id, mode) {
        Ok(()) => 0,
        Err(err) => vi_set_layer_scaling_mode_error_to_rc(err),
    }
}

/// Gets indirect layer image map.
///
/// Corresponds to `viGetIndirectLayerImageMap()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_indirect_layer_image_map(
    buffer: *mut c_void,
    size: usize,
    width: i32,
    height: i32,
    indirect_layer_consumer_handle: u64,
    out_size: *mut u64,
    out_stride: *mut u64,
) -> u32 {
    if buffer.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    // Get ARUID from applet manager
    let aruid = applet_manager::get_applet_resource_user_id()
        .map(|a| a.to_raw())
        .unwrap_or(0);

    let buffer_slice = unsafe { core::slice::from_raw_parts_mut(buffer as *mut u8, size) };

    match service.get_indirect_layer_image_map(
        width,
        height,
        indirect_layer_consumer_handle,
        aruid,
        buffer_slice,
    ) {
        Ok(info) => {
            if !out_size.is_null() {
                unsafe { *out_size = info.size as u64 };
            }
            if !out_stride.is_null() {
                unsafe { *out_stride = info.stride as u64 };
            }
            0
        }
        Err(err) => vi_get_indirect_layer_image_map_error_to_rc(err),
    }
}

/// Gets indirect layer image required memory info.
///
/// Corresponds to `viGetIndirectLayerImageRequiredMemoryInfo()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_get_indirect_layer_image_required_memory_info(
    width: i32,
    height: i32,
    out_size: *mut u64,
    out_alignment: *mut u64,
) -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    match service.get_indirect_layer_image_required_memory_info(width, height) {
        Ok(info) => {
            if !out_size.is_null() {
                unsafe { *out_size = info.size as u64 };
            }
            if !out_alignment.is_null() {
                unsafe { *out_alignment = info.alignment as u64 };
            }
            0
        }
        Err(err) => vi_get_indirect_layer_image_required_memory_info_error_to_rc(err),
    }
}

/// Sets content visibility.
///
/// Corresponds to `viSetContentVisibility()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_set_content_visibility(visible: bool) -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return GENERIC_ERROR;
    };

    match service.set_content_visibility(visible) {
        Ok(()) => 0,
        Err(err) => vi_set_content_visibility_error_to_rc(err),
    }
}

/// Prepares the fatal display (16.0.0+).
///
/// Corresponds to `viManagerPrepareFatal()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_prepare_fatal() -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.prepare_fatal() {
        Ok(()) => 0,
        Err(err) => vi_prepare_fatal_error_to_rc(err),
    }
}

/// Shows the fatal display (16.0.0+).
///
/// Corresponds to `viManagerShowFatal()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_show_fatal() -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.show_fatal() {
        Ok(()) => 0,
        Err(err) => vi_show_fatal_error_to_rc(err),
    }
}

/// Draws a fatal rectangle (16.0.0+).
///
/// Corresponds to `viManagerDrawFatalRectangle()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_draw_fatal_rectangle(
    x: i32,
    y: i32,
    end_x: i32,
    end_y: i32,
    color: u16,
) -> u32 {
    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    match service.draw_fatal_rectangle(x, y, end_x, end_y, color) {
        Ok(()) => 0,
        Err(err) => vi_draw_fatal_rectangle_error_to_rc(err),
    }
}

/// Draws fatal text using UTF-32 codepoints (16.0.0+).
///
/// Corresponds to `viManagerDrawFatalText32()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__vi_manager_draw_fatal_text32(
    out_advance: *mut i32,
    x: i32,
    y: i32,
    utf32_codepoints: *const u32,
    num_codepoints: usize,
    scale_x: f32,
    scale_y: f32,
    font_type: u32,
    bg_color: u32,
    fg_color: u32,
    initial_advance: i32,
) -> u32 {
    if utf32_codepoints.is_null() || out_advance.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = vi_manager::get_service() else {
        return libnx_error(LibnxError::NotInitialized);
    };

    let codepoints_slice = unsafe { core::slice::from_raw_parts(utf32_codepoints, num_codepoints) };

    match service.draw_fatal_text32(
        x,
        y,
        codepoints_slice,
        scale_x,
        scale_y,
        font_type,
        bg_color,
        fg_color,
        initial_advance,
    ) {
        Ok(advance) => {
            unsafe { *out_advance = advance };
            0
        }
        Err(err) => vi_draw_fatal_text32_error_to_rc(err),
    }
}

// VI helper functions

/// Sets VI FFI session buffers from the active service.
fn set_vi_ffi_sessions() {
    let Some(service_ref) = vi_manager::get_service() else {
        return;
    };

    // IApplicationDisplayService
    let app_display = Service {
        session: service_ref.application_display_session(),
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };
    // SAFETY: Called only during first initialization.
    unsafe {
        VI_FFI_APPLICATION_DISPLAY
            .get()
            .cast::<Service>()
            .write(app_display)
    };

    // IHOSBinderDriverRelay
    let binder_relay = Service {
        session: service_ref.binder_relay().session,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };
    // SAFETY: Called only during first initialization.
    unsafe {
        VI_FFI_BINDER_RELAY
            .get()
            .cast::<Service>()
            .write(binder_relay)
    };

    // ISystemDisplayService (optional)
    if let Some(session) = service_ref.system_display_session() {
        let sys_display = Service {
            session,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during first initialization.
        unsafe {
            VI_FFI_SYSTEM_DISPLAY
                .get()
                .cast::<Service>()
                .write(sys_display)
        };
    }

    // IManagerDisplayService (optional)
    if let Some(session) = service_ref.manager_display_session() {
        let mgr_display = Service {
            session,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during first initialization.
        unsafe {
            VI_FFI_MANAGER_DISPLAY
                .get()
                .cast::<Service>()
                .write(mgr_display)
        };
    }

    // IHOSBinderDriverIndirect (optional)
    if let Some(session) = service_ref.binder_indirect_session() {
        let binder_indirect = Service {
            session,
            own_handle: 1,
            object_id: 0,
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during first initialization.
        unsafe {
            VI_FFI_BINDER_INDIRECT
                .get()
                .cast::<Service>()
                .write(binder_indirect)
        };
    }
}

/// Clears VI FFI session buffers.
fn clear_vi_ffi_sessions() {
    // SAFETY: Called only during exit, after service is closed.
    unsafe {
        VI_FFI_APPLICATION_DISPLAY
            .get()
            .write(MaybeUninit::zeroed());
        VI_FFI_BINDER_RELAY.get().write(MaybeUninit::zeroed());
        VI_FFI_SYSTEM_DISPLAY.get().write(MaybeUninit::zeroed());
        VI_FFI_MANAGER_DISPLAY.get().write(MaybeUninit::zeroed());
        VI_FFI_BINDER_INDIRECT.get().write(MaybeUninit::zeroed());
    }
}

/// Parses native window data to extract binder object ID.
fn parse_native_window_binder_id(
    native_window: &[u8; nx_service_vi::NATIVE_WINDOW_SIZE],
) -> Option<u32> {
    // Parcel header structure
    #[repr(C)]
    struct ParcelHeader {
        payload_off: u32,
        payload_size: u32,
        objects_off: u32,
        objects_size: u32,
    }

    if native_window.len() < core::mem::size_of::<ParcelHeader>() {
        return None;
    }

    let header =
        unsafe { core::ptr::read_unaligned(native_window.as_ptr().cast::<ParcelHeader>()) };

    let payload_off = header.payload_off as usize;
    let payload_size = header.payload_size as usize;

    if payload_off > native_window.len() {
        return None;
    }
    if payload_off + payload_size > native_window.len() {
        return None;
    }
    if payload_size < 3 * 4 {
        return None;
    }

    // Binder object ID is at offset 2 (third u32) in the payload
    let binder_id_offset = payload_off + 2 * 4;
    if binder_id_offset + 4 > native_window.len() {
        return None;
    }

    let binder_id = unsafe {
        core::ptr::read_unaligned(native_window.as_ptr().add(binder_id_offset).cast::<u32>())
    };

    Some(binder_id)
}

// VI error conversion functions

fn vi_connect_error_to_rc(err: vi_manager::ConnectError) -> u32 {
    match err {
        vi_manager::ConnectError::Connect(e) => vi_service_connect_error_to_rc(e),
    }
}

fn vi_service_connect_error_to_rc(err: nx_service_vi::ConnectError) -> u32 {
    match err {
        nx_service_vi::ConnectError::GetService(e) => match e {
            nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
            nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
        },
        nx_service_vi::ConnectError::NoServiceAvailable => GENERIC_ERROR,
        nx_service_vi::ConnectError::GetDisplayService(e) => vi_get_display_service_error_to_rc(e),
        nx_service_vi::ConnectError::GetSubService(e) => vi_get_sub_service_error_to_rc(e),
    }
}

fn vi_get_display_service_error_to_rc(err: nx_service_vi::GetDisplayServiceError) -> u32 {
    match err {
        nx_service_vi::GetDisplayServiceError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetDisplayServiceError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_vi::GetDisplayServiceError::MissingHandle => GENERIC_ERROR,
    }
}

fn vi_get_sub_service_error_to_rc(err: nx_service_vi::GetSubServiceError) -> u32 {
    match err {
        nx_service_vi::GetSubServiceError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetSubServiceError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_vi::GetSubServiceError::MissingHandle => GENERIC_ERROR,
    }
}

fn vi_open_display_error_to_rc(err: nx_service_vi::OpenDisplayError) -> u32 {
    match err {
        nx_service_vi::OpenDisplayError::SendRequest(e) => e.to_rc(),
        nx_service_vi::OpenDisplayError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_close_display_error_to_rc(err: nx_service_vi::CloseDisplayError) -> u32 {
    match err {
        nx_service_vi::CloseDisplayError::SendRequest(e) => e.to_rc(),
        nx_service_vi::CloseDisplayError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_display_resolution_error_to_rc(err: nx_service_vi::GetDisplayResolutionError) -> u32 {
    match err {
        nx_service_vi::GetDisplayResolutionError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetDisplayResolutionError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_display_logical_resolution_error_to_rc(
    err: nx_service_vi::GetDisplayLogicalResolutionWrapperError,
) -> u32 {
    match err {
        nx_service_vi::GetDisplayLogicalResolutionWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::GetDisplayLogicalResolutionWrapperError::Cmif(e) => match e {
            nx_service_vi::GetDisplayLogicalResolutionError::SendRequest(e) => e.to_rc(),
            nx_service_vi::GetDisplayLogicalResolutionError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_display_magnification_error_to_rc(
    err: nx_service_vi::SetDisplayMagnificationWrapperError,
) -> u32 {
    match err {
        nx_service_vi::SetDisplayMagnificationWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetDisplayMagnificationWrapperError::Cmif(e) => match e {
            nx_service_vi::SetDisplayMagnificationError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetDisplayMagnificationError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_get_display_vsync_event_error_to_rc(err: nx_service_vi::GetDisplayVsyncEventError) -> u32 {
    match err {
        nx_service_vi::GetDisplayVsyncEventError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetDisplayVsyncEventError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
        nx_service_vi::GetDisplayVsyncEventError::MissingHandle => GENERIC_ERROR,
    }
}

fn vi_set_display_power_state_error_to_rc(
    err: nx_service_vi::SetDisplayPowerStateWrapperError,
) -> u32 {
    match err {
        nx_service_vi::SetDisplayPowerStateWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetDisplayPowerStateWrapperError::Cmif(e) => match e {
            nx_service_vi::SetDisplayPowerStateError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetDisplayPowerStateError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_display_alpha_error_to_rc(err: nx_service_vi::SetDisplayAlphaWrapperError) -> u32 {
    match err {
        nx_service_vi::SetDisplayAlphaWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetDisplayAlphaWrapperError::Cmif(e) => match e {
            nx_service_vi::SetDisplayAlphaError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetDisplayAlphaError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_get_z_order_count_min_error_to_rc(err: nx_service_vi::GetZOrderCountMinError) -> u32 {
    match err {
        nx_service_vi::GetZOrderCountMinError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::GetZOrderCountMinError::Cmif(e) => match e {
            nx_service_vi::GetZOrderCountError::SendRequest(e) => e.to_rc(),
            nx_service_vi::GetZOrderCountError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_get_z_order_count_max_error_to_rc(err: nx_service_vi::GetZOrderCountMaxError) -> u32 {
    match err {
        nx_service_vi::GetZOrderCountMaxError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::GetZOrderCountMaxError::Cmif(e) => match e {
            nx_service_vi::GetZOrderCountError::SendRequest(e) => e.to_rc(),
            nx_service_vi::GetZOrderCountError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_create_stray_layer_error_to_rc(err: nx_service_vi::CreateStrayLayerError) -> u32 {
    match err {
        nx_service_vi::CreateStrayLayerError::SendRequest(e) => e.to_rc(),
        nx_service_vi::CreateStrayLayerError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_create_managed_layer_error_to_rc(err: nx_service_vi::CreateManagedLayerWrapperError) -> u32 {
    match err {
        nx_service_vi::CreateManagedLayerWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::CreateManagedLayerWrapperError::Cmif(e) => match e {
            nx_service_vi::CreateManagedLayerError::SendRequest(e) => e.to_rc(),
            nx_service_vi::CreateManagedLayerError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_destroy_managed_layer_error_to_rc(
    err: nx_service_vi::DestroyManagedLayerWrapperError,
) -> u32 {
    match err {
        nx_service_vi::DestroyManagedLayerWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::DestroyManagedLayerWrapperError::Cmif(e) => match e {
            nx_service_vi::DestroyManagedLayerError::SendRequest(e) => e.to_rc(),
            nx_service_vi::DestroyManagedLayerError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_close_layer_error_to_rc(err: nx_service_vi::CloseLayerError) -> u32 {
    match err {
        nx_service_vi::CloseLayerError::SendRequest(e) => e.to_rc(),
        nx_service_vi::CloseLayerError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_destroy_stray_layer_error_to_rc(err: nx_service_vi::DestroyStrayLayerError) -> u32 {
    match err {
        nx_service_vi::DestroyStrayLayerError::SendRequest(e) => e.to_rc(),
        nx_service_vi::DestroyStrayLayerError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_set_layer_size_error_to_rc(err: nx_service_vi::SetLayerSizeWrapperError) -> u32 {
    match err {
        nx_service_vi::SetLayerSizeWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetLayerSizeWrapperError::Cmif(e) => match e {
            nx_service_vi::SetLayerSizeError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetLayerSizeError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_layer_z_error_to_rc(err: nx_service_vi::SetLayerZWrapperError) -> u32 {
    match err {
        nx_service_vi::SetLayerZWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetLayerZWrapperError::Cmif(e) => match e {
            nx_service_vi::SetLayerZError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetLayerZError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_layer_position_error_to_rc(err: nx_service_vi::SetLayerPositionWrapperError) -> u32 {
    match err {
        nx_service_vi::SetLayerPositionWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetLayerPositionWrapperError::Cmif(e) => match e {
            nx_service_vi::SetLayerPositionError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetLayerPositionError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_set_layer_scaling_mode_error_to_rc(err: nx_service_vi::SetLayerScalingModeError) -> u32 {
    match err {
        nx_service_vi::SetLayerScalingModeError::SendRequest(e) => e.to_rc(),
        nx_service_vi::SetLayerScalingModeError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_indirect_layer_image_map_error_to_rc(
    err: nx_service_vi::GetIndirectLayerImageMapError,
) -> u32 {
    match err {
        nx_service_vi::GetIndirectLayerImageMapError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetIndirectLayerImageMapError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_get_indirect_layer_image_required_memory_info_error_to_rc(
    err: nx_service_vi::GetIndirectLayerImageRequiredMemoryInfoError,
) -> u32 {
    match err {
        nx_service_vi::GetIndirectLayerImageRequiredMemoryInfoError::SendRequest(e) => e.to_rc(),
        nx_service_vi::GetIndirectLayerImageRequiredMemoryInfoError::ParseResponse(e) => match e {
            cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
            cmif::ParseResponseError::ServiceError(code) => code,
        },
    }
}

fn vi_set_content_visibility_error_to_rc(
    err: nx_service_vi::SetContentVisibilityWrapperError,
) -> u32 {
    match err {
        nx_service_vi::SetContentVisibilityWrapperError::NotAvailable => {
            libnx_error(LibnxError::NotInitialized)
        }
        nx_service_vi::SetContentVisibilityWrapperError::Cmif(e) => match e {
            nx_service_vi::SetContentVisibilityError::SendRequest(e) => e.to_rc(),
            nx_service_vi::SetContentVisibilityError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_prepare_fatal_error_to_rc(err: nx_service_vi::PrepareFatalWrapperError) -> u32 {
    match err {
        nx_service_vi::PrepareFatalWrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::PrepareFatalWrapperError::Cmif(e) => match e {
            nx_service_vi::PrepareFatalError::SendRequest(e) => e.to_rc(),
            nx_service_vi::PrepareFatalError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_show_fatal_error_to_rc(err: nx_service_vi::ShowFatalWrapperError) -> u32 {
    match err {
        nx_service_vi::ShowFatalWrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::ShowFatalWrapperError::Cmif(e) => match e {
            nx_service_vi::ShowFatalError::SendRequest(e) => e.to_rc(),
            nx_service_vi::ShowFatalError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_draw_fatal_rectangle_error_to_rc(err: nx_service_vi::DrawFatalRectangleWrapperError) -> u32 {
    match err {
        nx_service_vi::DrawFatalRectangleWrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::DrawFatalRectangleWrapperError::Cmif(e) => match e {
            nx_service_vi::DrawFatalRectangleError::SendRequest(e) => e.to_rc(),
            nx_service_vi::DrawFatalRectangleError::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

fn vi_draw_fatal_text32_error_to_rc(err: nx_service_vi::DrawFatalText32WrapperError) -> u32 {
    match err {
        nx_service_vi::DrawFatalText32WrapperError::NotAvailable => {
            libnx_error(LibnxError::IncompatSysVer)
        }
        nx_service_vi::DrawFatalText32WrapperError::Cmif(e) => match e {
            nx_service_vi::DrawFatalText32Error::SendRequest(e) => e.to_rc(),
            nx_service_vi::DrawFatalText32Error::ParseResponse(e) => match e {
                cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                cmif::ParseResponseError::ServiceError(code) => code,
            },
        },
    }
}

// libnx error codes

/// libnx error enumeration for MAKERESULT(Module_Libnx, error).
#[repr(u32)]
enum LibnxError {
    NotInitialized = 2,
    IncompatSysVer = 100,
}

/// Constructs a libnx result code.
const fn libnx_error(err: LibnxError) -> u32 {
    const MODULE_LIBNX: u32 = 345;
    (MODULE_LIBNX & 0x1FF) | ((err as u32 & 0x1FFF) << 9)
}

/// Wrapper to make UnsafeCell Sync for static storage.
#[repr(transparent)]
struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: Access is synchronized by SM_SESSION lock in service_manager.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    fn get(&self) -> *mut T {
        self.0.get()
    }
}
