//! FFI exports for libnx runtime functions

use core::{
    cell::UnsafeCell,
    ffi::{c_char, c_uint, c_void},
    mem::MaybeUninit,
};

use nx_sf::{ServiceName, cmif, service::Service, tipc};
use nx_svc::{error::ToRawResultCode, raw::INVALID_HANDLE, thread::Handle as ThreadHandle};

use crate::{
    argv,
    env::{self, AccountUid, ConfigEntry, LoaderReturnFn},
    init, service_manager,
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
