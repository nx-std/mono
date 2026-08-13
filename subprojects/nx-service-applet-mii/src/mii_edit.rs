//! Opening the Mii editor on a chosen screen.

use nx_service_applet::{
    AppletId,
    LibraryAppletCreator,
    LibraryAppletExitReason,
    LibraryAppletMode,
    SelfController,
    library_applet::{
        CreateLibraryAppletError,
        CreateStorageError,
        GetAppletStateChangedEventError,
        JoinError,
        PopOutDataError,
        PushInDataError,
        ReadStorageError,
        StartError,
        WriteStorageError,
    },
};
use nx_service_mii::{
    MiiCharInfo,
    MiiSpecialKeyCode,
};
use zerocopy::{
    FromZeros as _,
    IntoBytes as _,
};

use crate::proto::{
    AppletInput,
    AppletInputPayload,
    AppletMode,
    AppletOutput,
    AppletOutputForCharInfoEditing,
    INPUT_VERSION_V3,
    INPUT_VERSION_V4,
    RESULT_CANCEL,
    RESULT_SUCCESS,
    Uuid,
};

/// Which Mii-editor screen to open, and the data that screen accepts.
///
/// These are the screens the applet answers with a database index. The two that
/// edit a Mii without saving it answer with the Mii itself and live in
/// [`MiiCharInfoEdit`].
///
/// libnx exposes one entry point per variant, each filling a single argument
/// struct whose unused members every caller leaves zeroed. Which members a
/// screen reads is fixed, so they are carried by the variant here and a
/// combination libnx would ignore cannot be built.
#[derive(Debug, Clone, Copy)]
pub enum MiiEdit<'a> {
    /// Opens the editor on the console's Mii database.
    ///
    /// libnx `miiLaShowMiiEdit`.
    Show,
    /// Adds a Mii to the database.
    ///
    /// libnx `miiLaAppendMii`.
    AppendMii,
    /// Adds a Mii image, offering the Miis named by `valid_uuids`.
    ///
    /// libnx `miiLaAppendMiiImage`.
    AppendMiiImage {
        /// The Miis the applet offers, of which it reads at most
        /// [`VALID_UUID_ARRAY_LEN`](crate::proto::VALID_UUID_ARRAY_LEN).
        valid_uuids: &'a [Uuid],
    },
    /// Replaces the Mii image named by `used_uuid`.
    ///
    /// libnx `miiLaUpdateMiiImage`.
    UpdateMiiImage {
        /// The Miis the applet offers, of which it reads at most
        /// [`VALID_UUID_ARRAY_LEN`](crate::proto::VALID_UUID_ARRAY_LEN).
        valid_uuids: &'a [Uuid],
        /// The Mii image being replaced.
        used_uuid: Uuid,
    },
}

/// What the applet reported after the user left a [`MiiEdit`] screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiiEditReply {
    /// The user completed the screen.
    Completed {
        /// The database index the screen produced.
        ///
        /// The applet leaves this zero for [`MiiEdit::Show`], which produces no
        /// index; libnx does not read it for that screen either.
        index: i32,
    },
    /// The user left the screen without completing it.
    Cancelled,
}

