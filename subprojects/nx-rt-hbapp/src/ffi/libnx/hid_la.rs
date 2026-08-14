//! Controller applet (`controller` library applet) FFI.
//!
//! libnx's `hid_la.c` holds no file-local state, but every function in it
//! reaches the applet through `g_appletILibraryAppletCreator`, which is `static`
//! in `applet.c` and so cannot be aliased. Our `appletInitialize` override
//! replaces the only code that would populate it, so once `use_nx_service_applet`
//! is on, *every* libnx `hidLa*` function runs against a zeroed session.
//!
//! That is why this module covers the whole surface: a command left to libnx
//! does not fail cleanly. Here that costs nothing, because all ten of libnx's
//! entry points are ported.
//!
//! # Where the system version is read
//!
//! This is the most version-dependent applet in the family, and all of its
//! version handling lives here rather than in `nx-service-applet-hid`: a service
//! crate may not depend on an `nx-rt-*` crate, which is where the system version
//! is held. Three kinds of check appear below, reproducing libnx exactly:
//!
//! - [`version`] merges libnx's two ladders, the library-applet API version and
//!   the argument layout, into the one value the crate takes.
//! - The availability gates on the strap guide, the firmware update and key
//!   remapping, which refuse outright on a system that lacks the screen. libnx
//!   checks the strap guide's before it reads the HID service and the other two
//!   after; all three are checked first here, since a request the system cannot
//!   serve is worth refusing before spending two round trips on it. The two
//!   differ only when the HID read would also have failed, and then only in
//!   which of the two failures is reported.
//! - The pre-[3.0.0] branch in
//!   [`__nx_rt_hbapp__libnx_hid_la_show_controller_support_for_system`], which
//!   sends fixed controller information rather than asking the HID service.

use core::ffi::{
    CStr,
    c_char,
};

use nx_rt_core::env::hos_version::{
    self,
    HosVersion,
};
use nx_service_applet_hid::{
    ControllerSupport,
    ControllerSupportContext,
    ControllerSupportVersion,
    NpadJoyHoldType,
    proto::{
        ControllerFirmwareUpdateArg,
        ControllerKeyRemappingArg,
        ControllerSupportArg,
        ControllerSupportCaller,
        ControllerSupportResultInfo,
    },
};
use nx_sf::error::ToResultCode as _;

use crate::{
    ffi::common::{
        GENERIC_ERROR,
        LibnxError,
        libnx_error,
    },
    services::{
        applet,
        hid,
    },
};

/// Returns the protocol revision the running system speaks.
///
/// libnx picks the library-applet API version from this ladder and the argument
/// layout from a second one that steps at [8.0.0]; because that step is on both,
/// one value settles both, and `ControllerSupportVersion` is that value.
fn version() -> ControllerSupportVersion {
    let hosver = hos_version::get();

    if hosver >= HosVersion::new(11, 0, 0) {
        ControllerSupportVersion::V8
    } else if hosver >= HosVersion::new(8, 0, 0) {
        ControllerSupportVersion::V7
    } else if hosver >= HosVersion::new(6, 0, 0) {
        ControllerSupportVersion::V5
    } else if hosver >= HosVersion::new(3, 0, 0) {
        ControllerSupportVersion::V4
    } else {
        ControllerSupportVersion::V3
    }
}

/// Asks the HID service what the applet must be told about the controllers.
///
/// libnx reads both values before every launch and gives up if either fails.
///
/// # Errors
///
/// Returns the result code the failing HID command reported, or
/// `LibnxError_NotInitialized` when the service was never brought up.
fn controller_context() -> Result<ControllerSupportContext, u32> {
    let Some(service) = hid::get_service() else {
        return Err(libnx_error(LibnxError::NotInitialized));
    };

    let npad_style_set = service
        .supported_npad_style_set()
        .map_err(nx_sf::error::ToResultCode::to_rc)?;
    let npad_joy_hold_type = service
        .npad_joy_hold_type()
        .map_err(nx_sf::error::ToResultCode::to_rc)?;

    Ok(ControllerSupportContext {
        npad_style_set,
        npad_joy_hold_type,
    })
}

/// Opens `request`, writing what the applet reported into `result_info`.
///
/// Shared by the six `hidLaShow*` entry points, which differ only in the screen
/// they open and in whether they expose a result slot.
fn show(
    request: ControllerSupport<'_>,
    context: &ControllerSupportContext,
    result_info: Option<&mut ControllerSupportResultInfo>,
) -> u32 {
    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    let info = match request.show(&self_controller.get(), &creator.get(), version(), context) {
        Ok(info) => info,
        Err(err) => return err.to_rc(),
    };

    if let Some(result_info) = result_info {
        *result_info = info;
    }

    0
}

