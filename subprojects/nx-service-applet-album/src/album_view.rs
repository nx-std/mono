//! Launching the Album on a chosen set of files.

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

use crate::proto::AlbumLaArg;

/// Library applet API version the album applet is addressed with.
///
/// libnx passes 0x10000 for every `photoViewer` launch.
const ALBUM_LA_VERSION: u32 = 0x10000;

/// Which set of album files the applet presents.
///
/// The variant fixes both the argument byte and whether the startup sound
/// plays; libnx pairs the two by hand at each of its three entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlbumView {
    /// Only the files the launching application created, with the filter button
    /// disabled.
    ApplicationFiles,
    /// Every file, with filtering allowed.
    AllFiles,
    /// Every file, opened as the HOME menu opens it, startup sound included.
    AllFilesForHomeMenu,
}

impl AlbumView {
    /// Shows the Album, blocking until the user leaves it.
    ///
    /// The applet returns no data, so a run that reached the user is a success
    /// whether they browsed or backed straight out.
    ///
    /// This blocks on the user, so it must not be called from a context that
    /// cannot wait indefinitely, and it performs IPC, so it must not be called
    /// from one where IPC may already be broken.
    ///
    /// # Errors
    ///
    /// Returns a [`ShowError`] when the applet could not be presented or exited
    /// abnormally.
    pub fn show(
        self,
        self_controller: &SelfController<'_>,
        creator: &LibraryAppletCreator<'_>,
    ) -> Result<(), ShowError> {
        let applet = LibraryApplet {
            id: AppletId::LibraryAppletPhotoViewer,
            mode: LibraryAppletMode::AllForeground,
            la_version: ALBUM_LA_VERSION,
            play_startup_sound: self.plays_startup_sound(),
        };

        let exit_reason = library_applet::launch(
            self_controller,
            creator,
            &applet,
            &[&[self.arg().as_raw()]],
            None,
        )
        .map_err(ShowError::Launch)?;

        match exit_reason {
            // The user left the Album, which is the only way out of it.
            LibraryAppletExitReason::Normal | LibraryAppletExitReason::Canceled => Ok(()),
            reason => Err(ShowError::AbnormalExit(reason)),
        }
    }

    /// Returns the argument byte this view is launched with.
    const fn arg(self) -> AlbumLaArg {
        match self {
            Self::ApplicationFiles => AlbumLaArg::ShowAlbumFiles,
            Self::AllFiles => AlbumLaArg::ShowAllAlbumFiles,
            Self::AllFilesForHomeMenu => AlbumLaArg::ShowAllAlbumFilesForHomeMenu,
        }
    }

    /// Returns whether this view plays the startup sound.
    ///
    /// Only the HOME menu's own launch does; libnx passes false for the other
    /// two.
    const fn plays_startup_sound(self) -> bool {
        matches!(self, Self::AllFilesForHomeMenu)
    }
}

/// Error returned by [`AlbumView::show`].
#[derive(Debug, thiserror::Error)]
pub enum ShowError {
    /// Failed to run the album applet.
    #[error("failed to launch the album applet")]
    Launch(#[source] LaunchError),
    /// The applet terminated abnormally.
    #[error("the applet exited abnormally")]
    AbnormalExit(LibraryAppletExitReason),
}

#[cfg(feature = "ffi")]
impl nx_sf::error::ToResultCode for ShowError {
    fn to_rc(self) -> nx_sf::error::ResultCode {
        match self {
            Self::Launch(err) => err.to_rc(),
            // Reported by the applet rather than by a service, so no server
            // named a code for it.
            Self::AbnormalExit(_) => nx_sf::error::GENERIC_ERROR,
        }
    }
}
