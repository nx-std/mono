//! FFI exports for libnx runtime functions

use core::{
    cell::UnsafeCell,
    ffi::{c_char, c_uint, c_void},
    mem::MaybeUninit,
};

use nx_sf::{ServiceName, cmif, service::Service, tipc};
use nx_svc::{
    error::ToRawResultCode, process::Handle as ProcessHandle, raw::INVALID_HANDLE,
    thread::Handle as ThreadHandle,
};

use crate::{
    apm_manager, applet_manager, argv,
    env::{self, AccountUid, ConfigEntry, LoaderReturnFn},
    init, service_manager, service_registry,
};

/// Generic error code for FFI when no specific result code is available.
const GENERIC_ERROR: u32 = 0xFFFF;

/// Static buffer for SM FFI session access. Updated on `initialize()` and `exit()`.
static SM_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
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
