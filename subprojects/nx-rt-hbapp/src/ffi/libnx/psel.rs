//! Player-select applet (`playerSelect` library applet) FFI.
//!
//! libnx's `psel.c` holds no file-local state, but every function in it reaches
//! the applet through `g_appletILibraryAppletCreator`, which is `static` in
//! `applet.c` and so cannot be aliased. Our `appletInitialize` override replaces
//! the only code that would populate it, so once `use_nx_service_applet` is on,
//! *every* libnx `psel*` function runs against a zeroed session.
//!
//! That is why this module covers the whole surface: a command left to libnx
//! does not fail cleanly. Here that costs nothing, because all twelve of
//! libnx's exported entry points are ported. The three `pselUiSet*` helpers in
//! `psel.h` are not among them: they are `static inline`, compiled into the
//! caller, and write into the settings struct this module also defines.
//!
//! # Where the version branching lives
//!
//! `nx-service-applet-psel` exposes one entry point per library applet API
//! version and reads no system version of its own, because a service crate may
//! not depend on the runtime that holds it. The branches libnx writes in
//! `_pselGetLaVersion`, and the two screens it gates on [6.0.0+] and [13.0.0+],
//! are therefore reproduced here.
//!
//! # Where the account service is still libnx's
//!
//! The user-selector and user-creator entry points ask the account service
//! whether creating a user is permitted, and take a skip path that selects a
//! user without showing anything. No `acc` client is wired into this runtime,
//! so those two commands go to libnx's own `acc.c`, against the session
//! `accountInitialize` opened, exactly as libnx's `psel.c` does.

