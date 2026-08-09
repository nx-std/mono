//! Running a library applet from start to reply.
//!
//! [`launch`] drives the sequence documented in
//! [`library_applet`](crate::library_applet) end to end: wait for the launchable
//! event, create the applet, push the common arguments and then the caller's
//! payloads, start, wait for the user, and read the reply. It is libnx's
//! `libappletLaunch`.
//!
//! An applet that exchanges interactive storages while running cannot use
//! [`launch`]; it drives [`LibraryAppletCreator`] and [`LibraryAppletAccessor`]
//! itself.

use zerocopy::IntoBytes as _;

use super::{
    accessor::{
        GetAppletStateChangedEventError,
        JoinError,
        LibraryAppletAccessor,
        PopOutDataError,
        PushInDataError,
        StartError,
    },
    creator::{
        CreateLibraryAppletError,
        CreateStorageError,
    },
    storage::{
        ReadStorageError,
        WriteStorageError,
    },
};
use crate::{
    LibraryAppletCreator,
    SelfController,
    cmif::GetLibraryAppletLaunchableEventError,
    proto::{
        AppletId,
        LibraryAppletArgs,
        LibraryAppletExitReason,
        LibraryAppletMode,
    },
};

/// Which library applet to launch, and how.
///
/// The three fields are constants of the applet rather than of the call, so a
/// crate wrapping one applet declares a single value of this and reuses it.
#[derive(Debug, Clone, Copy)]
pub struct LibraryApplet {
    /// The applet to create.
    pub id: AppletId,
    /// How the applet is presented once started.
    pub mode: LibraryAppletMode,
    /// Library applet API version the applet is addressed with.
    ///
    /// Carried in the common arguments; each applet defines its own numbering.
    pub la_version: u32,
    /// Whether the applet plays its startup sound.
    ///
    /// Part of how the applet is launched rather than of what it is asked to do,
    /// so an applet that varies it declares one value of this per variant.
    pub play_startup_sound: bool,
}

/// Launches `applet` with `payloads`, blocking until it exits.
///
/// The payload storages are pushed in the order given, after the common
/// arguments. Most applets read a single one; those that read several depend on
/// the order, so fixing it is the caller's job. The controller applet is one: it
/// reads a request storage and then the request's own arguments.
///
/// Reads the applet's reply into `reply` when one is expected. Pass [`None`] for
/// an applet that returns no data.
///
/// The exit reason is returned rather than judged: an applet the user dismissed
/// reports [`LibraryAppletExitReason::Canceled`] through `Ok`, because whether
/// that counts as a failure is the caller's question, not this function's.
///
/// This blocks on the user, so it must not be called from a context that cannot
/// wait indefinitely, and it performs IPC, so it must not be called from one
/// where IPC may already be broken.
///
/// # Errors
///
/// Returns a [`LaunchError`] naming the step that failed. A failure after the
/// applet was created leaves it to be torn down when the accessor drops.
pub fn launch(
    self_controller: &SelfController<'_>,
    creator: &LibraryAppletCreator<'_>,
    applet: &LibraryApplet,
    payloads: &[&[u8]],
    reply: Option<&mut [u8]>,
) -> Result<LibraryAppletExitReason, LaunchError> {
    let launchable = self_controller
        .get_library_applet_launchable_event()
        .map_err(LaunchError::LaunchableEvent)?;

    // SAFETY: The handle names the event the server just issued, and the wait
    // borrows it for the call.
    unsafe { nx_svc::sync::wait_synchronization_single(&launchable, u64::MAX) }
        .map_err(LaunchError::WaitLaunchable)?;

    let accessor = creator
        .create_library_applet(applet.id, applet.mode)
        .map_err(LaunchError::CreateApplet)?;

    // Obtained before Start: the applet may exit before we would otherwise get
    // around to asking, and the event is what tells us that happened.
    let state_changed = accessor
        .get_applet_state_changed_event()
        .map_err(LaunchError::StateChangedEvent)?;

    let tick = nx_cpu::counter::ticks().to_raw();
    let args = LibraryAppletArgs::new(applet.la_version, tick, applet.play_startup_sound);

    // The common arguments must be storage 0: the applet reads its own argument
    // struct from the storages that follow, so pushing a payload first makes it
    // decode the header as arguments.
    push_storage(creator, &accessor, args.as_bytes()).map_err(LaunchError::PushArgs)?;
    for payload in payloads {
        push_storage(creator, &accessor, payload).map_err(LaunchError::PushPayload)?;
    }

    accessor.start().map_err(LaunchError::Start)?;

    let exit_reason = accessor.join(&state_changed).map_err(LaunchError::Join)?;

    // Only an applet that ran to completion pushed a reply. Popping after a
    // cancellation asks for a storage the applet never queued.
    if exit_reason != LibraryAppletExitReason::Normal {
        return Ok(exit_reason);
    }

    if let Some(reply) = reply {
        let storage = accessor.pop_out_data().map_err(LaunchError::PopOutData)?;
        storage.read(0, reply).map_err(LaunchError::ReadReply)?;
    }

    Ok(exit_reason)
}

/// Error returned by [`launch`].
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    /// Failed to get the library applet launchable event.
    #[error("failed to get the launchable event")]
    LaunchableEvent(#[source] GetLibraryAppletLaunchableEventError),
    /// Failed to wait for the system to allow a library applet launch.
    #[error("failed to wait for the launchable event")]
    WaitLaunchable(#[source] nx_svc::sync::WaitSyncError),
    /// Failed to create the applet.
    #[error("failed to create the applet")]
    CreateApplet(#[source] CreateLibraryAppletError),
    /// Failed to get the applet state-changed event.
    #[error("failed to get the state-changed event")]
    StateChangedEvent(#[source] GetAppletStateChangedEventError),
    /// Failed to push the [`LibraryAppletArgs`] storage.
    #[error("failed to push the library applet args")]
    PushArgs(#[source] PushStorageError),
    /// Failed to push the applet-specific payload storage.
    #[error("failed to push the payload")]
    PushPayload(#[source] PushStorageError),
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
impl nx_sf::error::ToResultCode for LaunchError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::LaunchableEvent(err) => err.to_rc(),
            Self::WaitLaunchable(err) => nx_svc::error::ToResultCode::to_rc(err),
            Self::CreateApplet(err) => err.to_rc(),
            Self::StateChangedEvent(err) => err.to_rc(),
            Self::PushArgs(err) | Self::PushPayload(err) => err.to_rc(),
            Self::Start(err) => err.to_rc(),
            Self::Join(err) => err.to_rc(),
            Self::PopOutData(err) => err.to_rc(),
            Self::ReadReply(err) => err.to_rc(),
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
    // a few kilobytes.
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

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for PushStorageError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Create(err) => err.to_rc(),
            Self::Write(err) => err.to_rc(),
            Self::Push(err) => err.to_rc(),
        }
    }
}
