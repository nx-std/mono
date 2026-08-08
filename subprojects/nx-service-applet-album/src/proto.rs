//! Wire values the album applet reads.

/// The applet's single-byte argument storage.
///
/// libnx calls this `AlbumLaArg`. Unlike most library applets the payload is one
/// byte rather than a struct, so there is nothing here to lay out with zerocopy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AlbumLaArg {
    /// Only the files the launching application created, with the filter button
    /// disabled.
    ShowAlbumFiles = 0,
    /// Every file, with filtering allowed.
    ShowAllAlbumFiles = 1,
    /// Every file, as the HOME menu opens it.
    ShowAllAlbumFilesForHomeMenu = 2,
}

impl AlbumLaArg {
    /// Returns the raw byte this argument is pushed as.
    #[inline]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}
