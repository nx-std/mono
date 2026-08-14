//! Mii editor applet (`miiEdit` library applet) FFI.
//!
//! libnx's `mii_la.c` holds no file-local state, but every function in it
//! reaches the applet through `g_appletILibraryAppletCreator`, which is `static`
//! in `applet.c` and so cannot be aliased. Our `appletInitialize` override
//! replaces the only code that would populate it, so once `use_nx_service_applet`
//! is on, *every* libnx `miiLa*` function runs against a zeroed session.
//!
//! That is why this module covers the whole surface: a command left to libnx
//! does not fail cleanly. Here that costs nothing, because all six of libnx's
//! entry points are ported.
//!
//! # Firmware versions
//!
//! A service crate does not read the firmware version, so the three places
//! `mii_la.c` branches on it live here: `_miiLaGetVersion` picks
//! argument-storage version 3 below `[10.2.0]` and 4 from it, and both
//! `miiLaCreateMii` and `miiLaEditMii` refuse to run below `[10.2.0]` at all.
//!
//! # Nullability
//!
//! libnx dereferences every out pointer unconditionally, and reads
//! `valid_uuid_array` only when `count` is positive. Each entry point turns its
//! raw pointers into references once, rejecting a null one rather than faulting
//! on it. Past that conversion nothing here is raw or `unsafe`.

use nx_service_applet_mii::{
    MiiCharInfo,
    MiiCharInfoEdit,
    MiiCharInfoEditReply,
    MiiEdit,
    MiiEditReply,
    MiiSpecialKeyCode,
    Uuid,
    proto::VALID_UUID_ARRAY_LEN,
};
use nx_sf::error::ToResultCode as _;

use crate::{
    env::hos_version::{
        self,
        HosVersion,
    },
    ffi::common::{
        GENERIC_ERROR,
        LibnxError,
        libnx_error,
    },
    services::applet,
};

/// First firmware that speaks argument-storage version 4 and hosts the two
/// screens that edit a Mii without saving it.
const CHAR_INFO_EDITING_VERSION: HosVersion = HosVersion::new(10, 2, 0);

/// Maps a raw key code onto the two values libnx names.
///
/// libnx passes the caller's word straight into the argument storage, but the
/// applet reads it as a database selector, so a word naming no database is
/// rejected here rather than sent on.
fn special_key_code(raw: u32) -> Option<MiiSpecialKeyCode> {
    // Exact: `MiiSpecialKeyCode` is `#[repr(u32)]`, so each cast is the
    // discriminant the C caller passed in.
    const NORMAL: u32 = MiiSpecialKeyCode::Normal as u32;
    const SPECIAL: u32 = MiiSpecialKeyCode::Special as u32;

    match raw {
        NORMAL => Some(MiiSpecialKeyCode::Normal),
        SPECIAL => Some(MiiSpecialKeyCode::Special),
        _ => None,
    }
}

/// Borrows the uuids the applet will read from `valid_uuid_array`.
///
/// The applet reads at most [`VALID_UUID_ARRAY_LEN`] of them, so a longer count
/// is clamped rather than trusted: libnx clamps it too, in
/// `_miiLaInitializeValidUuidArray`, but only after the caller's pointer has
/// been taken at face value.
///
/// # Safety
///
/// `valid_uuid_array` must point to `count` readable [`Uuid`] values, unless
/// `count` is zero or negative.
unsafe fn borrow_uuids<'a>(valid_uuid_array: *const Uuid, count: i32) -> Option<&'a [Uuid]> {
    if count <= 0 {
        return Some(&[]);
    }
    if valid_uuid_array.is_null() {
        return None;
    }

    // Widening cast, then clamped: `count` is positive here, and the applet
    // never looks past the eighth entry.
    let len = (count as usize).min(VALID_UUID_ARRAY_LEN);

    // SAFETY: The caller guarantees the pointer covers `count` readable uuids,
    // and `len` is at most that. The borrow ends with the call it is handed to.
    Some(unsafe { core::slice::from_raw_parts(valid_uuid_array, len) })
}

