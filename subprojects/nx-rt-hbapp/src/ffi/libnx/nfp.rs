//! Cabinet applet (`cabinet` library applet) FFI.
//!
//! libnx's `nfp_la.c` holds no file-local state, but every function in it
//! reaches the applet through `g_appletILibraryAppletCreator`, which is `static`
//! in `applet.c` and so cannot be aliased. Our `appletInitialize` override
//! replaces the only code that would populate it, so once `use_nx_service_applet`
//! is on, *every* libnx `nfpLa*` function runs against a zeroed session.
//!
//! That is why this module covers the whole surface: a command left to libnx
//! does not fail cleanly. Here that costs nothing, because all four of libnx's
//! entry points are ported.
//!
//! # Nullability
//!
//! libnx documents `in_tag_info`, `in_reg_info`, `reg_info_flag` and
//! `out_tag_info` as optional, and dereferences `in_param` and `handle`
//! unconditionally. Each entry point turns its raw pointers into references
//! once, rejecting the two mandatory ones when they are null rather than
//! faulting on them. Past that conversion nothing here is raw or `unsafe`:
//! [`ReplyOut`] carries `Option<&mut _>`, so a screen that exposes no
//! register-info slot passes [`None`] rather than a null pointer.

use core::ffi::c_void;

use nx_service_applet_nfp::{
    AmiiboSettings,
    NfcDeviceHandle,
    NfcTagInfo,
    NfpRegisterInfo,
    proto::AmiiboSettingsStartParam,
};
use nx_sf::error::ToResultCode as _;

use crate::{
    ffi::common::GENERIC_ERROR,
    services::applet,
};

/// The caller's slots for the applet's reply.
///
/// `handle` is the one libnx writes unconditionally; the rest are optional, and
/// a screen that does not expose a slot leaves it [`None`].
struct ReplyOut<'a> {
    /// The amiibo the applet acted on.
    tag_info: Option<&'a mut NfcTagInfo>,
    /// The device the applet used.
    handle: &'a mut NfcDeviceHandle,
    /// Whether the applet reported registration data.
    reg_info_flag: Option<&'a mut bool>,
    /// The registration data itself.
    register_info: Option<&'a mut NfpRegisterInfo>,
}

/// Runs `settings` and scatters the reply into `out`.
///
/// Shared by the four `nfpLaStart*` entry points, which differ only in the
/// screen they open and in how many slots they expose.
fn start(
    settings: AmiiboSettings<'_>,
    start_param: &AmiiboSettingsStartParam,
    out: ReplyOut<'_>,
) -> u32 {
    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    let reply = match settings.start(&self_controller.get(), &creator.get(), start_param) {
        Ok(reply) => reply,
        Err(err) => return err.to_rc(),
    };

    *out.handle = reply.handle;

    if let Some(tag_info) = out.tag_info {
        *tag_info = reply.tag_info;
    }

    // libnx reports the flag whenever the slot is given, and copies the data
    // only when the applet actually set it.
    if let Some(reg_info_flag) = out.reg_info_flag {
        *reg_info_flag = reply.register_info.is_some();
    }
    if let (Some(register_info), Some(slot)) = (reply.register_info, out.register_info) {
        *slot = register_info;
    }

    0
}

/// Opens the applet on the nickname-and-owner screen.
///
/// Corresponds to `nfpLaStartNicknameAndOwnerSettings()` in `nfp_la.h`. Blocks
/// until the user leaves the applet.
///
/// # Safety
///
/// Every pointer must be null or point to a valid value of its type, and
/// `in_param` and `handle` must be non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_nfp_la_start_nickname_and_owner_settings(
    in_param: *const c_void,
    in_tag_info: *const NfcTagInfo,
    in_reg_info: *const NfpRegisterInfo,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
    reg_info_flag: *mut bool,
    out_reg_info: *mut NfpRegisterInfo,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so every
    // pointer is null or points to a valid value of its type. Null becomes
    // `None` here; the two libnx dereferences unconditionally are rejected
    // below rather than followed.
    let (start_param, tag_info, register_info, out_tag_info, handle, reg_info_flag, out_reg_info) = unsafe {
        (
            in_param.cast::<AmiiboSettingsStartParam>().as_ref(),
            in_tag_info.as_ref(),
            in_reg_info.as_ref(),
            out_tag_info.as_mut(),
            handle.as_mut(),
            reg_info_flag.as_mut(),
            out_reg_info.as_mut(),
        )
    };

    let (Some(start_param), Some(handle)) = (start_param, handle) else {
        return GENERIC_ERROR;
    };

    start(
        AmiiboSettings::NicknameAndOwner {
            tag_info,
            register_info,
        },
        start_param,
        ReplyOut {
            tag_info: out_tag_info,
            handle,
            reg_info_flag,
            register_info: out_reg_info,
        },
    )
}

