//! NVIDIA Driver (NV) service FFI

use core::{
    ffi::{c_char, c_void},
    mem::MaybeUninit,
};

use nx_service_nv::fd::Fd;
use nx_sf::{cmif, service::Service};

use super::common::{GENERIC_ERROR, SyncUnsafeCell};

/// Static buffer for NV FFI session access. Updated on `nv_initialize()` and `nv_exit()`.
static NV_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

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
            if !was_initialized && let Some(service_ref) = crate::nv_manager::get_service() {
                let service = Service {
                    session: service_ref.session(),
                    own_handle: 1,
                    object_id: 0,
                    pointer_buffer_size: 0,
                };
                // SAFETY: Called only during first initialization.
                unsafe { NV_FFI_SESSION.get().cast::<Service>().write(service) };
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
    use nx_svc::error::ToRawResultCode;

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
    use nx_svc::error::ToRawResultCode;

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
    use nx_svc::error::ToRawResultCode;

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
    use nx_svc::error::ToRawResultCode;

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
    use nx_svc::error::ToRawResultCode;

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
    use nx_svc::error::ToRawResultCode;

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
    use nx_svc::error::ToRawResultCode;

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