/// Runs a database screen, addressing the applet as this firmware expects.
///
/// Shared by the four `miiLa*` entry points that answer with a database index.
/// The version split is `_miiLaGetVersion`.
fn show(request: MiiEdit<'_>, key_code: MiiSpecialKeyCode) -> Result<MiiEditReply, u32> {
    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return Err(GENERIC_ERROR);
    };

    let self_controller = self_controller.get();
    let creator = creator.get();

    let reply = if hos_version::get() >= CHAR_INFO_EDITING_VERSION {
        request.show_v4(&self_controller, &creator, key_code)
    } else {
        request.show_v3(&self_controller, &creator, key_code)
    };

    reply.map_err(|err| err.to_rc())
}

/// Runs a database screen and writes the index it produced.
///
/// libnx treats the applet's cancel status as a failure on every screen but
/// `ShowMiiEdit`, and reads the index only when the user completed the screen.
/// It names that failure `LibnxError_LibAppletBadExit`; `nx-sf`'s libnx
/// vocabulary has no such description, so it reports as [`GENERIC_ERROR`], the
/// same collapse the album and cabinet shims make.
fn show_for_index(request: MiiEdit<'_>, key_code: MiiSpecialKeyCode, index: &mut i32) -> u32 {
    match show(request, key_code) {
        Ok(MiiEditReply::Completed { index: value }) => {
            *index = value;
            0
        }
        Ok(MiiEditReply::Cancelled) => GENERIC_ERROR,
        Err(rc) => rc,
    }
}

/// Runs a Mii-editing screen and writes the Mii it produced.
///
/// Shared by the two `miiLa*` entry points that answer with a Mii. Both arrived
/// in `[10.2.0]`, and libnx refuses them below it rather than opening a screen
/// the applet does not have.
fn show_for_char_info(
    request: MiiCharInfoEdit<'_>,
    key_code: MiiSpecialKeyCode,
    out_char: &mut MiiCharInfo,
) -> u32 {
    if hos_version::get() < CHAR_INFO_EDITING_VERSION {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    match request.show_v4(&self_controller.get(), &creator.get(), key_code) {
        Ok(MiiCharInfoEditReply::Completed { char_info }) => {
            *out_char = char_info;
            0
        }
        Ok(MiiCharInfoEditReply::Cancelled) => GENERIC_ERROR,
        Err(err) => err.to_rc(),
    }
}

/// Opens the editor on the console's Mii database.
///
/// Corresponds to `miiLaShowMiiEdit()` in `mii_la.h`. Blocks until the user
/// leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_mii_la_show_mii_edit(special_key_code_raw: u32) -> u32 {
    let Some(key_code) = special_key_code(special_key_code_raw) else {
        return GENERIC_ERROR;
    };

    // libnx reports a cancelled `ShowMiiEdit` as success: the screen produces
    // nothing, so backing out of it is not a failure.
    match show(MiiEdit::Show, key_code) {
        Ok(_) => 0,
        Err(rc) => rc,
    }
}

/// Adds a Mii to the database.
///
/// Corresponds to `miiLaAppendMii()` in `mii_la.h`. Blocks until the user
/// leaves the applet.
///
/// # Safety
///
/// `index` must point to a writable `s32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_mii_la_append_mii(
    special_key_code_raw: u32,
    index: *mut i32,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so `index`
    // is null or points to a writable `s32`. Null becomes `None` here and is
    // rejected below rather than followed.
    let index = unsafe { index.as_mut() };

    let (Some(key_code), Some(index)) = (special_key_code(special_key_code_raw), index) else {
        return GENERIC_ERROR;
    };

    show_for_index(MiiEdit::AppendMii, key_code, index)
}