impl MiiEdit<'_> {
    /// Opens the screen as an argument-storage version 3 request, blocking
    /// until the user leaves it.
    ///
    /// libnx addresses the applet this way below `[10.2.0]` (`_miiLaGetVersion`).
    /// Which firmware this runs on is not a question a service crate answers,
    /// so the caller picks the version.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be run, exited
    /// abnormally, or reported a status this mapping does not name.
    pub fn show_v3(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        key_code: MiiSpecialKeyCode,
    ) -> Result<MiiEditReply, ShowError> {
        self.show(self_controller, creator, key_code, INPUT_VERSION_V3)
    }

    /// Opens the screen as an argument-storage version 4 request, blocking
    /// until the user leaves it.
    ///
    /// libnx addresses the applet this way from `[10.2.0]` (`_miiLaGetVersion`).
    /// Which firmware this runs on is not a question a service crate answers,
    /// so the caller picks the version.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be run, exited
    /// abnormally, or reported a status this mapping does not name.
    pub fn show_v4(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        key_code: MiiSpecialKeyCode,
    ) -> Result<MiiEditReply, ShowError> {
        self.show(self_controller, creator, key_code, INPUT_VERSION_V4)
    }

    /// Runs the screen with the given argument-storage version.
    ///
    /// Shared by [`show_v3`](Self::show_v3) and [`show_v4`](Self::show_v4),
    /// which differ only in that version.
    fn show(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        key_code: MiiSpecialKeyCode,
        version: i32,
    ) -> Result<MiiEditReply, ShowError> {
        let input = self.build_input(key_code, version);

        // Read straight into the reply struct rather than into a byte array and
        // parsing after: every field is valid for any bit pattern, so the struct
        // is the buffer and there is no decode step that could fail.
        let mut output = AppletOutput::new_zeroed();
        let exit_reason =
            run(self_controller, creator, &input, output.as_mut_bytes()).map_err(ShowError::Run)?;

        if exit_reason != LibraryAppletExitReason::Normal {
            return Err(ShowError::AbnormalExit(exit_reason));
        }

        match output.res {
            RESULT_SUCCESS => Ok(MiiEditReply::Completed {
                index: output.index,
            }),
            RESULT_CANCEL => Ok(MiiEditReply::Cancelled),
            res => Err(ShowError::UnexpectedResult(res)),
        }
    }

    /// Builds the argument storage, filling the members this screen reads.
    fn build_input(self, key_code: MiiSpecialKeyCode, version: i32) -> AppletInput {
        let payload = match self {
            Self::Show | Self::AppendMii => AppletInputPayload::empty(),
            Self::AppendMiiImage { valid_uuids } | Self::UpdateMiiImage { valid_uuids, .. } => {
                AppletInputPayload::valid_uuids(valid_uuids)
            }
        };

        // Exact: `MiiSpecialKeyCode` is `#[repr(u32)]`, so the cast is its
        // discriminant, which is the word the applet reads.
        let mut input = AppletInput::new(version, self.mode(), key_code as u32, payload);

        if let Self::UpdateMiiImage { used_uuid, .. } = self {
            input.used_uuid = used_uuid;
        }

        input
    }

    /// Returns the mode this screen is opened with.
    const fn mode(self) -> AppletMode {
        match self {
            Self::Show => AppletMode::ShowMiiEdit,
            Self::AppendMii => AppletMode::AppendMii,
            Self::AppendMiiImage { .. } => AppletMode::AppendMiiImage,
            Self::UpdateMiiImage { .. } => AppletMode::UpdateMiiImage,
        }
    }
}

/// Which Mii the applet is asked to edit without saving it.
///
/// libnx exposes one entry point per variant (`miiLaCreateMii`,
/// `miiLaEditMii`). Both screens answer with a Mii rather than a database
/// index, which is what separates them from [`MiiEdit`].
#[derive(Debug, Clone, Copy)]
pub enum MiiCharInfoEdit<'a> {
    /// Makes a Mii from scratch.
    ///
    /// libnx `miiLaCreateMii`.
    Create,
    /// Edits the given Mii.
    ///
    /// libnx `miiLaEditMii`. Official software validates the Mii before
    /// handing it over; libnx does not, and neither does this.
    Edit {
        /// The Mii the editor opens on.
        char_info: &'a MiiCharInfo,
    },
}

/// What the applet reported after the user left a [`MiiCharInfoEdit`] screen.
#[derive(Debug, Clone, Copy)]
pub enum MiiCharInfoEditReply {
    /// The user completed the screen.
    Completed {
        /// The Mii the user made, which the applet did not save.
        char_info: MiiCharInfo,
    },
    /// The user left the screen without completing it.
    Cancelled,
}