use nx_service_applet_psel::{
    AccountUid,
    PlayerSelect,
    proto::{
        UiMode,
        UiSettings,
        UserSelectionSettings,
        UserSelectionSettingsForSystemService,
    },
    show_ui_v1,
    show_ui_v2,
    show_ui_v6,
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

unsafe extern "C" {
    /// libnx `accountIsUserRegistrationRequestPermitted`.
    fn accountIsUserRegistrationRequestPermitted(out: *mut bool) -> u32;

    /// libnx `accountTrySelectUserWithoutInteraction`.
    fn accountTrySelectUserWithoutInteraction(
        uid: *mut AccountUid,
        is_network_service_account_required: bool,
    ) -> u32;
}

/// Whether the running system is at least `major.minor.patch`.
fn hos_at_least(major: u8, minor: u8, patch: u8) -> bool {
    hos_version::get() >= HosVersion::new(major, minor, patch)
}

/// Asks the account service whether creating a user is permitted.
///
/// On failure the account service's own result code comes back, for the caller
/// to hand straight to C.
fn user_registration_permitted() -> Result<bool, u32> {
    let mut permitted = false;

    // SAFETY: The out-pointer addresses a live local. The command needs nothing
    // else beyond the account session its caller opened, which libnx owns.
    let rc = unsafe { accountIsUserRegistrationRequestPermitted(&raw mut permitted) };
    if rc != 0 {
        return Err(rc);
    }

    Ok(permitted)
}

/// Shows `request`, addressing the applet the way the running system takes it.
///
/// libnx picks the version in `_pselGetLaVersion`: 0x20000 on [6.0.0+], 0x10000
/// on [2.0.0+], 0x1 before that.
fn show(request: PlayerSelect<'_>, out_user: Option<&mut AccountUid>) -> u32 {
    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };
    let self_controller = self_controller.get();
    let creator = creator.get();

    let shown = if hos_at_least(6, 0, 0) {
        request.show_v6(&self_controller, &creator)
    } else if hos_at_least(2, 0, 0) {
        request.show_v2(&self_controller, &creator)
    } else {
        request.show_v1(&self_controller, &creator)
    };

    match shown {
        Ok(user) => {
            if let Some(out_user) = out_user {
                *out_user = user;
            }
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Shows caller-built argument storage, addressing the applet the way the
/// running system takes it.
///
/// The version split is the same one [`show`] applies; here it also decides how
/// much of `ui` is sent.
fn show_ui(ui: &UiSettings, out_user: Option<&mut AccountUid>) -> u32 {
    let (Some(self_controller), Some(creator)) = (
        applet::get_self_controller(),
        applet::get_library_applet_creator(),
    ) else {
        return GENERIC_ERROR;
    };
    let self_controller = self_controller.get();
    let creator = creator.get();

    let shown = if hos_at_least(6, 0, 0) {
        show_ui_v6(&self_controller, &creator, ui)
    } else if hos_at_least(2, 0, 0) {
        show_ui_v2(&self_controller, &creator, ui)
    } else {
        show_ui_v1(&self_controller, &creator, ui)
    };

    match shown {
        Ok(user) => {
            if let Some(out_user) = out_user {
                *out_user = user;
            }
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// Selects a user without showing the applet when the settings ask for it, and
/// shows it otherwise.
///
/// libnx's `_pselShowUserSelectorCommon`. `settings` is the same one `request`
/// carries: the skip decision is read from it before the applet is reached.
fn show_user_selector(
    request: PlayerSelect<'_>,
    settings: &UserSelectionSettings,
    out_user: &mut AccountUid,
) -> u32 {
    if settings.is_skip_enabled != 0 {
        // Skipping the applet contradicts naming an excluded user or asking for
        // an additional selection; libnx rejects the pair outright.
        if settings.invalid_uid_list[0].is_valid() || settings.additional_select != 0 {
            return GENERIC_ERROR;
        }

        // SAFETY: The out-pointer addresses the caller's slot, which the entry
        // point checked is non-null.
        let rc = unsafe {
            accountTrySelectUserWithoutInteraction(
                &raw mut *out_user,
                settings.is_network_service_account_required != 0,
            )
        };
        if rc != 0 {
            return rc;
        }

        // A user came back, so there is nothing left to ask the user about.
        if out_user.is_valid() {
            return 0;
        }
    }

    show(request, Some(out_user))
}

/// Clears `ui` and opens it on `mode`.
///
/// Corresponds to `pselUiCreate()` in `psel.h`.
///
/// # Safety
///
/// `ui` must point to a writable `PselUiSettings`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_psel_ui_create(
    ui: *mut UiSettings,
    mode: u32,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so `ui` is
    // null or points to a writable settings struct.
    let ui = unsafe { ui.as_mut() };

    // libnx takes `mode` as a C enum and never checks it. Here it arrives as a
    // bare integer, so a value naming no screen is rejected rather than written
    // into the storage the applet reads.
    let (Some(ui), Some(mode)) = (ui, UiMode::from_raw(mode)) else {
        return GENERIC_ERROR;
    };

    *ui = UiSettings::new(mode);

    0
}

/// Records `user` in the first free slot of `ui`'s user list.
///
/// Corresponds to `pselUiAddUser()` in `psel.h`.
///
/// # Safety
///
/// `ui` must point to a writable `PselUiSettings`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_psel_ui_add_user(
    ui: *mut UiSettings,
    user: AccountUid,
) {
    // SAFETY: The caller upholds this function's `# Safety` contract, so `ui` is
    // null or points to a writable settings struct.
    let Some(ui) = (unsafe { ui.as_mut() }) else {
        return;
    };

    ui.add_user(user);
}

/// Shows the applet with the settings `ui` holds.
///
/// Corresponds to `pselUiShow()` in `psel.h`. Blocks until the user leaves the
/// applet.
///
/// # Safety
///
/// `ui` must point to a readable `PselUiSettings`, and `out_user` must be null
/// or point to a writable `AccountUid`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_psel_ui_show(
    ui: *mut UiSettings,
    out_user: *mut AccountUid,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so each
    // pointer is null or points to a valid value of its type. Null becomes
    // `None` here; the settings libnx dereferences unconditionally are rejected
    // below rather than followed.
    let (ui, out_user) = unsafe { (ui.as_ref(), out_user.as_mut()) };

    let Some(ui) = ui else {
        return GENERIC_ERROR;
    };

    // The storage goes to the applet as the caller assembled it, which is what
    // `pselUiShow` is. Its `mode` is not re-checked here: the boundary that
    // writes it is `pselUiCreate` above, and rejecting a value at this end would
    // refuse a caller libnx serves.
    show_ui(ui, out_user)
}

/// Shows the user selector on behalf of a system service.
///
/// Corresponds to `pselShowUserSelectorForSystem()` in `psel.h`. Blocks until
/// the user leaves the applet, unless the settings allow skipping it.
///
/// # Safety
///
/// Every pointer must point to a valid value of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_selector_for_system(
    out_user: *mut AccountUid,
    settings: *const UserSelectionSettings,
    settings_system: *const UserSelectionSettingsForSystemService,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so each
    // pointer is null or points to a valid value of its type. libnx dereferences
    // all three, so all three are rejected below when null; libnx reads
    // `settings_system` only on [2.0.0+], but its header does not sanction a
    // null there either.
    let (out_user, settings, settings_system) = unsafe {
        (
            out_user.as_mut(),
            settings.as_ref(),
            settings_system.as_ref(),
        )
    };

    let (Some(out_user), Some(settings), Some(system_settings)) =
        (out_user, settings, settings_system)
    else {
        return GENERIC_ERROR;
    };

    let allow_user_creation = match user_registration_permitted() {
        Ok(permitted) => permitted,
        Err(rc) => return rc,
    };

    show_user_selector(
        PlayerSelect::UserSelectorForSystem {
            settings,
            allow_user_creation,
            system_settings,
        },
        settings,
        out_user,
    )
}

/// Shows the user selector on behalf of an application about to be launched.
///
/// Corresponds to `pselShowUserSelectorForLauncher()` in `psel.h`. Blocks until
/// the user leaves the applet, unless the settings allow skipping it.
///
/// # Safety
///
/// Every pointer must point to a valid value of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_selector_for_launcher(
    out_user: *mut AccountUid,
    settings: *const UserSelectionSettings,
    application_id: u64,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so each
    // pointer is null or points to a valid value of its type. Both are ones
    // libnx dereferences, so both are rejected below when null.
    let (out_user, settings) = unsafe { (out_user.as_mut(), settings.as_ref()) };

    let (Some(out_user), Some(settings)) = (out_user, settings) else {
        return GENERIC_ERROR;
    };

    let allow_user_creation = match user_registration_permitted() {
        Ok(permitted) => permitted,
        Err(rc) => return rc,
    };

    show_user_selector(
        PlayerSelect::UserSelectorForLauncher {
            settings,
            allow_user_creation,
            application_id,
        },
        settings,
        out_user,
    )
}

/// Shows the user selector.
///
/// Corresponds to `pselShowUserSelector()` in `psel.h`. Blocks until the user
/// leaves the applet, unless the settings allow skipping it.
///
/// # Safety
///
/// Every pointer must point to a valid value of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_selector(
    out_user: *mut AccountUid,
    settings: *const UserSelectionSettings,
) -> u32 {
    // SAFETY: The caller upholds this function's `# Safety` contract, so each
    // pointer is null or points to a valid value of its type. Both are ones
    // libnx dereferences, so both are rejected below when null.
    let (out_user, settings) = unsafe { (out_user.as_mut(), settings.as_ref()) };

    let (Some(out_user), Some(settings)) = (out_user, settings) else {
        return GENERIC_ERROR;
    };

    let allow_user_creation = match user_registration_permitted() {
        Ok(permitted) => permitted,
        Err(rc) => return rc,
    };

    show_user_selector(
        PlayerSelect::UserSelector {
            settings,
            allow_user_creation,
        },
        settings,
        out_user,
    )
}