/// Adds a Mii image, offering the Miis named by `valid_uuid_array`.
///
/// Corresponds to `miiLaAppendMiiImage()` in `mii_la.h`. Blocks until the user
/// leaves the applet.
///
/// # Safety
///
/// `valid_uuid_array` must point to `count` readable `Uuid` values unless
/// `count` is zero or negative, and `index` must point to a writable `s32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_mii_la_append_mii_image(
    special_key_code_raw: u32,
    valid_uuid_array: *const Uuid,
    count: i32,
    index: *mut i32,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so the
    // array covers `count` readable uuids and `index` is null or writable. Null
    // becomes `None` here and is rejected below rather than followed.
    let (valid_uuids, index) = unsafe { (borrow_uuids(valid_uuid_array, count), index.as_mut()) };

    let (Some(key_code), Some(valid_uuids), Some(index)) =
        (special_key_code(special_key_code_raw), valid_uuids, index)
    else {
        return GENERIC_ERROR;
    };

    show_for_index(MiiEdit::AppendMiiImage { valid_uuids }, key_code, index)
}

/// Replaces the Mii image named by `used_uuid`.
///
/// Corresponds to `miiLaUpdateMiiImage()` in `mii_la.h`. Blocks until the user
/// leaves the applet.
///
/// # Safety
///
/// `valid_uuid_array` must point to `count` readable `Uuid` values unless
/// `count` is zero or negative, and `index` must point to a writable `s32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_mii_la_update_mii_image(
    special_key_code_raw: u32,
    valid_uuid_array: *const Uuid,
    count: i32,
    used_uuid: Uuid,
    index: *mut i32,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so the
    // array covers `count` readable uuids and `index` is null or writable. Null
    // becomes `None` here and is rejected below rather than followed.
    let (valid_uuids, index) = unsafe { (borrow_uuids(valid_uuid_array, count), index.as_mut()) };

    let (Some(key_code), Some(valid_uuids), Some(index)) =
        (special_key_code(special_key_code_raw), valid_uuids, index)
    else {
        return GENERIC_ERROR;
    };

    show_for_index(
        MiiEdit::UpdateMiiImage {
            valid_uuids,
            used_uuid,
        },
        key_code,
        index,
    )
}

/// Makes a Mii and returns it without saving it in the database.
///
/// Corresponds to `miiLaCreateMii()` in `mii_la.h`. Blocks until the user
/// leaves the applet.
///
/// # Safety
///
/// `out_char` must point to a writable `MiiCharInfo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_mii_la_create_mii(
    special_key_code_raw: u32,
    out_char: *mut MiiCharInfo,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so
    // `out_char` is null or points to a writable Mii. Null becomes `None` here
    // and is rejected below rather than followed.
    let out_char = unsafe { out_char.as_mut() };

    let (Some(key_code), Some(out_char)) = (special_key_code(special_key_code_raw), out_char)
    else {
        return GENERIC_ERROR;
    };

    show_for_char_info(MiiCharInfoEdit::Create, key_code, out_char)
}

/// Edits `in_char` and returns the result without saving it in the database.
///
/// Corresponds to `miiLaEditMii()` in `mii_la.h`. Blocks until the user leaves
/// the applet.
///
/// # Safety
///
/// `in_char` must point to a readable `MiiCharInfo` and `out_char` to a
/// writable one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_mii_la_edit_mii(
    special_key_code_raw: u32,
    in_char: *const MiiCharInfo,
    out_char: *mut MiiCharInfo,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so both
    // pointers are null or point to a valid Mii. Null becomes `None` here and
    // is rejected below rather than followed.
    let (in_char, out_char) = unsafe { (in_char.as_ref(), out_char.as_mut()) };

    let (Some(key_code), Some(in_char), Some(out_char)) =
        (special_key_code(special_key_code_raw), in_char, out_char)
    else {
        return GENERIC_ERROR;
    };

    show_for_char_info(
        MiiCharInfoEdit::Edit { char_info: in_char },
        key_code,
        out_char,
    )
}
