//! Human Interface Device (HID) service FFI

use core::ffi::c_void;

use nx_service_hid;
use nx_svc::error::ToRawResultCode;

use super::common::{GENERIC_ERROR, parse_resp_error_to_rc};

/// Initializes the HID service.
///
/// Corresponds to `hidInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize() -> u32 {
    match crate::services::hid::init() {
        Ok(()) => 0,
        Err(err) => {
            // Convert error to result code
            let crate::services::hid::ConnectError(conn_err) = err;
            match conn_err {
                nx_service_hid::ConnectError::GetService(sm_err) => match sm_err {
                    nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                    _ => GENERIC_ERROR,
                },
                nx_service_hid::ConnectError::CreateAppletResource(ar_err) => match ar_err {
                    nx_service_hid::CreateAppletResourceError::SendRequest(e) => e.to_rc(),
                    nx_service_hid::CreateAppletResourceError::ParseResponse(e) => {
                        parse_resp_error_to_rc(e)
                    }
                    nx_service_hid::CreateAppletResourceError::BuildRequest(_) => GENERIC_ERROR,
                    nx_service_hid::CreateAppletResourceError::MissingHandle => GENERIC_ERROR,
                },
                nx_service_hid::ConnectError::GetSharedMemoryHandle(sh_err) => match sh_err {
                    nx_service_hid::GetSharedMemoryHandleError::SendRequest(e) => e.to_rc(),
                    nx_service_hid::GetSharedMemoryHandleError::ParseResponse(e) => {
                        parse_resp_error_to_rc(e)
                    }
                    nx_service_hid::GetSharedMemoryHandleError::BuildRequest(_) => GENERIC_ERROR,
                    nx_service_hid::GetSharedMemoryHandleError::MissingHandle => GENERIC_ERROR,
                },
                nx_service_hid::ConnectError::MapSharedMemory(_) => GENERIC_ERROR,
                nx_service_hid::ConnectError::NullPointer => GENERIC_ERROR,
            }
        }
    }
}

/// Exits the HID service.
///
/// Corresponds to `hidExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_exit() {
    crate::services::hid::exit();
}

/// Gets the shared memory address for HID.
///
/// Corresponds to `hidGetSharedmemAddr()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_get_sharedmem_addr() -> *const c_void {
    match crate::services::hid::get_service() {
        Some(service) => service.shared_memory() as *const _ as *const c_void,
        None => core::ptr::null(),
    }
}

/// Initializes Npad (controller) support.
///
/// Corresponds to `hidInitializeNpad()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_npad() {
    if let Some(service) = crate::services::hid::get_service() {
        // Ignore errors - libnx diagAborts on failure, but we'll just return
        let _ = service.activate_npad();
    }
}

/// Sets the supported Npad style set.
///
/// Corresponds to `hidSetSupportedNpadStyleSet()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_set_supported_npad_style_set(style_set: u32) -> u32 {
    match crate::services::hid::get_service() {
        Some(service) => match service.set_supported_npad_style_set(style_set) {
            Ok(()) => 0,
            Err(err) => match err {
                nx_service_hid::SetSupportedNpadStyleSetError::SendRequest(e) => e.to_rc(),
                nx_service_hid::SetSupportedNpadStyleSetError::ParseResponse(e) => {
                    parse_resp_error_to_rc(e)
                }
                nx_service_hid::SetSupportedNpadStyleSetError::BuildRequest(_) => GENERIC_ERROR,
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

    match crate::services::hid::get_service() {
        Some(service) => match service.set_supported_npad_id_type(ids_slice) {
            Ok(()) => 0,
            Err(err) => match err {
                nx_service_hid::SetSupportedNpadIdTypeError::SendRequest(e) => e.to_rc(),
                nx_service_hid::SetSupportedNpadIdTypeError::ParseResponse(e) => {
                    parse_resp_error_to_rc(e)
                }
                nx_service_hid::SetSupportedNpadIdTypeError::BuildRequest(_) => GENERIC_ERROR,
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
    if let Some(service) = crate::services::hid::get_service() {
        let _ = service.activate_touch_screen();
    }
}

/// Initializes mouse support.
///
/// Corresponds to `hidInitializeMouse()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_mouse() {
    if let Some(service) = crate::services::hid::get_service() {
        let _ = service.activate_mouse();
    }
}

/// Initializes keyboard support.
///
/// Corresponds to `hidInitializeKeyboard()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_keyboard() {
    if let Some(service) = crate::services::hid::get_service() {
        let _ = service.activate_keyboard();
    }
}

/// Initializes gesture recognition support.
///
/// This is not in libnx but provides access to the activate_gesture command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__hid_initialize_gesture() {
    if let Some(service) = crate::services::hid::get_service() {
        let _ = service.activate_gesture();
    }
}
