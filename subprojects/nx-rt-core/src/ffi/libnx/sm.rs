//! Service Manager (SM) service FFI

use core::mem::MaybeUninit;

use nx_sf::{ServiceName, ffi::Service};

use crate::{
    ffi::common::{
        GENERIC_ERROR, SyncUnsafeCell, parse_resp_error_to_rc, parse_tipc_resp_error_to_rc,
    },
    services::sm,
};

/// Static buffer for SM FFI session access. Updated on `initialize()` and `exit()`.
static SM_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Initializes SM connection. Returns 0 on success, error code on failure.
///
/// Corresponds to `smInitialize()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_initialize() -> u32 {
    if let Err(err) = sm::initialize() {
        return sm_initialize_error_to_rc(err);
    }
    set_sm_ffi_session();
    0
}

/// Closes SM connection.
///
/// Corresponds to `smExit()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_exit() {
    sm::exit();
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
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_get_service_wrapper(
    service_out: *mut Service,
    name: ServiceName,
) -> u32 {
    if service_out.is_null() {
        return GENERIC_ERROR;
    }

    let srv = match sm::get_service(name) {
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
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_get_service_original(
    handle_out: *mut u32,
    name: ServiceName,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match sm::get_service_handle(name) {
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
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_get_service_override(name: ServiceName) -> u32 {
    sm::get_override(name)
        .map(|h| h.to_raw())
        .unwrap_or(nx_svc::raw::INVALID_HANDLE)
}

/// Adds a service override.
///
/// Corresponds to `smAddOverrideHandle()` in `sm.h`.
///
/// # Safety
///
/// `handle` must be a valid handle value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_add_override_handle(
    name: ServiceName,
    handle: u32,
) {
    // SAFETY: Caller guarantees handle is valid.
    let handle = unsafe { nx_svc::ipc::Handle::from_raw(handle) };
    // Ignore error (matches libnx behavior)
    let _ = sm::add_override(name, handle);
}

/// Registers a service (auto-selects protocol). Returns 0 on success, error code on failure.
///
/// Corresponds to `smRegisterService()` in `sm.h`.
///
/// # Safety
///
/// `handle_out` must point to valid, writable memory for a u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_register_service(
    handle_out: *mut u32,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match sm::register_service(name, is_light, max_sessions) {
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
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_register_service_cmif(
    handle_out: *mut u32,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match sm::register_service_cmif(name, is_light, max_sessions) {
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
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_register_service_tipc(
    handle_out: *mut u32,
    name: ServiceName,
    is_light: bool,
    max_sessions: i32,
) -> u32 {
    if handle_out.is_null() {
        return GENERIC_ERROR;
    }

    let handle = match sm::register_service_tipc(name, is_light, max_sessions) {
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
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_unregister_service(name: ServiceName) -> u32 {
    if let Err(err) = sm::unregister_service(name) {
        return sm_unregister_service_error_to_rc(err);
    }

    0
}

/// Unregisters a service via CMIF. Returns 0 on success, error code on failure.
///
/// Corresponds to `smUnregisterServiceCmif()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_unregister_service_cmif(name: ServiceName) -> u32 {
    if let Err(err) = sm::unregister_service_cmif(name) {
        return sm_unregister_service_error_to_rc(err);
    }

    0
}

/// Unregisters a service via TIPC. Returns 0 on success, error code on failure.
///
/// Corresponds to `smUnregisterServiceTipc()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_unregister_service_tipc(name: ServiceName) -> u32 {
    if let Err(err) = sm::unregister_service_tipc(name) {
        return sm_unregister_service_error_to_rc(err);
    }

    0
}

/// Detaches the client (auto-selects protocol). Returns 0 on success, error code on failure.
///
/// Corresponds to `smDetachClient()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_detach_client() -> u32 {
    if let Err(err) = sm::detach_client() {
        return sm_detach_client_error_to_rc(err);
    }

    0
}

/// Detaches via CMIF. Returns 0 on success, error code on failure.
///
/// Corresponds to `smDetachClientCmif()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_detach_client_cmif() -> u32 {
    if let Err(err) = sm::detach_client_cmif() {
        return sm_detach_client_error_to_rc(err);
    }

    0
}

/// Detaches via TIPC. Returns 0 on success, error code on failure.
///
/// Corresponds to `smDetachClientTipc()` in `sm.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_detach_client_tipc() -> u32 {
    if let Err(err) = sm::detach_client_tipc() {
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
pub unsafe extern "C" fn __nx_rt_core__libnx_sm_get_service_session() -> *mut Service {
    SM_FFI_SESSION.get().cast::<Service>()
}

fn set_sm_ffi_session() {
    let guard = sm::sm_session();
    if let Some(sm) = guard.as_ref() {
        // Non-owning FFI view (`own_handle = 0`, `object_id = 0` — libnx's
        // "override" mode): the Rust `SmService` retains exclusive ownership
        // of the kernel handle and closes it on `Drop` via `sm::exit`. A
        // `own_handle = 1` snapshot here would risk a double-close if libnx
        // ever invoked `serviceClose` on the cached pointer.
        // SAFETY: Called only during initialization, single-threaded access.
        unsafe {
            let service = Service {
                session: sm.session(),
                own_handle: 0,
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

fn sm_initialize_error_to_rc(err: sm::InitializeError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err.0 {
        nx_service_sm::ConnectError::Connect(e) => e.to_rc(),
        nx_service_sm::ConnectError::RegisterClient(e) => match e {
            nx_service_sm::RegisterClientCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::RegisterClientCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
            nx_service_sm::RegisterClientCmifError::BuildRequest(_) => GENERIC_ERROR,
        },
    }
}

fn sm_get_service_error_to_rc(err: sm::GetServiceError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err.0 {
        nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
        nx_service_sm::GetServiceCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
        nx_service_sm::GetServiceCmifError::BuildRequest(_) => GENERIC_ERROR,
        nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
    }
}

fn sm_register_service_error_to_rc(err: sm::RegisterServiceError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        sm::RegisterServiceError::Cmif(e) => match e {
            nx_service_sm::RegisterServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::RegisterServiceCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
            nx_service_sm::RegisterServiceCmifError::BuildRequest(_) => GENERIC_ERROR,
            nx_service_sm::RegisterServiceCmifError::MissingHandle => GENERIC_ERROR,
        },
        sm::RegisterServiceError::Tipc(e) => match e {
            nx_service_sm::RegisterServiceTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::RegisterServiceTipcError::ParseResponse(e) => {
                parse_tipc_resp_error_to_rc(e)
            }
            nx_service_sm::RegisterServiceTipcError::BuildRequest(_) => GENERIC_ERROR,
            nx_service_sm::RegisterServiceTipcError::MissingHandle => GENERIC_ERROR,
        },
    }
}

fn sm_unregister_service_error_to_rc(err: sm::UnregisterServiceError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        sm::UnregisterServiceError::Cmif(e) => match e {
            nx_service_sm::UnregisterServiceCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::UnregisterServiceCmifError::ParseResponse(e) => {
                parse_resp_error_to_rc(e)
            }
            nx_service_sm::UnregisterServiceCmifError::BuildRequest(_) => GENERIC_ERROR,
        },
        sm::UnregisterServiceError::Tipc(e) => match e {
            nx_service_sm::UnregisterServiceTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::UnregisterServiceTipcError::ParseResponse(e) => {
                parse_tipc_resp_error_to_rc(e)
            }
            nx_service_sm::UnregisterServiceTipcError::BuildRequest(_) => GENERIC_ERROR,
        },
    }
}

fn sm_detach_client_error_to_rc(err: sm::DetachClientError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        // LibnxError_IncompatSysVer = 0x64 (100) in module 345
        // MAKERESULT(Module_Libnx, LibnxError_IncompatSysVer) = 0x8A564
        sm::DetachClientError::IncompatibleVersion => 0x8A564,
        sm::DetachClientError::Cmif(e) => match e {
            nx_service_sm::DetachClientCmifError::SendRequest(e) => e.to_rc(),
            nx_service_sm::DetachClientCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
            nx_service_sm::DetachClientCmifError::BuildRequest(_) => GENERIC_ERROR,
        },
        sm::DetachClientError::Tipc(e) => match e {
            nx_service_sm::DetachClientTipcError::SendRequest(e) => e.to_rc(),
            nx_service_sm::DetachClientTipcError::ParseResponse(e) => {
                parse_tipc_resp_error_to_rc(e)
            }
            nx_service_sm::DetachClientTipcError::BuildRequest(_) => GENERIC_ERROR,
        },
    }
}
