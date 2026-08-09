//! Opening the player-select applet on a chosen screen.

use nx_service_acc::AccountUid;
use nx_service_applet::{
    AppletId,
    LibraryApplet,
    LibraryAppletCreator,
    LibraryAppletExitReason,
    LibraryAppletMode,
    SelfController,
    library_applet::{
        self,
        LaunchError,
    },
};
use zerocopy::{
    FromZeros as _,
    IntoBytes as _,
};

use crate::proto::{
    LaVersion,
    UiMode,
    UiReturnArg,
    UiSettings,
    UserSelectionSettings,
    UserSelectionSettingsForSystemService,
};

/// Which player-select screen to open, and the data that screen accepts.
///
/// libnx exposes one `pselShow*` entry point per variant. Which data a screen
/// accepts is fixed, so it is carried by the variant here and a combination
/// libnx would reject cannot be built.
///
/// Two of the screens arrived after [1.0.0]:
/// [`NintendoAccountNnidLinker`](Self::NintendoAccountNnidLinker) on [6.0.0+]
/// and [`UserQualificationPromoter`](Self::UserQualificationPromoter) on
/// [13.0.0+]. This crate does not read the running system version, so refusing
/// a screen the console does not have is the caller's to do.
#[derive(Debug, Clone, Copy)]
pub enum PlayerSelect<'a> {
    /// Select one of the users on the console.
    UserSelector {
        /// How the selection is presented and constrained.
        settings: &'a UserSelectionSettings,
        /// Whether the screen offers to create a new user.
        ///
        /// libnx fills this from `accountIsUserRegistrationRequestPermitted`.
        /// The account service is not this crate's to talk to, so the answer
        /// comes in with the request.
        allow_user_creation: bool,
    },
    /// Select a user on behalf of an application about to be launched.
    UserSelectorForLauncher {
        /// How the selection is presented and constrained.
        settings: &'a UserSelectionSettings,
        /// Whether the screen offers to create a new user.
        allow_user_creation: bool,
        /// The application the selection is made for.
        application_id: u64,
    },
    /// Select a user on behalf of a system service.
    UserSelectorForSystem {
        /// How the selection is presented and constrained.
        settings: &'a UserSelectionSettings,
        /// Whether the screen offers to create a new user.
        allow_user_creation: bool,
        /// The settings only a system service passes.
        system_settings: &'a UserSelectionSettingsForSystemService,
    },
    /// Create a user.
    ///
    /// libnx refuses this screen outright when
    /// `accountIsUserRegistrationRequestPermitted` says no. That check belongs
    /// to whoever holds the account session, not here.
    UserCreator,
    /// Edit a user's icon.
    UserIconEditor {
        /// The user to edit.
        user: AccountUid,
    },
    /// Edit a user's nickname.
    UserNicknameEditor {
        /// The user to edit.
        user: AccountUid,
    },
    /// Create a user, as the starter applet does during console setup.
    UserCreatorForStarter,
    /// [6.0.0+] Link a user to a Nintendo Account NNID.
    NintendoAccountNnidLinker {
        /// The user to link.
        user: AccountUid,
    },
    /// [13.0.0+] Promote a user's qualification.
    UserQualificationPromoter {
        /// The user to promote.
        user: AccountUid,
    },
}