impl MiiCharInfoEdit<'_> {
    /// Opens the screen as an argument-storage version 4 request, blocking
    /// until the user leaves it.
    ///
    /// There is no version 3 counterpart: both screens arrived in `[10.2.0]`,
    /// from which libnx addresses the applet as version 4 (`_miiLaGetVersion`).
    /// Whether the console is that new is not a question a service crate
    /// answers, so the caller gates the call.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be run, exited
    /// abnormally, or reported a status this mapping does not name.
    pub fn show_v4(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        key_code: MiiSpecialKeyCode,
    ) -> Result<MiiCharInfoEditReply, ShowError> {
        let payload = match self {
            Self::Create => AppletInputPayload::empty(),
            Self::Edit { char_info } => AppletInputPayload::char_info(char_info),
        };
        // Exact, as in `MiiEdit::build_input`: `MiiSpecialKeyCode` is
        // `#[repr(u32)]`, so the cast is its discriminant.
        let input = AppletInput::new(INPUT_VERSION_V4, self.mode(), key_code as u32, payload);

        // As in `MiiEdit::show`: the reply struct is valid for any bit pattern,
        // so it is the buffer rather than something decoded out of one.
        let mut output = AppletOutputForCharInfoEditing::new_zeroed();
        let exit_reason =
            run(self_controller, creator, &input, output.as_mut_bytes()).map_err(ShowError::Run)?;

        if exit_reason != LibraryAppletExitReason::Normal {
            return Err(ShowError::AbnormalExit(exit_reason));
        }

        match output.res {
            RESULT_SUCCESS => Ok(MiiCharInfoEditReply::Completed {
                char_info: output.char_info,
            }),
            RESULT_CANCEL => Ok(MiiCharInfoEditReply::Cancelled),
            res => Err(ShowError::UnexpectedResult(res)),
        }
    }

    /// Returns the mode this screen is opened with.
    const fn mode(self) -> AppletMode {
        match self {
            Self::Create => AppletMode::CreateMii,
            Self::Edit { .. } => AppletMode::EditMii,
        }
    }
}

