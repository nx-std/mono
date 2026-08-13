//! The SD card, where the report is kept once the run is over.
//!
//! The console draws its text straight to the framebuffer and keeps no copy, so a run that was only
//! shown cannot be read back. The card is what makes a run launched by hand -- with no host to
//! report to -- readable at all, which is why it is written whether or not a host is listening.

use alloc::string::String;
use core::fmt::Write as _;

use nx_std::fs::File;
use nx_std_path::Path;

/// Writes `text` as the report for `suite`, under `report_dir`.
///
/// # Errors
///
/// Returns [`WriteError`] when the report could not be filed, which loses the file and nothing
/// else: the console has already shown the run and the host is sent it regardless.
pub fn write(report_dir: &str, suite: &str, text: &str) -> Result<(), WriteError> {
    // The directory is created on the way: a card that has never run a test has no such directory,
    // and it already existing is the ordinary case rather than a failure.
    let _ = nx_std::fs::create_dir(Path::new(report_dir));

    let mut path = String::new();
    write!(&mut path, "{report_dir}/{suite}.tap").map_err(|_| WriteError::Path)?;

    let mut file = File::create(Path::new(&path)).map_err(WriteError::Create)?;
    file.write_all(text.as_bytes()).map_err(WriteError::Write)?;

    // What the writes produced is not on the card until the close says so: the last of it is still
    // held in a buffer, and a close that cannot place it is how a card that filled up reports
    // itself.
    file.close().map_err(WriteError::Close)
}

/// Errors returned by [`write`].
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The path could not be built
    #[error("the report path could not be built")]
    Path,

    /// The file could not be created
    #[error("the report file could not be created")]
    Create(#[source] nx_std::fs::Error),

    /// The document could not be written
    #[error("the report could not be written")]
    Write(#[source] nx_std::fs::Error),

    /// The file could not be closed, so what was written may not be on the card
    #[error("the report file could not be closed")]
    Close(#[source] nx_std::fs::Error),
}