/// Opens the applet on the game-data eraser screen.
///
/// Corresponds to `nfpLaStartGameDataEraser()` in `nfp_la.h`. Blocks until the
/// user leaves the applet.
///
/// # Safety
///
/// Every pointer must be null or point to a valid value of its type, and
/// `in_param` and `handle` must be non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_nfp_la_start_game_data_eraser(
    in_param: *const c_void,
    in_tag_info: *const NfcTagInfo,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so every
    // pointer is null or points to a valid value of its type. Null becomes
    // `None` here; the two libnx dereferences unconditionally are rejected
    // below rather than followed.
    let (start_param, tag_info, out_tag_info, handle) = unsafe {
        (
            in_param.cast::<AmiiboSettingsStartParam>().as_ref(),
            in_tag_info.as_ref(),
            out_tag_info.as_mut(),
            handle.as_mut(),
        )
    };

    let (Some(start_param), Some(handle)) = (start_param, handle) else {
        return GENERIC_ERROR;
    };

    // libnx exposes no register-info slot on this screen.
    start(
        AmiiboSettings::GameDataEraser { tag_info },
        start_param,
        ReplyOut {
            tag_info: out_tag_info,
            handle,
            reg_info_flag: None,
            register_info: None,
        },
    )
}

/// Opens the applet on the restorer screen.
///
/// Corresponds to `nfpLaStartRestorer()` in `nfp_la.h`. Blocks until the user
/// leaves the applet.
///
/// # Safety
///
/// Every pointer must be null or point to a valid value of its type, and
/// `in_param` and `handle` must be non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_nfp_la_start_restorer(
    in_param: *const c_void,
    in_tag_info: *const NfcTagInfo,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so every
    // pointer is null or points to a valid value of its type. Null becomes
    // `None` here; the two libnx dereferences unconditionally are rejected
    // below rather than followed.
    let (start_param, tag_info, out_tag_info, handle) = unsafe {
        (
            in_param.cast::<AmiiboSettingsStartParam>().as_ref(),
            in_tag_info.as_ref(),
            out_tag_info.as_mut(),
            handle.as_mut(),
        )
    };

    let (Some(start_param), Some(handle)) = (start_param, handle) else {
        return GENERIC_ERROR;
    };

    // libnx exposes no register-info slot on this screen.
    start(
        AmiiboSettings::Restorer { tag_info },
        start_param,
        ReplyOut {
            tag_info: out_tag_info,
            handle,
            reg_info_flag: None,
            register_info: None,
        },
    )
}

/// Opens the applet on the formatter screen.
///
/// Corresponds to `nfpLaStartFormatter()` in `nfp_la.h`. Blocks until the user
/// leaves the applet.
///
/// # Safety
///
/// Every pointer must be null or point to a valid value of its type, and
/// `in_param` and `handle` must be non-null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_nfp_la_start_formatter(
    in_param: *const c_void,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so every
    // pointer is null or points to a valid value of its type. Null becomes
    // `None` here; the two libnx dereferences unconditionally are rejected
    // below rather than followed.
    let (start_param, out_tag_info, handle) = unsafe {
        (
            in_param.cast::<AmiiboSettingsStartParam>().as_ref(),
            out_tag_info.as_mut(),
            handle.as_mut(),
        )
    };

    let (Some(start_param), Some(handle)) = (start_param, handle) else {
        return GENERIC_ERROR;
    };

    // The formatter takes no tag or register info in, and exposes no
    // register-info slot out.
    start(
        AmiiboSettings::Formatter,
        start_param,
        ReplyOut {
            tag_info: out_tag_info,
            handle,
            reg_info_flag: None,
            register_info: None,
        },
    )
}
