//! What it means to be an open file.
//!
//! A [`File`] is the object behind one open descriptor. Unlike a [`crate::device::Device`], which is
//! registered once and shared by every descriptor opened against it, a file is created per
//! descriptor and owned by the descriptor table for exactly as long as that descriptor is open.
//!
//! That is why the operations here take `&mut self` and the device's take `&self`: a file has a
//! position and a session of its own, and the table proves exclusive access before calling in, so
//! an implementation does not have to lock anything itself.

use super::{
    error::DeviceError,
    metadata::Metadata,
};

/// The object behind one open descriptor.
///
/// Created by [`crate::device::Device::open`], owned by the descriptor table, and dropped when the
/// descriptor is closed. Dropping happens with no table lock held, so an implementation may block
/// while releasing whatever it holds.
///
/// Every operation defaults to reporting [`DeviceError::Unsupported`], so an implementation writes
/// only what it actually offers: a file opened read-only implements [`File::read`] and leaves
/// [`File::write`] alone.
pub trait File: Send {
    /// Reads into `buf` from the current position, returning how many bytes were produced.
    ///
    /// Returning `Ok(0)` means end of file. A short read is not an error.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the file cannot be read, or [`DeviceError::Io`]
    /// when it failed to produce bytes.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::Unsupported)
    }

    /// Writes `buf` at the current position, returning how many bytes were consumed.
    ///
    /// A short write is not an error; the caller is expected to retry with the remainder.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the file cannot be written, or
    /// [`DeviceError::Io`] when it rejected the bytes.
    fn write(&mut self, buf: &[u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::Unsupported)
    }

    /// Moves the position, returning where it ended up as an offset from the start.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the file has no position, or
    /// [`DeviceError::InvalidPath`] when the requested position is before the start of the file.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, DeviceError> {
        let _ = pos;
        Err(DeviceError::Unsupported)
    }

    /// Reports what this file is and how large it is.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the file reports nothing about itself, or
    /// [`DeviceError::Io`] when the query failed.
    fn metadata(&self) -> Result<Metadata, DeviceError> {
        Err(DeviceError::Unsupported)
    }

    /// Resizes the file to `len` bytes, padding with zeroes when it grows.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the file cannot be resized, or
    /// [`DeviceError::Io`] when the resize failed.
    fn set_len(&mut self, len: u64) -> Result<(), DeviceError> {
        let _ = len;
        Err(DeviceError::Unsupported)
    }

    /// Commits everything written so far to the underlying storage.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the file has nothing to commit, or
    /// [`DeviceError::Io`] when the commit failed.
    fn sync(&mut self) -> Result<(), DeviceError> {
        Err(DeviceError::Unsupported)
    }

    /// Releases what this file holds, reporting whatever could not be finished.
    ///
    /// Called once, by the descriptor table, immediately before the file is dropped. It exists
    /// alongside `Drop` because closing can fail in a way a caller wants to hear about: a buffered
    /// write flushed at close may be the first point the storage refuses it, and a destructor has
    /// nowhere to report that. An implementation that cannot fail leaves this alone and does its
    /// releasing in `Drop`.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when releasing failed. The descriptor is gone either way, so
    /// this reports what could not be finished rather than a reason to retry.
    fn close(&mut self) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// Where a seek is measured from.
///
/// Mirrors `std::io::SeekFrom`, so the eventual `std` port can pass one through unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// From the beginning of the file, forwards.
    Start(u64),
    /// From the end of the file, where a negative offset moves backwards.
    End(i64),
    /// From the current position, where a negative offset moves backwards.
    Current(i64),
}

/// What a caller asked for when opening a path.
///
/// The C standard library passes the `open(2)` flag word, which is decoded once at the boundary so
/// that no device has to know the bit values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenFlags {
    /// The caller intends to read.
    pub read: bool,
    /// The caller intends to write.
    pub write: bool,
    /// Every write goes to the end of the file, regardless of the position.
    pub append: bool,
    /// Create the entry when it does not exist.
    pub create: bool,
    /// Fail when `create` is set and the entry already exists.
    pub exclusive: bool,
    /// Discard the existing contents on open.
    pub truncate: bool,
}
