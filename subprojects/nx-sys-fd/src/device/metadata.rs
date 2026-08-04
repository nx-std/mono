//! What a device reports about an entry.
//!
//! The C standard library asks for `struct stat`, which has around a dozen fields inherited from
//! Unix. Almost none of them mean anything on Horizon: there are no inodes, no owners, no link
//! counts, and no device numbers. A device that had to fill in a `struct stat` would be inventing
//! most of it.
//!
//! [`Metadata`] is therefore the subset that is real: what kind of entry this is, how large it is,
//! and when it was touched. Translating that into the C structure, including the invented parts, is
//! the boundary's job and happens once, in [`crate::ffi`].

/// What kind of entry a path names.
///
/// Horizon distinguishes exactly these two. There are no symbolic links, sockets, or device nodes,
/// so there is nothing else a device could report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// A regular file, holding bytes.
    File,
    /// A directory, holding entries.
    Dir,
}

/// When an entry was touched, in seconds since the Unix epoch.
///
/// Horizon reports all three together or not at all, which is why this is one type rather than
/// three optional fields on [`Metadata`]: a device cannot know one timestamp without knowing the
/// others, and modelling them separately would suggest otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamps {
    /// When the entry was created.
    pub created: u64,
    /// When the entry was last written.
    pub modified: u64,
    /// When the entry was last read.
    pub accessed: u64,
}

/// What a device reports about one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    /// What kind of entry this is.
    pub file_type: FileType,
    /// Size in bytes. Zero for a directory, whose size Horizon does not report.
    pub size: u64,
    /// When the entry was touched, when the filesystem tracks it.
    ///
    /// `None` is the honest answer for a filesystem that does not keep timestamps, and for one
    /// that keeps them but reported them invalid for this entry.
    pub timestamps: Option<Timestamps>,
}

impl Metadata {
    /// Describes a regular file of `size` bytes, with no timestamps.
    pub const fn file(size: u64) -> Self {
        Self {
            file_type: FileType::File,
            size,
            timestamps: None,
        }
    }

    /// Describes a directory, with no timestamps.
    pub const fn dir() -> Self {
        Self {
            file_type: FileType::Dir,
            size: 0,
            timestamps: None,
        }
    }

    /// Returns this metadata with `timestamps` attached.
    pub const fn with_timestamps(self, timestamps: Timestamps) -> Self {
        Self {
            timestamps: Some(timestamps),
            ..self
        }
    }
}

/// How much space a filesystem has.
///
/// Reported per mounted filesystem rather than per entry, which is why it is separate from
/// [`Metadata`] even though both end up in a C structure at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceInfo {
    /// Size of the unit `total_blocks` and `free_blocks` count, in bytes.
    pub block_size: u64,
    /// Total capacity, in blocks.
    pub total_blocks: u64,
    /// Capacity not yet used, in blocks.
    pub free_blocks: u64,
}
