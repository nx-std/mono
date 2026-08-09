//! Opening the controller applet on one of its screens.

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
use nx_service_hid::NpadJoyHoldType;
use zerocopy::{
    FromZeros as _,
    IntoBytes as _,
};

use crate::proto::{
    ControllerFirmwareUpdateArg,
    ControllerKeyRemappingArg,
    ControllerSupportArg,
    ControllerSupportArgPrivate,
    ControllerSupportArgV3,
    ControllerSupportCaller,
    ControllerSupportMode,
    ControllerSupportResultInfo,
    ControllerSupportResultInfoInternal,
};

/// Which revision of the controller applet's protocol to speak.
///
/// libnx derives two things from the running system version: the library-applet
/// API version carried in the common arguments, and which of the two
/// controller-support argument layouts the applet reads. Both move on the same
/// ladder, and [8.0.0], the only point at which the layout changes, is also one
/// of the API-version steps, so a single value settles both and the two can
/// never be paired wrongly.
///
/// Each variant is named for the API version it sends. Choosing one is the
/// caller's job: this crate never reads the system version, because a service
/// crate may not depend on the runtime that holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControllerSupportVersion {
    /// Pre-[3.0.0]: API version 3, four-player argument layout.
    V3,
    /// [3.0.0] to [6.0.0): API version 4, four-player argument layout.
    V4,
    /// [6.0.0] to [8.0.0): API version 5, four-player argument layout.
    V5,
    /// [8.0.0] to [11.0.0): API version 7, eight-player argument layout.
    V7,
    /// [11.0.0+]: API version 8, eight-player argument layout.
    V8,
}

impl ControllerSupportVersion {
    /// Returns the library-applet API version carried in the common arguments.
    pub const fn la_version(self) -> u32 {
        match self {
            Self::V3 => 0x3,
            Self::V4 => 0x4,
            Self::V5 => 0x5,
            Self::V7 => 0x7,
            Self::V8 => 0x8,
        }
    }

    /// Returns whether the controller-support screens take the four-player
    /// argument layout.
    pub const fn sends_v3_arg(self) -> bool {
        matches!(self, Self::V3 | Self::V4 | Self::V5)
    }

    /// Returns the size the request storage declares for the screen's own
    /// storage.
    pub const fn support_arg_size(self) -> u32 {
        // Narrowing casts: the `const_assert_eq!` on each layout pins them at
        // 0x21C and 0x430 bytes, so both fit in a `u32`.
        if self.sends_v3_arg() {
            core::mem::size_of::<ControllerSupportArgV3>() as u32
        } else {
            core::mem::size_of::<ControllerSupportArg>() as u32
        }
    }
}

/// What the applet is told about the console's current controller setup.
///
/// libnx reads both from the HID service immediately before every launch. They
/// are asked of the caller here: reaching the HID service is that crate's job,
/// not this one's, and the system entry point on pre-[3.0.0] supplies fixed
/// values instead of asking at all.
#[derive(Debug, Clone, Copy)]
pub struct ControllerSupportContext {
    /// The Npad style set the system supports, as libnx's
    /// `hidGetSupportedNpadStyleSet` reports it.
    pub npad_style_set: u32,
    /// How the system expects a pair of Joy-Cons to be held.
    pub npad_joy_hold_type: NpadJoyHoldType,
}

