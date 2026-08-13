//! One open directory walk on a mounted image.
//!
//! A directory's contents are two chains hanging off its record, one of child directories and one
//! of child files, so walking it is following those chains in turn. Nothing is read from the image
//! while it runs: the chains are in the tables the mount already loaded.
//!
//! ## `.` and `..` are reported
//!
//! libnx's romfs device produces both before anything else, and its filesystem device produces
//! neither. That difference is visible to a program: one that walks `romfs:/` today receives them
//! and skips them by name, and a walk that stopped producing them would hand such a program two
//! entries it never had to handle. So this device keeps them, and the filesystem device keeps not
//! having them.
//!
//! ## The chains are captured, not re-read
//!
//! Where the walk has got to is a position in a chain, held here. Restarting means asking the
//! directory record for its two chain heads again, which is what makes [`Dir::reset`] free and
//! worth offering, unlike on a filesystem whose server-side directory object cannot be rewound.

use alloc::sync::Arc;

use nx_sys_fd::device::{
    DeviceError,
    Dir,
    DirEntry,
    EntryName,
    Metadata,
};

use crate::image::Image;

/// An open directory walk, addressed by where the directory sits in the image.
pub(crate) struct RomfsDir {
    /// The image the chains are followed through.
    image: Arc<Image>,
    /// Where the directory being walked sits in the directory table.
    dir: u32,
    /// How far the walk has got.
    step: Step,
}

/// How far a walk has got, and what it will produce next.
///
/// The order is the one libnx produces: the two synthetic entries, then every child directory, then
/// every child file.
enum Step {
    /// The entry naming the directory itself has not been produced yet.
    SelfEntry,
    /// The entry naming the parent has not been produced yet.
    ParentEntry,
    /// A child directory is next, or the chain has ended.
    ChildDir(Option<u32>),
    /// A child file is next, or the chain has ended.
    ChildFile(Option<u32>),
    /// Everything has been produced.
    Exhausted,
}

impl RomfsDir {
    /// Opens a walk over the directory at `dir` in `image`.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when `dir` names no directory.
    pub(crate) fn new(image: Arc<Image>, dir: u32) -> Result<Self, DeviceError> {
        // The record is read here rather than on the first entry so that opening a directory that
        // is not there fails at the open, which is where a caller expects to hear about it.
        image.children_of(dir)?;

        Ok(Self {
            image,
            dir,
            step: Step::SelfEntry,
        })
    }

    /// Produces the next child directory, and moves the walk on.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when the chain leads somewhere the table does not reach, or the
    /// entry's name is longer than one can be reported as.
    fn next_child_dir(&mut self, off: u32) -> Result<DirEntry, DeviceError> {
        let (name, sibling) = self.image.dir_entry_at(off)?;
        let entry = entry_for(name, Metadata::dir())?;

        self.step = Step::ChildDir(sibling);
        Ok(entry)
    }

    /// Produces the next child file, and moves the walk on. See [`RomfsDir::next_child_dir`].
    fn next_child_file(&mut self, off: u32) -> Result<DirEntry, DeviceError> {
        let (name, size, sibling) = self.image.file_entry_at(off)?;
        let entry = entry_for(name, Metadata::file(size))?;

        self.step = Step::ChildFile(sibling);
        Ok(entry)
    }
}

impl Dir for RomfsDir {
    fn next(&mut self) -> Result<Option<DirEntry>, DeviceError> {
        loop {
            match self.step {
                Step::SelfEntry => {
                    self.step = Step::ParentEntry;
                    return Ok(Some(entry_for(b".", Metadata::dir())?));
                }
                Step::ParentEntry => {
                    let (child_dir, _) = self.image.children_of(self.dir)?;
                    self.step = Step::ChildDir(child_dir);
                    return Ok(Some(entry_for(b"..", Metadata::dir())?));
                }
                Step::ChildDir(Some(off)) => return self.next_child_dir(off).map(Some),
                Step::ChildDir(None) => {
                    let (_, child_file) = self.image.children_of(self.dir)?;
                    self.step = Step::ChildFile(child_file);
                }
                Step::ChildFile(Some(off)) => return self.next_child_file(off).map(Some),
                Step::ChildFile(None) => self.step = Step::Exhausted,
                Step::Exhausted => return Ok(None),
            }
        }
    }

    fn reset(&mut self) -> Result<(), DeviceError> {
        self.step = Step::SelfEntry;
        Ok(())
    }
}

/// Returns the entry called `name`, described by `metadata`.
///
/// # Errors
///
/// Returns [`DeviceError::Io`] when the name does not fit what the C caller's buffer can hold.
/// There is nowhere to deliver such an entry, and skipping it silently would report a directory
/// with fewer entries than it has.
fn entry_for(name: &[u8], metadata: Metadata) -> Result<DirEntry, DeviceError> {
    let name = EntryName::try_from(name).map_err(|_| DeviceError::Io)?;
    Ok(DirEntry { name, metadata })
}
