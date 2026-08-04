//! What it means to be an open directory.
//!
//! A [`Dir`] is the object behind one open directory iterator. It is the directory counterpart of
//! [`crate::device::File`], and it exists separately for the same reason the C standard library
//! keeps `DIR` separate from a file descriptor: a directory is walked, not read, and it has no
//! position a caller can address.

use super::{
    error::DeviceError,
    metadata::Metadata,
};

/// The object behind one open directory iterator.
///
/// Created by [`crate::device::Device::open_dir`], owned for as long as the iterator is open, and
/// dropped when it is closed. Dropping happens with no lock held, so an implementation may block
/// while releasing whatever it holds.
pub trait Dir: Send {
    /// Produces the next entry, or `None` once the directory is exhausted.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when the directory could not be read, which is distinct from
    /// reaching the end.
    fn next(&mut self) -> Result<Option<DirEntry>, DeviceError>;

    /// Restarts the walk from the first entry.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the directory cannot be rewound, or
    /// [`DeviceError::Io`] when the rewind failed.
    fn reset(&mut self) -> Result<(), DeviceError> {
        Err(DeviceError::Unsupported)
    }
}

/// One entry in a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirEntry {
    /// The entry's name, relative to the directory being walked.
    pub name: EntryName,
    /// What the entry is, and how large.
    pub metadata: Metadata,
}

/// The name of one directory entry.
///
/// Holds its bytes inline so that walking a directory allocates nothing per entry. The bytes carry
/// no trailing nul; adding one is the boundary's job, which is why the C caller's buffer is one
/// byte longer than the longest name allowed here.
///
/// Validation lives in the [`TryFrom<&[u8]>`] impl below, which is the only way to build one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryName {
    bytes: [u8; MAX_NAME_LEN],
    len: usize,
}

impl EntryName {
    /// Returns the name, which carries no trailing nul.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl TryFrom<&[u8]> for EntryName {
    type Error = InvalidEntryName;

    fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
        if name.len() > MAX_NAME_LEN {
            return Err(InvalidEntryName::TooLong(name.len()));
        }
        if name.contains(&0) {
            return Err(InvalidEntryName::InteriorNul);
        }

        let mut bytes = [0u8; MAX_NAME_LEN];
        bytes[..name.len()].copy_from_slice(name);
        Ok(Self {
            bytes,
            len: name.len(),
        })
    }
}

/// Errors returned when converting bytes into an [`EntryName`].
#[derive(Debug, thiserror::Error)]
pub enum InvalidEntryName {
    /// The name does not fit the buffer the C standard library provides
    ///
    /// Occurs when a filesystem reports a name longer than [`MAX_NAME_LEN`]. Nothing was copied,
    /// and the entry cannot be delivered at all.
    #[error("Entry name of {0} bytes exceeds the maximum")]
    TooLong(usize),

    /// The name contains a nul byte
    ///
    /// Occurs when a filesystem reports a name that cannot be expressed as a C string, so
    /// delivering it would silently truncate at the nul.
    #[error("Entry name contains an interior nul byte")]
    InteriorNul,
}

/// Longest entry name that can be reported, in bytes.
///
/// Fixed by the C standard library, whose `readdir` hands the device a `NAME_MAX + 1` buffer to
/// write into. A filesystem may hold a longer name, but there is nowhere to deliver it.
pub const MAX_NAME_LEN: usize = 255;