/// Fills `arg` with the controller-support defaults.
///
/// Corresponds to `hidLaCreateControllerSupportArg()` in `hid_la.h`.
///
/// # Safety
///
/// `arg` must be null or point to a writable `HidLaControllerSupportArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_create_controller_support_arg(
    arg: *mut ControllerSupportArg,
) {
    // SAFETY: The caller upholds this function's `# Safety` contract, so `arg`
    // is null or points to a writable value of its type. libnx dereferences it
    // unconditionally; a null one is dropped here rather than followed.
    let Some(arg) = (unsafe { arg.as_mut() }) else {
        return;
    };

    *arg = ControllerSupportArg::new();
}

/// Clears `arg` to the firmware-update defaults.
///
/// Corresponds to `hidLaCreateControllerFirmwareUpdateArg()` in `hid_la.h`.
///
/// # Safety
///
/// `arg` must be null or point to a writable `HidLaControllerFirmwareUpdateArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_create_controller_firmware_update_arg(
    arg: *mut ControllerFirmwareUpdateArg,
) {
    // SAFETY: the caller guarantees `arg` is null or points to a writable
    // `ControllerFirmwareUpdateArg`.
    let Some(arg) = (unsafe { arg.as_mut() }) else {
        return;
    };

    *arg = ControllerFirmwareUpdateArg::default();
}

/// Clears `arg` to the key-remapping defaults.
///
/// Corresponds to `hidLaCreateControllerKeyRemappingArg()` in `hid_la.h`.
///
/// # Safety
///
/// `arg` must be null or point to a writable `HidLaControllerKeyRemappingArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_create_controller_key_remapping_arg(
    arg: *mut ControllerKeyRemappingArg,
) {
    // SAFETY: the caller guarantees `arg` is null or points to a writable
    // `ControllerKeyRemappingArg`.
    let Some(arg) = (unsafe { arg.as_mut() }) else {
        return;
    };

    *arg = ControllerKeyRemappingArg::default();
}

/// Sets the text shown in player `id`'s box.
///
/// Corresponds to `hidLaSetExplainText()` in `hid_la.h`.
///
/// libnx copies the bytes as they come; here they must be UTF-8, because that is
/// what the applet renders and rejecting a malformed string at the boundary is
/// cheaper than shipping one to another process.
///
/// # Safety
///
/// `arg` must be null or point to a writable `HidLaControllerSupportArg`, and
/// `text` must be null or point to a NUL-terminated string that stays valid for
/// the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_set_explain_text(
    arg: *mut ControllerSupportArg,
    text: *const c_char,
    id: u32,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so both
    // pointers are null or point to a valid value of their type. libnx
    // dereferences both unconditionally; a null one is rejected below rather
    // than followed.
    let (arg, text) = unsafe { (arg.as_mut(), text.as_ref().map(|text| CStr::from_ptr(text))) };

    let (Some(arg), Some(text)) = (arg, text) else {
        return GENERIC_ERROR;
    };

    let Ok(text) = text.to_str() else {
        return libnx_error(LibnxError::BadInput);
    };

    // Widening cast: `usize` is 64-bit here, and the index is bounds-checked
    // against the argument's player slots either way.
    match arg.set_explain_text(id as usize, text) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Opens the controller-support screen.
///
/// Corresponds to `hidLaShowControllerSupport()` in `hid_la.h`. Blocks until the
/// user leaves the applet, which only presents itself when the request is not
/// already satisfied.
///
/// # Safety
///
/// `result_info` must be null or point to a writable
/// `HidLaControllerSupportResultInfo`, and `arg` must point to a readable
/// `HidLaControllerSupportArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_show_controller_support(
    result_info: *mut ControllerSupportResultInfo,
    arg: *const ControllerSupportArg,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so both
    // pointers are null or point to a valid value of their type. libnx
    // documents `result_info` as optional and dereferences `arg`
    // unconditionally.
    let (result_info, arg) = unsafe { (result_info.as_mut(), arg.as_ref()) };

    let Some(arg) = arg else {
        return GENERIC_ERROR;
    };

    let context = match controller_context() {
        Ok(context) => context,
        Err(rc) => return rc,
    };

    show(ControllerSupport::Support { arg }, &context, result_info)
}

/// Opens the wrist-strap guide.
///
/// Corresponds to `hidLaShowControllerStrapGuide()` in `hid_la.h`. Available on
/// [3.0.0+]. Blocks until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_hid_la_show_controller_strap_guide() -> u32 {
    if hos_version::get() < HosVersion::new(3, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    let context = match controller_context() {
        Ok(context) => context,
        Err(rc) => return rc,
    };

    // libnx exposes no result slot on this screen.
    show(ControllerSupport::StrapGuide, &context, None)
}