/// Error returned by [`MiiEdit::show_v3`], [`MiiEdit::show_v4`] and
/// [`MiiCharInfoEdit::show_v4`].
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to run the Mii editor applet.
    #[error("failed to run the Mii editor applet")]
    Run(#[source] RunError),
    /// The applet terminated abnormally.
    ///
    /// libnx reports the same case as `LibnxError_LibAppletBadExit`, for every
    /// exit reason other than a normal one.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// The applet reported a status neither success nor cancellation.
    ///
    /// libnx reports the same case as `LibnxError_ShouldNotHappen`.
    #[error("the applet reported an unexpected status {0}")]
    UnexpectedResult(u32),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ShowError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Run(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for either of these.
            Self::AbnormalExit(_) | Self::UnexpectedResult(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Runs the Mii editor with `input`, reading its reply into `reply`.
///
/// This is libnx's `_miiLaShow`, and the reason this crate does not call
/// [`library_applet::launch`]: that function pushes the common arguments as the
/// first storage, and the Mii editor is the one library applet libnx launches
/// without them: `mii_la.c` pushes nothing but `input`. An applet reads its own
/// arguments from the first storage pushed, so adding the common ones would
/// make it decode the wrong bytes.
///
/// The exit reason is returned rather than judged, matching
/// [`library_applet::launch`]; the caller decides what an abnormal exit means.
///
/// [`library_applet::launch`]: nx_service_applet::library_applet::launch
fn run(
    self_controller: &SelfController<'_>,
    creator: &LibraryAppletCreator<'_>,
    input: &AppletInput,
    reply: &mut [u8],
) -> Result<LibraryAppletExitReason, RunError> {
    let launchable = self_controller
        .get_library_applet_launchable_event()
        .map_err(RunError::LaunchableEvent)?;

    nx_svc::sync::wait_synchronization(&launchable, None).map_err(RunError::WaitLaunchable)?;

    let accessor = creator
        .create_library_applet(
            AppletId::LibraryAppletMiiEdit,
            LibraryAppletMode::AllForeground,
        )
        .map_err(RunError::CreateApplet)?;

    // Obtained before Start: the applet may exit before we would otherwise get
    // around to asking, and the event is what tells us that happened.
    let state_changed = accessor
        .get_applet_state_changed_event()
        .map_err(RunError::StateChangedEvent)?;

    // The argument struct is the only storage the applet gets, and the server
    // copies its contents, so the storage is done with once it is pushed.
    let bytes = input.as_bytes();
    // Widening cast: the argument storage is a fixed 0x100 bytes.
    let storage = creator
        .create_storage(bytes.len() as i64)
        .map_err(RunError::CreateStorage)?;
    storage.write(0, bytes).map_err(RunError::WriteStorage)?;
    accessor
        .push_in_data(&storage)
        .map_err(RunError::PushInData)?;

    accessor.start().map_err(RunError::Start)?;

    let exit_reason = accessor.join(&state_changed).map_err(RunError::Join)?;

    // Only an applet that ran to completion pushed a reply. Popping after a
    // cancellation asks for a storage the applet never queued.
    if exit_reason != LibraryAppletExitReason::Normal {
        return Ok(exit_reason);
    }

    let reply_storage = accessor.pop_out_data().map_err(RunError::PopOutData)?;
    reply_storage.read(0, reply).map_err(RunError::ReadReply)?;

    Ok(exit_reason)
}

/// Error returned by [`run`].
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// Failed to get the library applet launchable event.
    #[error("failed to get the launchable event")]
    LaunchableEvent(#[source] nx_service_applet::GetLibraryAppletLaunchableEventError),
    /// Failed to wait for the system to allow a library applet launch.
    #[error("failed to wait for the launchable event")]
    WaitLaunchable(#[source] nx_svc::sync::WaitSyncError),
    /// Failed to create the applet.
    #[error("failed to create the applet")]
    CreateApplet(#[source] CreateLibraryAppletError),
    /// Failed to get the applet state-changed event.
    #[error("failed to get the state-changed event")]
    StateChangedEvent(#[source] GetAppletStateChangedEventError),
    /// Failed to create the argument storage.
    #[error("failed to create the argument storage")]
    CreateStorage(#[source] CreateStorageError),
    /// Failed to write the argument struct into the storage.
    #[error("failed to write the argument storage")]
    WriteStorage(#[source] WriteStorageError),
    /// Failed to push the argument storage to the applet.
    #[error("failed to push the argument storage")]
    PushInData(#[source] PushInDataError),
    /// Failed to start the applet.
    #[error("failed to start the applet")]
    Start(#[source] StartError),
    /// Failed to wait for the applet to exit.
    #[error("failed to wait for the applet to exit")]
    Join(#[source] JoinError),
    /// Failed to pop the applet's reply storage.
    #[error("failed to pop the reply storage")]
    PopOutData(#[source] PopOutDataError),
    /// Failed to read the applet's reply storage.
    #[error("failed to read the reply storage")]
    ReadReply(#[source] ReadStorageError),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for RunError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::LaunchableEvent(err) => err.to_rc(),
            Self::WaitLaunchable(err) => nx_svc::error::ToResultCode::to_rc(err),
            Self::CreateApplet(err) => err.to_rc(),
            Self::StateChangedEvent(err) => err.to_rc(),
            Self::CreateStorage(err) => err.to_rc(),
            Self::WriteStorage(err) => err.to_rc(),
            Self::PushInData(err) => err.to_rc(),
            Self::Start(err) => err.to_rc(),
            Self::Join(err) => err.to_rc(),
            Self::PopOutData(err) => err.to_rc(),
            Self::ReadReply(err) => err.to_rc(),
        }
    }
}
