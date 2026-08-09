//! Opening the cabinet applet on an amiibo settings screen.

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
use nx_service_nfc::{
    NfcDeviceHandle,
    NfcTagInfo,
    NfpRegisterInfo,
};
use zerocopy::{
    FromZeros as _,
    IntoBytes as _,
};

use crate::proto::{
    AmiiboSettingsStartParam,
    AmiiboSettingsType,
    RETURN_FLAG_REGISTER_INFO,
    ReturnValueForAmiiboSettings,
    START_FLAG_REGISTER_INFO,
    START_FLAG_TAG_INFO,
    StartParamForAmiiboSettings,
};

/// The `cabinet` applet, addressed as libnx addresses it.
///
/// libnx passes `la_version` 1 for every cabinet launch.
const CABINET_APPLET: LibraryApplet = LibraryApplet {
    id: AppletId::LibraryAppletCabinet,
    mode: LibraryAppletMode::AllForeground,
    la_version: 1,
    play_startup_sound: false,
};

/// Which settings screen to open, and the data that screen accepts.
///
/// libnx exposes one entry point per variant, each funnelling into a single
/// function whose optional arguments it passes as null pointers. Which
/// arguments a screen accepts is fixed, so they are carried by the variant here
/// and a combination libnx would reject cannot be built.
#[derive(Debug, Clone, Copy)]
pub enum AmiiboSettings<'a> {
    /// Edit the amiibo's nickname and owner.
    ///
    /// `tag_info` constrains which amiibo the applet accepts; `register_info`
    /// seeds the data it writes, which the user may still change.
    NicknameAndOwner {
        /// The amiibo the scan must match.
        tag_info: Option<&'a NfcTagInfo>,
        /// Registration data to start from.
        register_info: Option<&'a NfpRegisterInfo>,
    },
    /// Erase the game data written to the amiibo.
    GameDataEraser {
        /// The amiibo the scan must match.
        tag_info: Option<&'a NfcTagInfo>,
    },
    /// Restore the amiibo from a backup.
    Restorer {
        /// The amiibo the scan must match.
        tag_info: Option<&'a NfcTagInfo>,
    },
    /// Format the amiibo.
    Formatter,
}

/// What the cabinet applet reported after the user left it.
#[derive(Debug, Clone, Copy)]
pub struct AmiiboSettingsReply {
    /// The device the applet used.
    pub handle: NfcDeviceHandle,
    /// The amiibo the applet acted on.
    pub tag_info: NfcTagInfo,
    /// Registration data, present only when the applet reported it as set.
    pub register_info: Option<NfpRegisterInfo>,
}

impl AmiiboSettings<'_> {
    /// Opens the applet on this screen, blocking until the user leaves it.
    ///
    /// `start_param` carries the opaque opening parameters libnx copies into the
    /// argument storage.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`StartError`] when the applet could not be presented, exited
    /// abnormally, or reported that it did not complete.
    pub fn start(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        start_param: &AmiiboSettingsStartParam,
    ) -> Result<AmiiboSettingsReply, StartError> {
        let param = self.build_param(start_param);

        // Read straight into the reply struct rather than into a byte array and
        // parsing after: every field is valid for any bit pattern, so the struct
        // is the buffer and there is no decode step that could fail.
        let mut reply = ReturnValueForAmiiboSettings::new_zeroed();
        let exit_reason = library_applet::launch(
            self_controller,
            creator,
            &CABINET_APPLET,
            &[param.as_bytes()],
            Some(reply.as_mut_bytes()),
        )
        .map_err(StartError::Launch)?;

        if exit_reason != LibraryAppletExitReason::Normal {
            return Err(StartError::AbnormalExit(exit_reason));
        }

        // libnx treats a zero `flags` as the applet having failed. Official
        // software checks the same byte.
        if reply.flags == 0 {
            return Err(StartError::AppletFailed);
        }

        Ok(AmiiboSettingsReply {
            handle: reply.handle,
            tag_info: reply.tag_info,
            register_info: (reply.flags & RETURN_FLAG_REGISTER_INFO != 0)
                .then_some(reply.register_info),
        })
    }

    /// Builds the argument storage, setting the flag for each member this
    /// screen carries.
    fn build_param(self, start_param: &AmiiboSettingsStartParam) -> StartParamForAmiiboSettings {
        let mut param = StartParamForAmiiboSettings::new(self.ty(), start_param);

        let (tag_info, register_info) = match self {
            Self::NicknameAndOwner {
                tag_info,
                register_info,
            } => (tag_info, register_info),
            Self::GameDataEraser { tag_info } | Self::Restorer { tag_info } => (tag_info, None),
            Self::Formatter => (None, None),
        };

        if let Some(tag_info) = tag_info {
            param.tag_info = *tag_info;
            param.flags |= START_FLAG_TAG_INFO;
        }
        if let Some(register_info) = register_info {
            param.register_info = *register_info;
            param.flags |= START_FLAG_REGISTER_INFO;
        }

        param
    }

    /// Returns the screen type this request opens.
    const fn ty(self) -> AmiiboSettingsType {
        match self {
            Self::NicknameAndOwner { .. } => AmiiboSettingsType::NicknameAndOwnerSettings,
            Self::GameDataEraser { .. } => AmiiboSettingsType::GameDataEraser,
            Self::Restorer { .. } => AmiiboSettingsType::Restorer,
            Self::Formatter => AmiiboSettingsType::Formatter,
        }
    }
}

/// Error returned by [`AmiiboSettings::start`].
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// Failed to run the cabinet applet.
    #[error("failed to launch the cabinet applet")]
    Launch(#[source] LaunchError),
    /// The applet terminated abnormally.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// The applet reported that it did not complete.
    #[error("the applet reported failure")]
    AppletFailed,
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for StartError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Launch(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for any of these.
            Self::AbnormalExit(_) | Self::AppletFailed => nx_sf::error::GENERIC_ERROR,
        }
    }
}
