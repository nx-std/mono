//! Human Interface Device (HID) service FFI

use core::ffi::c_void;

use nx_rt_core::error::ToResultCode as _;
use nx_service_hid;
use nx_sf::error::ToResultCode;

use crate::ffi::common::GENERIC_ERROR;

/// Initializes the HID service.
///
/// Corresponds to `hidInitialize()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_initialize() -> u32 {
    match crate::services::hid::init() {
        Ok(()) => 0,
        // The manager owns the mapping for its own failures; a second copy
        // here would drift from it.
        Err(err) => err.to_rc(),
    }
}

/// Exits the HID service.
///
/// Corresponds to `hidExit()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_exit() {
    crate::services::hid::exit();
}

/// Gets the shared memory address for HID.
///
/// Corresponds to `hidGetSharedmemAddr()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_get_sharedmem_addr() -> *const c_void {
    match crate::services::hid::get_service() {
        Some(service) => service.shared_memory() as *const _ as *const c_void,
        None => core::ptr::null(),
    }
}

/// Initializes Npad (controller) support.
///
/// Corresponds to `hidInitializeNpad()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_initialize_npad() {
    if let Some(service) = crate::services::hid::get_service() {
        // A refusal leaves the npad style set as the system had it: the
        // process still reads input, under whatever styles were already
        // permitted. The C signature returns nothing, so there is nowhere to
        // report it.
        let _ = service.activate_npad();
    }
}

/// Sets the supported Npad style set.
///
/// Corresponds to `hidSetSupportedNpadStyleSet()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_set_supported_npad_style_set(
    style_set: u32,
) -> u32 {
    match crate::services::hid::get_service() {
        Some(service) => match service.set_supported_npad_style_set(style_set) {
            Ok(()) => 0,
            Err(err) => match err {
                nx_service_hid::SetSupportedNpadStyleSetError::SendRequest(e) => e.to_rc(),
                nx_service_hid::SetSupportedNpadStyleSetError::ParseResponse(e) => e.to_rc(),
            },
        },
        None => GENERIC_ERROR,
    }
}

/// Sets the supported Npad ID types.
///
/// Corresponds to `hidSetSupportedNpadIdType()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_set_supported_npad_id_type(
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
                nx_service_hid::SetSupportedNpadIdTypeError::ParseResponse(e) => e.to_rc(),
            },
        },
        None => GENERIC_ERROR,
    }
}

/// Initializes touch screen support.
///
/// Corresponds to `hidInitializeTouchScreen()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_initialize_touch_screen() {
    if let Some(service) = crate::services::hid::get_service() {
        // A refusal leaves that one input source inactive: the process
        // reads no touch screen state, and every other source
        // it activated still works. The C signature returns nothing, so
        // there is nowhere to report it.
        let _ = service.activate_touch_screen();
    }
}

/// Initializes mouse support.
///
/// Corresponds to `hidInitializeMouse()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_initialize_mouse() {
    if let Some(service) = crate::services::hid::get_service() {
        // A refusal leaves that one input source inactive: the process
        // reads no mouse state, and every other source
        // it activated still works. The C signature returns nothing, so
        // there is nowhere to report it.
        let _ = service.activate_mouse();
    }
}

/// Initializes keyboard support.
///
/// Corresponds to `hidInitializeKeyboard()` in libnx.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_initialize_keyboard() {
    if let Some(service) = crate::services::hid::get_service() {
        // A refusal leaves that one input source inactive: the process
        // reads no keyboard state, and every other source
        // it activated still works. The C signature returns nothing, so
        // there is nowhere to report it.
        let _ = service.activate_keyboard();
    }
}

/// Initializes gesture recognition support.
///
/// This is not in libnx but provides access to the activate_gesture command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_hid_initialize_gesture() {
    if let Some(service) = crate::services::hid::get_service() {
        // A refusal leaves that one input source inactive: the process
        // reads no gesture state, and every other source
        // it activated still works. The C signature returns nothing, so
        // there is nowhere to report it.
        let _ = service.activate_gesture();
    }
}
