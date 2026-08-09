//! Opening the auth applet on a Parental Controls screen.

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
use nx_sf::error::ResultCode;
use zerocopy::{
    FromZeros as _,
    IntoBytes as _,
};

use crate::proto::{
    PctlAuthArg,
    PctlAuthReply,
    PctlAuthType,
};

/// Library applet API version HOS below 4.0.0 is addressed with.
const LA_VERSION_V1: u32 = 1;

/// Library applet API version HOS 4.0.0 and later is addressed with.
const LA_VERSION_V2: u32 = 2;

/// Which Parental Controls screen to open, and the arguments that screen
/// accepts.
///
/// libnx exposes one entry point per variant, each funnelling into a single
/// function that zeroes the argument struct first. Only the authentication
/// screen reads any of the argument bytes, so they are carried by that variant
/// here and the passcode screens cannot be handed bytes libnx would have left
/// at zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentalAuth {
    /// Ask the user for the Parental Controls PIN.
    ///
    /// The three bytes are libnx's `arg0`, `arg1` and `arg2`; see
    /// [`PctlAuthArg`] for what is known of them. Use
    /// [`authenticate`](Self::authenticate) for the one-byte form libnx
    /// documents on every system version, and
    /// [`authenticate_for_configuration`](Self::authenticate_for_configuration)
    /// for the screen the system settings open.
    Authenticate {
        /// Zero temporarily disables Parental Controls; one validates the PIN
        /// the user enters.
        arg0: u8,
        /// Read from `[4.0.0+]`, meaning unestablished.
        arg1: u8,
        /// Read from `[4.0.0+]`, meaning unestablished.
        arg2: u8,
    },
    /// Register the Parental Controls PIN.
    RegisterPasscode,
    /// Change the Parental Controls PIN.
    ChangePasscode,
}

impl ParentalAuth {
    /// Builds a request for the PIN, leaving the two bytes only `[4.0.0+]`
    /// reads at zero.
    ///
    /// This is the request libnx's `pctlauthShow` builds. `validate_pin` false
    /// temporarily disables Parental Controls; true validates the PIN the user
    /// enters.
    pub const fn authenticate(validate_pin: bool) -> Self {
        Self::Authenticate {
            // A `bool` cast yields 0 or 1 by definition, which is the byte
            // libnx writes for the same flag (`arg.arg0 = flag != 0`).
            arg0: validate_pin as u8,
            arg1: 0,
            arg2: 0,
        }
    }

    /// Builds a request for the PIN the way the system settings ask for it.
    ///
    /// This is the request libnx's `pctlauthShowForConfiguration` builds, which
    /// is exactly `pctlauthShowEx(1, 0, 1)`. libnx documents it as `[4.0.0+]`,
    /// so it belongs with [`show_v2`](Self::show_v2).
    pub const fn authenticate_for_configuration() -> Self {
        Self::Authenticate {
            arg0: 1,
            arg1: 0,
            arg2: 1,
        }
    }

    /// Opens the applet addressed with library-applet API version 1, blocking
    /// until the user leaves it.
    ///
    /// This is how libnx addresses the applet below `[4.0.0]`; the applet reads
    /// only the first argument byte there.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show_v1(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<(), ShowError> {
        self.show(self_controller, creator, LA_VERSION_V1)
    }

    /// Opens the applet addressed with library-applet API version 2, blocking
    /// until the user leaves it.
    ///
    /// This is how libnx addresses the applet from `[4.0.0+]`, the versions that
    /// read all three argument bytes.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a failure of its own.
    pub fn show_v2(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<(), ShowError> {
        self.show(self_controller, creator, LA_VERSION_V2)
    }

    /// Opens the applet addressed with `la_version`.
    ///
    /// Shared by [`show_v1`](Self::show_v1) and [`show_v2`](Self::show_v2),
    /// which differ only in that number.
    fn show(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
        la_version: u32,
    ) -> Result<(), ShowError> {
        let applet = LibraryApplet {
            id: AppletId::LibraryAppletAuth,
            mode: LibraryAppletMode::AllForeground,
            la_version,
            play_startup_sound: false,
        };

        // Read straight into the reply struct rather than into a byte array and
        // parsing after: its one field is valid for any bit pattern, so the
        // struct is the buffer and there is no decode step that could fail.
        // libnx instead reads into a buffer and rejects a reply that is not
        // exactly this size; asking for the size up front makes a shorter
        // storage fail the read.
        let mut reply = PctlAuthReply::new_zeroed();
        let exit_reason = library_applet::launch(
            self_controller,
            creator,
            &applet,
            self.build_arg().as_bytes(),
            Some(reply.as_mut_bytes()),
        )
        .map_err(ShowError::Launch)?;

        if exit_reason != LibraryAppletExitReason::Normal {
            return Err(ShowError::AbnormalExit(exit_reason));
        }

        // libnx returns this code as its own, so a non-zero value is the whole
        // failure: the applet already said why.
        let result = reply.result.get();
        if result != 0 {
            return Err(ShowError::AppletFailed(result));
        }

        Ok(())
    }

    /// Builds the argument storage, filling the bytes only this screen accepts.
    fn build_arg(self) -> PctlAuthArg {
        let mut arg = PctlAuthArg::new(self.ty());

        if let Self::Authenticate { arg0, arg1, arg2 } = self {
            arg.arg0 = arg0;
            arg.arg1 = arg1;
            arg.arg2 = arg2;
        }

        arg
    }

    /// Returns the screen type this request opens.
    const fn ty(self) -> PctlAuthType {
        match self {
            Self::Authenticate { .. } => PctlAuthType::Show,
            Self::RegisterPasscode => PctlAuthType::RegisterPasscode,
            Self::ChangePasscode => PctlAuthType::ChangePasscode,
        }
    }
}

/// Error returned by [`ParentalAuth::show_v1`] and [`ParentalAuth::show_v2`].
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to run the auth applet.
    #[error("failed to launch the parental controls auth applet")]
    Launch(#[source] LaunchError),
    /// The applet terminated abnormally.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// The applet reported a failure of its own.
    #[error("the applet reported result code {0:#x}")]
    AppletFailed(ResultCode),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ShowError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Launch(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for it.
            Self::AbnormalExit(_) => nx_sf::error::GENERIC_ERROR,
            // The applet named its own code, which is the one libnx returns.
            Self::AppletFailed(rc) => rc,
        }
    }
}