/// Opens the controller firmware-update screen.
///
/// Corresponds to `hidLaShowControllerFirmwareUpdate()` in `hid_la.h`. Available
/// on [3.0.0+]. Blocks until the user leaves the applet.
///
/// # Safety
///
/// `arg` must point to a readable `HidLaControllerFirmwareUpdateArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_show_controller_firmware_update(
    arg: *const ControllerFirmwareUpdateArg,
) -> u32 {
    if hos_version::get() < HosVersion::new(3, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    // SAFETY: The caller upholds this function's `# Safety` contract, so `arg`
    // is null or points to a valid value of its type. libnx dereferences it
    // unconditionally; a null one is rejected here rather than followed.
    let Some(arg) = (unsafe { arg.as_ref() }) else {
        return GENERIC_ERROR;
    };

    let context = match controller_context() {
        Ok(context) => context,
        Err(rc) => return rc,
    };

    // libnx exposes no result slot on this screen.
    show(ControllerSupport::FirmwareUpdate { arg }, &context, None)
}

/// Opens the system's controller-support screen.
///
/// Corresponds to `hidLaShowControllerSupportForSystem()` in `hid_la.h`. Blocks
/// until the user leaves the applet. Unlike
/// [`__nx_rt_hbapp__libnx_hid_la_show_controller_support`] it always presents
/// it, and with `flag` set it presents it as qlaunch does.
///
/// # Safety
///
/// `result_info` must be null or point to a writable
/// `HidLaControllerSupportResultInfo`, and `arg` must point to a readable
/// `HidLaControllerSupportArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_show_controller_support_for_system(
    result_info: *mut ControllerSupportResultInfo,
    arg: *const ControllerSupportArg,
    flag: bool,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so both
    // pointers are null or point to a valid value of their type. libnx
    // documents `result_info` as optional and dereferences `arg`
    // unconditionally.
    let (result_info, arg) = unsafe { (result_info.as_mut(), arg.as_ref()) };

    let Some(arg) = arg else {
        return GENERIC_ERROR;
    };

    // Pre-3.0.0 this entry point does not consult the HID service at all: libnx
    // sends a cleared style set and a horizontal hold type instead.
    let context = if hos_version::get() >= HosVersion::new(3, 0, 0) {
        match controller_context() {
            Ok(context) => context,
            Err(rc) => return rc,
        }
    } else {
        ControllerSupportContext {
            npad_style_set: 0,
            npad_joy_hold_type: NpadJoyHoldType::Horizontal,
        }
    };

    show(
        ControllerSupport::SupportForSystem {
            arg,
            as_qlaunch: flag,
        },
        &context,
        result_info,
    )
}

/// Opens the system's controller firmware-update screen.
///
/// Corresponds to `hidLaShowControllerFirmwareUpdateForSystem()` in `hid_la.h`.
/// Available on [3.0.0+]. Blocks until the user leaves the applet.
///
/// # Safety
///
/// `arg` must point to a readable `HidLaControllerFirmwareUpdateArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_show_controller_firmware_update_for_system(
    arg: *const ControllerFirmwareUpdateArg,
    caller: u32,
) -> u32 {
    if hos_version::get() < HosVersion::new(3, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    // SAFETY: The caller upholds this function's `# Safety` contract, so `arg`
    // is null or points to a valid value of its type. libnx dereferences it
    // unconditionally; a null one is rejected here rather than followed.
    let Some(arg) = (unsafe { arg.as_ref() }) else {
        return GENERIC_ERROR;
    };

    let Some(caller) = ControllerSupportCaller::from_raw(caller) else {
        return libnx_error(LibnxError::BadInput);
    };

    let context = match controller_context() {
        Ok(context) => context,
        Err(rc) => return rc,
    };

    // libnx exposes no result slot on this screen.
    show(
        ControllerSupport::FirmwareUpdateForSystem { arg, caller },
        &context,
        None,
    )
}

/// Opens the system's key-remapping screen.
///
/// Corresponds to `hidLaShowControllerKeyRemappingForSystem()` in `hid_la.h`.
/// Available on [11.0.0+]. Blocks until the user leaves the applet.
///
/// # Safety
///
/// `arg` must point to a readable `HidLaControllerKeyRemappingArg`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_hid_la_show_controller_key_remapping_for_system(
    arg: *const ControllerKeyRemappingArg,
    caller: u32,
) -> u32 {
    if hos_version::get() < HosVersion::new(11, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    // SAFETY: The caller upholds this function's `# Safety` contract, so `arg`
    // is null or points to a valid value of its type. libnx dereferences it
    // unconditionally; a null one is rejected here rather than followed.
    let Some(arg) = (unsafe { arg.as_ref() }) else {
        return GENERIC_ERROR;
    };

    let Some(caller) = ControllerSupportCaller::from_raw(caller) else {
        return libnx_error(LibnxError::BadInput);
    };

    let context = match controller_context() {
        Ok(context) => context,
        Err(rc) => return rc,
    };

    // libnx exposes no result slot on this screen.
    show(
        ControllerSupport::KeyRemappingForSystem { arg, caller },
        &context,
        None,
    )
}