/// Which screen to open, and the data that screen accepts.
///
/// libnx exposes one entry point per variant, all funnelling into a single
/// private launcher that then re-checks that the mode and the argument struct
/// agree. Here the variant carries both, so the pairs libnx rejects cannot be
/// built: no screen can be asked for with another screen's arguments.
///
/// The `ForSystem` variants are the system's own entry points. They differ from
/// their plain counterparts only in the flags the request storage carries, which
/// is why they are variants rather than a separate type.
#[derive(Debug, Clone, Copy)]
pub enum ControllerSupport<'a> {
    /// Pair and arrange the controllers the application wants.
    ///
    /// The applet only presents itself when doing so is actually needed.
    Support {
        /// How many players to accept, and how their boxes look.
        arg: &'a ControllerSupportArg,
    },
    /// Pair and arrange controllers as the system does.
    ///
    /// Unlike [`Self::Support`], this always presents the applet.
    SupportForSystem {
        /// How many players to accept, and how their boxes look.
        arg: &'a ControllerSupportArg,
        /// Whether to present the menu as qlaunch presents it, startup sound
        /// included.
        as_qlaunch: bool,
    },
    /// Show the wrist-strap guide. Requires [3.0.0].
    ///
    /// It takes a controller-support argument that it displays nothing of, so
    /// libnx passes a default-built one and so does this.
    StrapGuide,
    /// Update the controllers' firmware. Requires [3.0.0].
    FirmwareUpdate {
        /// Whether the update may be skipped.
        arg: &'a ControllerFirmwareUpdateArg,
    },
    /// Update the controllers' firmware as the system does. Requires [3.0.0].
    FirmwareUpdateForSystem {
        /// Whether the update may be skipped.
        arg: &'a ControllerFirmwareUpdateArg,
        /// On whose behalf the applet runs, which decides whether the
        /// confirmation dialog appears.
        caller: ControllerSupportCaller,
    },
    /// Remap the controllers' keys. Requires [11.0.0].
    KeyRemappingForSystem {
        /// The applet's opaque opening parameters.
        arg: &'a ControllerKeyRemappingArg,
        /// On whose behalf the applet runs.
        caller: ControllerSupportCaller,
    },
}

impl ControllerSupport<'_> {
    /// Opens the applet on this screen, blocking until the user leaves it.
    ///
    /// `version` picks the protocol revision to speak and `context` describes
    /// the console's controller setup; see [`ControllerSupportVersion`] and
    /// [`ControllerSupportContext`] for who chooses them.
    ///
    /// The result is returned for every screen, though only the
    /// controller-support ones fill it: the applet always replies with the same
    /// storage, and whether the reply is interesting is the caller's question.
    /// libnx reads the same storage on every screen but discards the error from
    /// reading it, so an applet that queued no reply reads there as a success
    /// with an all-zero result; here that is reported rather than assumed.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported that it did not complete.
    pub fn show(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        version: ControllerSupportVersion,
        context: &ControllerSupportContext,
    ) -> Result<ControllerSupportResultInfo, ShowError> {
        let private = self.private_arg(version, context);
        let screen_arg = self.screen_arg(version);

        let applet = LibraryApplet {
            id: AppletId::LibraryAppletController,
            mode: LibraryAppletMode::AllForeground,
            la_version: version.la_version(),
            // libnx plays the startup sound only when both flags are set, which
            // no entry point but the system's controller-support one does.
            play_startup_sound: private.flag0 != 0 && private.flag1 != 0,
        };

        // Read straight into the reply struct rather than into a byte array and
        // parsing after: every field is valid for any bit pattern, so the struct
        // is the buffer and there is no decode step that could fail.
        let mut reply = ControllerSupportResultInfoInternal::new_zeroed();
        let exit_reason = library_applet::launch(
            self_controller,
            creator,
            &applet,
            &[private.as_bytes(), screen_arg.as_bytes()],
            Some(reply.as_mut_bytes()),
        )
        .map_err(ShowError::Launch)?;

        if exit_reason != LibraryAppletExitReason::Normal {
            return Err(ShowError::AbnormalExit(exit_reason));
        }

        // libnx treats any non-zero `res` as the applet having failed. Official
        // software tells 2 apart from the rest; libnx does not, and neither do
        // we, so the value is carried out for whoever comes to care.
        if reply.res != 0 {
            return Err(ShowError::AppletFailed(reply.res));
        }

        Ok(reply.info)
    }

    /// Builds the request storage, setting the flags this entry point carries.
    fn private_arg(
        self,
        version: ControllerSupportVersion,
        context: &ControllerSupportContext,
    ) -> ControllerSupportArgPrivate {
        let mut private = ControllerSupportArgPrivate::new(self.mode(), version.support_arg_size());

        private.npad_style_set = context.npad_style_set;
        private.npad_joy_hold_type = context.npad_joy_hold_type.as_raw();

        match self {
            Self::Support { .. } | Self::StrapGuide | Self::FirmwareUpdate { .. } => {}
            Self::SupportForSystem { as_qlaunch, .. } => {
                private.flag0 = as_qlaunch.into();
                private.flag1 = 1;
            }
            Self::FirmwareUpdateForSystem { caller, .. }
            | Self::KeyRemappingForSystem { caller, .. } => {
                private.flag1 = 1;
                private.controller_support_caller = caller.as_raw();
            }
        }

        private
    }

    /// Builds the screen's own argument storage, in the layout `version` reads.
    fn screen_arg(self, version: ControllerSupportVersion) -> ScreenArg {
        match self {
            Self::Support { arg } | Self::SupportForSystem { arg, .. } => {
                Self::support_arg(arg, version)
            }
            Self::StrapGuide => Self::support_arg(&ControllerSupportArg::new(), version),
            Self::FirmwareUpdate { arg } | Self::FirmwareUpdateForSystem { arg, .. } => {
                ScreenArg::FirmwareUpdate(*arg)
            }
            Self::KeyRemappingForSystem { arg, .. } => ScreenArg::KeyRemapping(*arg),
        }
    }

    /// Narrows `arg` to the layout `version` reads.
    fn support_arg(arg: &ControllerSupportArg, version: ControllerSupportVersion) -> ScreenArg {
        if version.sends_v3_arg() {
            ScreenArg::SupportV3(arg.into())
        } else {
            ScreenArg::Support(*arg)
        }
    }

    /// Returns the screen this request opens.
    fn mode(self) -> ControllerSupportMode {
        match self {
            Self::Support { .. } | Self::SupportForSystem { .. } => {
                ControllerSupportMode::ShowControllerSupport
            }
            Self::StrapGuide => ControllerSupportMode::ShowControllerStrapGuide,
            Self::FirmwareUpdate { .. } | Self::FirmwareUpdateForSystem { .. } => {
                ControllerSupportMode::ShowControllerFirmwareUpdate
            }
            Self::KeyRemappingForSystem { .. } => {
                ControllerSupportMode::ShowControllerKeyRemappingForSystem
            }
        }
    }
}