impl PlayerSelect<'_> {
    /// Opens the applet on this screen as [1.0.0] takes it.
    ///
    /// See [`show_ui_v1`] for what the version decides and what the call costs.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show_v1(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<AccountUid, ShowError> {
        show_ui(
            self_controller,
            creator,
            &self.settings(LaVersion::V1),
            LaVersion::V1,
        )
    }

    /// Opens the applet on this screen as [2.0.0+] takes it.
    ///
    /// See [`show_ui_v2`] for what the version decides and what the call costs.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show_v2(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<AccountUid, ShowError> {
        show_ui(
            self_controller,
            creator,
            &self.settings(LaVersion::V2),
            LaVersion::V2,
        )
    }

    /// Opens the applet on this screen as [6.0.0+] takes it.
    ///
    /// See [`show_ui_v6`] for what the version decides and what the call costs.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show_v6(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<AccountUid, ShowError> {
        show_ui(
            self_controller,
            creator,
            &self.settings(LaVersion::V6),
            LaVersion::V6,
        )
    }

    /// Builds the argument storage this request is shown with at `version`.
    fn settings(self, version: LaVersion) -> UiSettings {
        let mut ui = UiSettings::new(self.mode());

        match self {
            Self::UserSelector {
                settings,
                allow_user_creation,
            } => {
                init_user_selector(&mut ui, settings, allow_user_creation, version);
                if version >= LaVersion::V2 {
                    ui.settings.unk_x96 = 1;
                }
            }
            Self::UserSelectorForLauncher {
                settings,
                allow_user_creation,
                application_id,
            } => {
                init_user_selector(&mut ui, settings, allow_user_creation, version);
                ui.settings.application_id = application_id;
                ui.settings.unk_x92 = 1;
                if version >= LaVersion::V2 {
                    ui.settings.unk_x96 = 1;
                }
            }
            Self::UserSelectorForSystem {
                settings,
                allow_user_creation,
                system_settings,
            } => {
                init_user_selector(&mut ui, settings, allow_user_creation, version);
                ui.settings.unk_x92 = 1;
                if version >= LaVersion::V2 {
                    ui.settings.unk_x96 = system_settings.enable_user_creation_button;
                    ui.unk_x98 = system_settings.purpose;
                }
            }
            // Every screen but the user selector reads the first list entry as
            // its input user.
            Self::UserIconEditor { user }
            | Self::UserNicknameEditor { user }
            | Self::NintendoAccountNnidLinker { user }
            | Self::UserQualificationPromoter { user } => ui.add_user(user),
            // The two creator screens are opened on the mode alone.
            Self::UserCreator | Self::UserCreatorForStarter => {}
        }

        ui
    }

    /// Returns the screen this request opens.
    const fn mode(self) -> UiMode {
        match self {
            Self::UserSelector { .. }
            | Self::UserSelectorForLauncher { .. }
            | Self::UserSelectorForSystem { .. } => UiMode::UserSelector,
            Self::UserCreator => UiMode::UserCreator,
            Self::UserIconEditor { .. } => UiMode::UserIconEditor,
            Self::UserNicknameEditor { .. } => UiMode::UserNicknameEditor,
            Self::UserCreatorForStarter => UiMode::UserCreatorForStarter,
            Self::NintendoAccountNnidLinker { .. } => UiMode::NintendoAccountNnidLinker,
            Self::UserQualificationPromoter { .. } => UiMode::UserQualificationPromoter,
        }
    }
}

/// Fills the members every user-selector screen shares.
///
/// libnx calls this `_pselUserSelectorCommonInit`, minus the account query it
/// performs there and this crate takes as [`PlayerSelect`] data.
fn init_user_selector(
    ui: &mut UiSettings,
    settings: &UserSelectionSettings,
    allow_user_creation: bool,
    version: LaVersion,
) {
    ui.settings.is_permitted = u8::from(allow_user_creation);
    ui.settings.invalid_uid_list = settings.invalid_uid_list;
    ui.settings.is_network_service_account_required = settings.is_network_service_account_required;
    ui.settings.is_skip_enabled = settings.is_skip_enabled;
    ui.settings.show_skip_button = settings.show_skip_button;
    ui.settings.additional_select = settings.additional_select;

    if version >= LaVersion::V6 {
        // The applet member is the complement of the settings member; libnx
        // writes the same `^ 1`.
        ui.settings.unk_x97 = settings.is_unqualified_user_selectable ^ 1;
    }
}

/// Shows the applet with caller-built argument storage, as [1.0.0] takes it.
///
/// Only [`UiSettings::settings`] is sent: the two members [2.0.0+] added are
/// past the end of the storage this version of the applet reads.
///
/// This blocks on the user, so it must not be called from a context that cannot
/// wait indefinitely, and it performs IPC, so it must not be called from one
/// where IPC may already be broken.
///
/// # Errors
///
/// Returns a [`ShowError`] when the applet could not be presented, exited
/// abnormally, or reported a failure of its own.
pub fn show_ui_v1(
    self_controller: &SelfController<'_>,
    creator: &LibraryAppletCreator<'_>,
    ui: &UiSettings,
) -> Result<AccountUid, ShowError> {
    show_ui(self_controller, creator, ui, LaVersion::V1)
}

