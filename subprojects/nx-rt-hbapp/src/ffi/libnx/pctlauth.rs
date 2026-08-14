//! Parental Controls auth applet (`auth` library applet) FFI.
//!
//! libnx's `pctlauth.c` holds no file-local state, but every function in it
//! reaches the applet through `g_appletILibraryAppletCreator`, which is `static`
//! in `applet.c` and so cannot be aliased. Our `appletInitialize` override
//! replaces the only code that would populate it, so once `use_nx_service_applet`
//! is on, *every* libnx `pctlauth*` function runs against a zeroed session.
//!
//! That is why this module covers the whole surface: a command left to libnx
//! does not fail cleanly. Here that costs nothing, because all five of libnx's
//! entry points are ported.
//!
//! # System version
//!
//! libnx addresses the applet with library-applet API version 1, or 2 from
//! 4.0.0, and rejects `pctlauthShowEx` outright below 4.0.0. The version is the
//! runtime's fact, so the branch lives here rather than in the service crate,
//! which exposes one method per version.

use nx_service_applet_pctlauth::ParentalAuth;
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

/// First system version addressed with library-applet API version 2, and the
/// first that reads the second and third argument bytes.
const LA_VERSION_2_SINCE: HosVersion = HosVersion::new(4, 0, 0);

/// Shows `request`, addressing the applet as the running system version expects.
///
/// Shared by the five `pctlauth*` entry points. libnx picks the API version with
/// `hosversionAtLeast(4,0,0)` inside its own shared helper; here the version is
/// the choice of method on [`ParentalAuth`].
fn show(request: ParentalAuth) -> u32 {
    let (Some(self_controller_ref), Some(creator_ref)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };

    let self_controller = self_controller_ref.get();
    let creator = creator_ref.get();

    let result = if hos_version::get() >= LA_VERSION_2_SINCE {
        request.show_v2(&self_controller, &creator)
    } else {
        request.show_v1(&self_controller, &creator)
    };

    match result {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Shows `request` on a system that implements it, rejecting older ones.
///
/// Shared by the two entry points libnx documents as 4.0.0+: it guards
/// `pctlauthShowEx` and reaches `pctlauthShowForConfiguration` through it, so
/// both report `LibnxError_IncompatSysVer` below that version.
fn show_ex(request: ParentalAuth) -> u32 {
    if hos_version::get() < LA_VERSION_2_SINCE {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    show(request)
}

/// Asks the user for the Parental Controls PIN.
///
/// Corresponds to `pctlauthShow()` in `pctlauth.h`. `flag` false temporarily
/// disables Parental Controls; true validates the PIN the user enters. Blocks
/// until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_pctlauth_show(flag: bool) -> u32 {
    show(ParentalAuth::authenticate(flag))
}

/// Asks the user for the Parental Controls PIN, with all three argument bytes.
///
/// Corresponds to `pctlauthShowEx()` in `pctlauth.h`, which libnx documents as
/// 4.0.0+. Blocks until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_pctlauth_show_ex(arg0: u8, arg1: u8, arg2: u8) -> u32 {
    show_ex(ParentalAuth::Authenticate { arg0, arg1, arg2 })
}

/// Asks the user for the Parental Controls PIN the way the system settings do.
///
/// Corresponds to `pctlauthShowForConfiguration()` in `pctlauth.h`, which is
/// exactly `pctlauthShowEx(1, 0, 1)`. Blocks until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_pctlauth_show_for_configuration() -> u32 {
    show_ex(ParentalAuth::authenticate_for_configuration())
}

/// Registers the Parental Controls PIN.
///
/// Corresponds to `pctlauthRegisterPasscode()` in `pctlauth.h`. Blocks until the
/// user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_pctlauth_register_passcode() -> u32 {
    show(ParentalAuth::RegisterPasscode)
}

/// Changes the Parental Controls PIN.
///
/// Corresponds to `pctlauthChangePasscode()` in `pctlauth.h`. Blocks until the
/// user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_pctlauth_change_passcode() -> u32 {
    show(ParentalAuth::ChangePasscode)
}