/// Error returned by [`ControllerSupport::show`].
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to run the controller applet.
    ///
    /// Occurs when a step of the launch sequence failed: the system would not
    /// host a library applet, the applet could not be created, a storage could
    /// not be pushed, or the reply could not be read.
    #[error("failed to launch the controller applet")]
    Launch(#[source] LaunchError),
    /// The applet terminated abnormally.
    ///
    /// Occurs when the applet exited without running to completion: the user
    /// dismissed it, or the system tore it down. Nothing it was asked to do
    /// took effect.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// The applet reported that it did not complete.
    ///
    /// Occurs when the applet ran to the end and then reported a non-zero
    /// result of its own, which libnx and official software both treat as the
    /// request having been refused. The reported value is carried through
    /// unexamined: no meaning has been established for any of them.
    #[error("the applet reported failure: {0}")]
    AppletFailed(u32),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ShowError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Launch(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for either.
            Self::AbnormalExit(_) | Self::AppletFailed(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// The screen's own argument storage, owned so its bytes outlive the match that
/// chose them.
///
/// The controller-support screens have two layouts and the other screens one
/// each; which is sent is settled here rather than by the caller.
#[expect(
    clippy::large_enum_variant,
    reason = "the spread is the applet's own: its argument layouts run from 4 bytes to 1072. \
              Boxing the large ones is not open to a crate with no allocator, and libnx puts \
              the same struct on the stack at the same point."
)]
enum ScreenArg {
    /// The eight-player controller-support layout.
    Support(ControllerSupportArg),
    /// The four-player controller-support layout.
    SupportV3(ControllerSupportArgV3),
    /// The firmware-update layout.
    FirmwareUpdate(ControllerFirmwareUpdateArg),
    /// The key-remapping layout.
    KeyRemapping(ControllerKeyRemappingArg),
}

impl ScreenArg {
    /// Returns the bytes pushed as the screen's storage.
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Support(arg) => arg.as_bytes(),
            Self::SupportV3(arg) => arg.as_bytes(),
            Self::FirmwareUpdate(arg) => arg.as_bytes(),
            Self::KeyRemapping(arg) => arg.as_bytes(),
        }
    }
}