/// Shows the screen that creates a user.
///
/// Corresponds to `pselShowUserCreator()` in `psel.h`. Blocks until the user
/// leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_creator() -> u32 {
    let permitted = match user_registration_permitted() {
        Ok(permitted) => permitted,
        Err(rc) => return rc,
    };

    // libnx refuses the screen outright rather than letting the applet turn the
    // user away.
    if !permitted {
        return GENERIC_ERROR;
    }

    show(PlayerSelect::UserCreator, None)
}

/// Shows the screen that edits a user's icon.
///
/// Corresponds to `pselShowUserIconEditor()` in `psel.h`. Blocks until the user
/// leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_icon_editor(user: AccountUid) -> u32 {
    show(PlayerSelect::UserIconEditor { user }, None)
}

/// Shows the screen that edits a user's nickname.
///
/// Corresponds to `pselShowUserNicknameEditor()` in `psel.h`. Blocks until the
/// user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_nickname_editor(user: AccountUid) -> u32 {
    show(PlayerSelect::UserNicknameEditor { user }, None)
}

/// Shows the screen the starter applet creates a user with during console
/// setup.
///
/// Corresponds to `pselShowUserCreatorForStarter()` in `psel.h`. Blocks until
/// the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_creator_for_starter() -> u32 {
    show(PlayerSelect::UserCreatorForStarter, None)
}

/// Shows the screen that links a user to a Nintendo Account NNID.
///
/// Corresponds to `pselShowNintendoAccountNnidLinker()` in `psel.h`. Blocks
/// until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_psel_show_nintendo_account_nnid_linker(
    user: AccountUid,
) -> u32 {
    // The screen arrived in [6.0.0].
    if !hos_at_least(6, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    show(PlayerSelect::NintendoAccountNnidLinker { user }, None)
}

/// Shows the screen that promotes a user's qualification.
///
/// Corresponds to `pselShowUserQualificationPromoter()` in `psel.h`. Blocks
/// until the user leaves the applet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_psel_show_user_qualification_promoter(
    user: AccountUid,
) -> u32 {
    // The screen arrived in [13.0.0].
    if !hos_at_least(13, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    show(PlayerSelect::UserQualificationPromoter { user }, None)
}
