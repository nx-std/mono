//! The application error dialog.

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
use zerocopy::IntoBytes as _;

use crate::proto::ErrorApplicationArg;

/// The `error` applet, addressed as libnx addresses it.
///
/// libnx passes `la_version` 0 for every `error` launch.
const ERROR_APPLET: LibraryApplet = LibraryApplet {
    id: AppletId::LibraryAppletError,
    mode: LibraryAppletMode::AllForeground,
    la_version: 0,
    play_startup_sound: false,
};

/// Size of the reply storage the applet pops back.
const REPLY_SIZE: usize = 2;

/// An application error dialog, ready to show.
///
/// Building one performs no IPC; [`show`](Self::show) does all of it.
pub struct ApplicationError {
    arg: ErrorApplicationArg,
}

impl ApplicationError {
    /// Builds a dialog showing `dialog_message`, with `fullscreen_message`
    /// behind the "Details" button when given.
    ///
    /// Each message is capped at 2 KB and truncated at a character boundary.
    pub fn new(dialog_message: &str, fullscreen_message: Option<&str>) -> Self {
        Self {
            arg: ErrorApplicationArg::new(dialog_message, fullscreen_message),
        }
    }

    /// Wraps an argument struct built elsewhere.
    ///
    /// Exists for the C boundary, where the caller fills the arg struct through
    /// one entry point and shows it through another, so the two halves cannot
    /// share an [`ApplicationError`].
    pub fn from_arg(arg: ErrorApplicationArg) -> Self {
        Self { arg }
    }

    /// Sets the decimal number the dialog displays as its error code.
    pub fn with_error_number(mut self, error_number: u32) -> Self {
        self.arg.error_number = error_number;
        self
    }

    /// Shows the dialog, blocking until the user dismisses it.
    ///
    /// A dialog the user cancels is still a dialog that ran, so cancellation
    /// reports success; only a failure to present it is an error.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented, exited
    /// abnormally, or reported a non-zero status.
    pub fn show(
        &self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<(), ShowError> {
        let mut reply = [0u8; REPLY_SIZE];

        let exit_reason = library_applet::launch(
            self_controller,
            creator,
            &ERROR_APPLET,
            &[self.arg.as_bytes()],
            Some(&mut reply),
        )
        .map_err(ShowError::Launch)?;

        match exit_reason {
            LibraryAppletExitReason::Normal => check_reply(&reply),
            // The user dismissed the dialog. It ran, which is all this asked for.
            LibraryAppletExitReason::Canceled => Ok(()),
            reason => Err(ShowError::AbnormalExit(reason)),
        }
    }
}

/// Error returned by [`ApplicationError::show`].
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to run the error applet.
    #[error("failed to launch the error applet")]
    Launch(#[source] LaunchError),
    /// The applet terminated abnormally.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// The applet reported a non-zero status.
    #[error("the applet reported status {0}")]
    AppletStatus(u8),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ShowError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Launch(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for either.
            Self::AbnormalExit(_) | Self::AppletStatus(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}

/// Checks the status carried by the applet's two-byte reply.
fn check_reply(reply: &[u8; REPLY_SIZE]) -> Result<(), ShowError> {
    // libnx reads the status from the second byte and treats a non-zero value
    // as a failed run. Official software ignores it entirely.
    let status = reply[1];
    if status != 0 {
        return Err(ShowError::AppletStatus(status));
    }

    Ok(())
}
