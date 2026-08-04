//! The application error dialog.

use nx_service_applet::{
    AppletId,
    GetLibraryAppletLaunchableEventError,
    LibraryAppletAccessor,
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
use nx_sf::error::{
    GENERIC_ERROR,
    ToResultCode,
};
use nx_svc::error::{
    ResultCode,
    ToResultCode as _,
};
use zerocopy::IntoBytes as _;

use crate::proto::{
    CommonArguments,
    ErrorApplicationArg,
};

/// Library applet API version the error applet is addressed with.
///
/// libnx passes 0 for every `error` launch.
const ERROR_LA_VERSION: u32 = 0;

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
    pub fn show(
        &self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<(), ShowError> {
        let launchable = self_controller
            .get_library_applet_launchable_event()
            .map_err(ShowError::LaunchableEvent)?;

        // SAFETY: The handle names the event the server just issued, and the
        // wait borrows it for the call.
        unsafe { nx_svc::sync::wait_synchronization_single(&launchable, u64::MAX) }
            .map_err(ShowError::WaitLaunchable)?;

        let accessor = creator
            .create_library_applet(
                AppletId::LibraryAppletError,
                LibraryAppletMode::AllForeground,
            )
            .map_err(ShowError::CreateApplet)?;

        // Obtained before Start: the applet may exit before we would otherwise
        // get around to asking, and the event is what tells us that happened.
        let state_changed = accessor
            .get_applet_state_changed_event()
            .map_err(ShowError::StateChangedEvent)?;

        let tick = nx_cpu::counter::ticks().to_raw();
        let common_arguments = CommonArguments::new(ERROR_LA_VERSION, tick);

        // The common arguments must be storage 0: the applet reads its own
        // argument struct from the second storage pushed, so reversing these
        // two makes it decode the header as arguments.
        push_storage(creator, &accessor, common_arguments.as_bytes())
            .map_err(ShowError::PushCommonArguments)?;
        push_storage(creator, &accessor, self.arg.as_bytes()).map_err(ShowError::PushArgument)?;

        accessor.start().map_err(ShowError::Start)?;

        let exit_reason = accessor.join(&state_changed).map_err(ShowError::Join)?;

        match exit_reason {
            LibraryAppletExitReason::Normal => self.check_reply(&accessor),
            // The user dismissed the dialog. It ran, which is all this asked for.
            LibraryAppletExitReason::Canceled => Ok(()),
            reason => Err(ShowError::AbnormalExit(reason)),
        }
    }

    /// Reads the applet's two-byte reply and checks the status it carries.
    fn check_reply(&self, accessor: &LibraryAppletAccessor<'_>) -> Result<(), ShowError> {
        let reply = accessor.pop_out_data().map_err(ShowError::PopOutData)?;

        let mut buffer = [0u8; REPLY_SIZE];
        reply.read(0, &mut buffer).map_err(ShowError::ReadReply)?;

        // libnx reads the status from the second byte and treats a non-zero
        // value as a failed run. Official software ignores it entirely.
        let status = buffer[1];
        if status != 0 {
            return Err(ShowError::AppletStatus(status));
        }

        Ok(())
    }
}

/// Error returned by [`ApplicationError::show`].
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to get the library applet launchable event.
    #[error("failed to get the launchable event")]
    LaunchableEvent(#[source] GetLibraryAppletLaunchableEventError),
    /// Failed to wait for the system to allow a library applet launch.
    #[error("failed to wait for the launchable event")]
    WaitLaunchable(#[source] nx_svc::sync::WaitSyncError),
    /// Failed to create the error applet.
    #[error("failed to create the error applet")]
    CreateApplet(#[source] CreateLibraryAppletError),
    /// Failed to get the applet state-changed event.
    #[error("failed to get the state-changed event")]
    StateChangedEvent(#[source] GetAppletStateChangedEventError),
    /// Failed to push the common arguments storage.
    #[error("failed to push the common arguments")]
    PushCommonArguments(#[source] PushStorageError),
    /// Failed to push the error argument storage.
    #[error("failed to push the error argument")]
    PushArgument(#[source] PushStorageError),
    /// Failed to start the applet.
    #[error("failed to start the applet")]
    Start(#[source] StartError),
    /// Failed to wait for the applet to exit.
    #[error("failed to wait for the applet to exit")]
    Join(#[source] JoinError),
    /// The applet terminated abnormally.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
    /// Failed to pop the applet's reply storage.
    #[error("failed to pop the reply storage")]
    PopOutData(#[source] PopOutDataError),
    /// Failed to read the applet's reply storage.
    #[error("failed to read the reply storage")]
    ReadReply(#[source] ReadStorageError),
    /// The applet reported a non-zero status.
    #[error("the applet reported status {0}")]
    AppletStatus(u8),
}

impl ToResultCode for ShowError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::LaunchableEvent(err) => err.to_rc(),
            Self::WaitLaunchable(err) => err.to_rc(),
            Self::CreateApplet(err) => err.to_rc(),
            Self::StateChangedEvent(err) => err.to_rc(),
            Self::PushCommonArguments(err) | Self::PushArgument(err) => err.to_rc(),
            Self::Start(err) => err.to_rc(),
            Self::Join(err) => err.to_rc(),
            Self::PopOutData(err) => err.to_rc(),
            Self::ReadReply(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for either.
            Self::AbnormalExit(_) | Self::AppletStatus(_) => GENERIC_ERROR,
        }
    }
}

/// Creates a storage holding `payload` and pushes it to the applet.
///
/// The server copies the contents, so the storage is closed as this returns.
fn push_storage(
    creator: &LibraryAppletCreator<'_>,
    accessor: &LibraryAppletAccessor<'_>,
    payload: &[u8],
) -> Result<(), PushStorageError> {
    // Widening cast: every payload here is a fixed-size arg struct of at most
    // 0x1014 bytes.
    let size = payload.len() as i64;

    let storage = creator
        .create_storage(size)
        .map_err(PushStorageError::Create)?;

    storage.write(0, payload).map_err(PushStorageError::Write)?;

    accessor
        .push_in_data(&storage)
        .map_err(PushStorageError::Push)
}

/// Error returned by [`push_storage`].
#[derive(Debug, thiserror::Error)]
pub enum PushStorageError {
    /// Failed to create the storage.
    #[error("failed to create the storage")]
    Create(#[source] CreateStorageError),
    /// Failed to write the payload into the storage.
    #[error("failed to write the storage")]
    Write(#[source] WriteStorageError),
    /// Failed to push the storage to the applet.
    #[error("failed to push the storage")]
    Push(#[source] PushInDataError),
}

impl ToResultCode for PushStorageError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Create(err) => err.to_rc(),
            Self::Write(err) => err.to_rc(),
            Self::Push(err) => err.to_rc(),
        }
    }
}
