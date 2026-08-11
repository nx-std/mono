//! Error applet (`error` library applet) FFI.
//!
//! libnx's `error.c` holds no file-local state of its own, but every function in
//! it reaches the applet through `g_appletILibraryAppletCreator`, which is
//! `static` in `applet.c` and so cannot be aliased. Our `appletInitialize`
//! override replaces the only code that would populate it, so once
//! `use_nx_service_applet` is on, *every* libnx `error*` function runs against a
//! zeroed session.
//!
//! That is why this module covers the whole surface rather than the two commands
//! it implements: a command left to libnx does not fail cleanly. The ones not
//! ported yet are aliased to stubs that panic naming the command, which is a
//! diagnosable failure rather than a request against a zeroed handle.

use core::ffi::{
    c_char,
    c_void,
};

use nx_service_applet_err::{
    ApplicationError,
    proto::ErrorApplicationArg,
};
use nx_sf::error::ToResultCode as _;

use crate::{
    ffi::common::GENERIC_ERROR,
    services::applet,
};

/// Borrows a NUL-terminated C string as UTF-8.
///
/// Returns `None` for a null pointer or for bytes that are not UTF-8; the applet
/// renders the message rather than validating it, so invalid input is rejected
/// here rather than passed on.
///
/// # Safety
///
/// `ptr` must be null or point to a NUL-terminated string that stays valid and
/// unwritten for `'a`.
unsafe fn borrow_c_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: The caller guarantees `ptr` is a live NUL-terminated string.
    unsafe { core::ffi::CStr::from_ptr(ptr) }.to_str().ok()
}

/// Fills `config` with an application error carrying the given messages.
///
/// Corresponds to `errorApplicationCreate()` in `error.h`.
///
/// # Safety
///
/// `config` must point to a writable `ErrorApplicationConfig`, and each message
/// must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_application_create(
    config: *mut ErrorApplicationArg,
    dialog_message: *const c_char,
    fullscreen_message: *const c_char,
) -> u32 {
    if config.is_null() {
        return GENERIC_ERROR;
    }

    // SAFETY: The caller guarantees the message pointers are null or live
    // NUL-terminated strings.
    let Some(dialog) = (unsafe { borrow_c_str(dialog_message) }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: As above; a null fullscreen message is the documented "no details"
    // case rather than an error.
    let fullscreen = unsafe { borrow_c_str(fullscreen_message) };

    let arg = ErrorApplicationArg::new(dialog, fullscreen);

    // SAFETY: `config` is non-null and the caller guarantees it is writable for
    // the size of the arg struct, which is what `ErrorApplicationConfig` holds.
    unsafe { config.write(arg) };

    0
}

/// Shows the application error dialog described by `config`.
///
/// Corresponds to `errorApplicationShow()` in `error.h`. Blocks until the user
/// dismisses the dialog.
///
/// # Safety
///
/// `config` must point to a readable `ErrorApplicationConfig` previously filled
/// by [`__nx_rt_nro__libnx_error_application_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_application_show(
    config: *const ErrorApplicationArg,
) -> u32 {
    if config.is_null() {
        return GENERIC_ERROR;
    }

    // SAFETY: The caller guarantees `config` is readable for the size of the arg
    // struct.
    let arg = unsafe { config.read() };

    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    match ApplicationError::from_arg(arg).show(&self_controller.get(), &creator.get()) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Stands in for libnx's `errorResultShow`.
///
/// # Safety
///
/// `ctx` must be null or point to a readable `ErrorContext`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_result_show(
    _res: u32,
    _jump_flag: bool,
    _ctx: *const c_void,
) -> u32 {
    todo!("errorResultShow")
}

/// Error code displayed as `XXXX-XXXX`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ErrorCode {
    /// Module portion, normally module + 2000.
    pub low: u32,
    /// Error description.
    pub desc: u32,
}

/// Stands in for libnx's `errorCodeShow`.
///
/// # Safety
///
/// `ctx` must be null or point to a readable `ErrorContext`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_code_show(
    _error_code: ErrorCode,
    _jump_flag: bool,
    _ctx: *const c_void,
) -> u32 {
    todo!("errorCodeShow")
}

/// Stands in for libnx's `errorCodeRecordShow`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_code_record_show(
    _error_code: ErrorCode,
    _timestamp: u64,
) -> u32 {
    todo!("errorCodeRecordShow")
}

/// Stands in for libnx's `errorResultBacktraceCreate`.
///
/// # Safety
///
/// `backtrace` must point to a writable `ErrorResultBacktrace`, and `entries` to
/// `count` readable results.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_result_backtrace_create(
    _backtrace: *mut c_void,
    _count: i32,
    _entries: *const u32,
) -> u32 {
    todo!("errorResultBacktraceCreate")
}

/// Stands in for libnx's `errorResultBacktraceShow`.
///
/// # Safety
///
/// `backtrace` must point to a readable `ErrorResultBacktrace`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_result_backtrace_show(
    _res: u32,
    _backtrace: *const c_void,
) -> u32 {
    todo!("errorResultBacktraceShow")
}

/// Stands in for libnx's `errorSystemCreate`.
///
/// # Safety
///
/// `config` must point to a writable `ErrorSystemConfig`, and each message must
/// be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_system_create(
    _config: *mut c_void,
    _dialog_message: *const c_char,
    _fullscreen_message: *const c_char,
) -> u32 {
    todo!("errorSystemCreate")
}

/// Stands in for libnx's `errorSystemShow`.
///
/// # Safety
///
/// `config` must point to a readable `ErrorSystemConfig`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_system_show(_config: *mut c_void) -> u32 {
    todo!("errorSystemShow")
}

/// Stands in for libnx's `errorSystemSetContext`.
///
/// # Safety
///
/// `config` must point to a writable `ErrorSystemConfig`, and `ctx` to a
/// readable `ErrorContext`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_system_set_context(
    _config: *mut c_void,
    _ctx: *const c_void,
) {
    todo!("errorSystemSetContext")
}

/// Stands in for libnx's `errorEulaShow`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_eula_show(_region_code: u32) -> u32 {
    todo!("errorEulaShow")
}

/// Stands in for libnx's `errorSystemUpdateEulaShow`.
///
/// # Safety
///
/// `eula` must point to a readable `ErrorEulaData`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_error_system_update_eula_show(
    _region_code: u32,
    _eula: *const c_void,
) -> u32 {
    todo!("errorSystemUpdateEulaShow")
}
