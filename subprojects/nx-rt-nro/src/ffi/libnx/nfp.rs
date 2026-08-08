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

/// Borrows the caller's start parameters.
///
/// # Safety
///
/// `ptr` must be null or point to a readable `NfpLaAmiiboSettingsStartParam`.
unsafe fn borrow_start_param<'a>(ptr: *const c_void) -> Option<&'a AmiiboSettingsStartParam> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: The caller guarantees `ptr` is a live start-param struct, whose
    // layout `AmiiboSettingsStartParam` mirrors.
    Some(unsafe { &*ptr.cast::<AmiiboSettingsStartParam>() })
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_nfp_la_start_nickname_and_owner_settings(
    in_param: *const c_void,
    in_tag_info: *const NfcTagInfo,
    in_reg_info: *const NfpRegisterInfo,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
    reg_info_flag: *mut bool,
    out_reg_info: *mut NfpRegisterInfo,
) -> u32 {
    // SAFETY: The caller guarantees `in_param` is null or readable.
    let Some(start_param) = (unsafe { borrow_start_param(in_param) }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: The caller guarantees `handle` is null or writable; libnx writes
    // it unconditionally, so a null one is rejected rather than faulted on.
    let Some(handle) = (unsafe { handle.as_mut() }) else {
        return GENERIC_ERROR;
    };

    // SAFETY: The caller guarantees each is null or points to a readable value.
    let settings = AmiiboSettings::NicknameAndOwner {
        tag_info: unsafe { in_tag_info.as_ref() },
        register_info: unsafe { in_reg_info.as_ref() },
    };

    // SAFETY: The caller guarantees each is null or points to a writable value.
    let out = ReplyOut {
        tag_info: unsafe { out_tag_info.as_mut() },
        handle,
        reg_info_flag: unsafe { reg_info_flag.as_mut() },
        register_info: unsafe { out_reg_info.as_mut() },
    };

    start(settings, start_param, out)
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_nfp_la_start_game_data_eraser(
    in_param: *const c_void,
    in_tag_info: *const NfcTagInfo,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
) -> u32 {
    // SAFETY: The caller guarantees `in_param` is null or readable.
    let Some(start_param) = (unsafe { borrow_start_param(in_param) }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: As above; libnx writes `handle` unconditionally.
    let Some(handle) = (unsafe { handle.as_mut() }) else {
        return GENERIC_ERROR;
    };

    // SAFETY: The caller guarantees it is null or points to a readable value.
    let settings = AmiiboSettings::GameDataEraser {
        tag_info: unsafe { in_tag_info.as_ref() },
    };

    // SAFETY: The caller guarantees `out_tag_info` is null or writable. libnx
    // exposes no register-info slot on this screen.
    let out = ReplyOut {
        tag_info: unsafe { out_tag_info.as_mut() },
        handle,
        reg_info_flag: None,
        register_info: None,
    };

    start(settings, start_param, out)
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_nfp_la_start_restorer(
    in_param: *const c_void,
    in_tag_info: *const NfcTagInfo,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
) -> u32 {
    // SAFETY: The caller guarantees `in_param` is null or readable.
    let Some(start_param) = (unsafe { borrow_start_param(in_param) }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: As above; libnx writes `handle` unconditionally.
    let Some(handle) = (unsafe { handle.as_mut() }) else {
        return GENERIC_ERROR;
    };

    // SAFETY: The caller guarantees it is null or points to a readable value.
    let settings = AmiiboSettings::Restorer {
        tag_info: unsafe { in_tag_info.as_ref() },
    };

    // SAFETY: The caller guarantees `out_tag_info` is null or writable. libnx
    // exposes no register-info slot on this screen.
    let out = ReplyOut {
        tag_info: unsafe { out_tag_info.as_mut() },
        handle,
        reg_info_flag: None,
        register_info: None,
    };

    start(settings, start_param, out)
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_nfp_la_start_formatter(
    in_param: *const c_void,
    out_tag_info: *mut NfcTagInfo,
    handle: *mut NfcDeviceHandle,
) -> u32 {
    // SAFETY: The caller guarantees `in_param` is null or readable.
    let Some(start_param) = (unsafe { borrow_start_param(in_param) }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: As above; libnx writes `handle` unconditionally.
    let Some(handle) = (unsafe { handle.as_mut() }) else {
        return GENERIC_ERROR;
    };

    // SAFETY: The caller guarantees `out_tag_info` is null or writable. The
    // formatter takes no tag or register info in, and exposes no register-info
    // slot out.
    let out = ReplyOut {
        tag_info: unsafe { out_tag_info.as_mut() },
        handle,
        reg_info_flag: None,
        register_info: None,
    };

    start(AmiiboSettings::Formatter, start_param, out)
}