/// Shows the applet with caller-built argument storage, as [2.0.0+] takes it.
///
/// The whole of [`UiSettings`] is sent.
///
/// This blocks on the user, so it must not be called from a context that cannot
/// wait indefinitely, and it performs IPC, so it must not be called from one
/// where IPC may already be broken.
///
/// # Errors
///
/// Returns a [`ShowError`] when the applet could not be presented, exited
/// abnormally, or reported a failure of its own.
pub fn show_ui_v2(
    self_controller: &SelfController<'_>,
    creator: &LibraryAppletCreator<'_>,
    ui: &UiSettings,
) -> Result<AccountUid, ShowError> {
    show_ui(self_controller, creator, ui, LaVersion::V2)
}

/// Shows the applet with caller-built argument storage, as [6.0.0+] takes it.
///
/// The whole of [`UiSettings`] is sent, and the applet reads
/// [`UiSettingsV1::unk_x97`](crate::proto::UiSettingsV1::unk_x97) on top of what
/// [`show_ui_v2`] sends.
///
/// This blocks on the user, so it must not be called from a context that cannot
/// wait indefinitely, and it performs IPC, so it must not be called from one
/// where IPC may already be broken.
///
/// # Errors
///
/// Returns a [`ShowError`] when the applet could not be presented, exited
/// abnormally, or reported a failure of its own.
pub fn show_ui_v6(
    self_controller: &SelfController<'_>,
    creator: &LibraryAppletCreator<'_>,
    ui: &UiSettings,
) -> Result<AccountUid, ShowError> {
    show_ui(self_controller, creator, ui, LaVersion::V6)
}

/// Runs the applet at `version` and reports the user it ended on.
///
/// Shared by the three `show_ui_*` entry points, which differ only in the
/// version.
fn show_ui(
    self_controller: &SelfController<'_>,
    creator: &LibraryAppletCreator<'_>,
    ui: &UiSettings,
    version: LaVersion,
) -> Result<AccountUid, ShowError> {
    let applet = LibraryApplet {
        id: AppletId::LibraryAppletPlayerSelect,
        mode: LibraryAppletMode::AllForeground,
        la_version: version.as_raw(),
        play_startup_sound: false,
    };

    // [1.0.0] reads only the members it had, so the storage stops where they do.
    let payload = match version {
        LaVersion::V1 => ui.settings.as_bytes(),
        LaVersion::V2 | LaVersion::V6 => ui.as_bytes(),
    };

    // Read straight into the reply struct rather than into a byte array and
    // parsing after: every field is valid for any bit pattern, so the struct is
    // the buffer and there is no decode step that could fail.
    let mut reply = UiReturnArg::new_zeroed();
    let exit_reason = library_applet::launch(
        self_controller,
        creator,
        &applet,
        &[payload],
        Some(reply.as_mut_bytes()),
    )
    .map_err(ShowError::Launch)?;

    if exit_reason != LibraryAppletExitReason::Normal {
        return Err(ShowError::AbnormalExit(exit_reason));
    }

    // The applet's own verdict outranks the launch having gone through; libnx
    // returns this code to its caller unchanged, and so does the mapping below.
    if reply.res != 0 {
        return Err(ShowError::Reported(reply.res));
    }

    Ok(reply.user_id)
}

/// Error returned by the `show_*` entry points.
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to run the player-select applet.
    #[error("failed to launch the player-select applet")]
    Launch(#[source] LaunchError),
    /// The applet terminated abnormally.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// The applet ran to completion but reported a failure.
    #[error("the applet reported a failure")]
    Reported(nx_sf::error::ResultCode),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ShowError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Launch(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for it.
            Self::AbnormalExit(_) => nx_sf::error::GENERIC_ERROR,
            // The applet named its own code; libnx hands it back unchanged.
            Self::Reported(rc) => rc,
        }
    }
}
