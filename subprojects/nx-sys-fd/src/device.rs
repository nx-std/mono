//! What it means to be a device.
//!
//! A device is a named place paths resolve into: `"sdmc:/file"` names the device registered as
//! `sdmc`, and everything after the colon is that device's business. Implementing one is the point
//! of this crate, and it is done in Rust: a device implements [`Device`] and registers itself, and
//! never sees a descriptor number, a raw pointer, or an error number.
//!
//! ## Three traits, not one
//!
//! The C standard library packs every operation into a single table of function pointers and tells
//! them apart by what they are handed: some take a path, some take per-descriptor state, some take
//! a directory iterator. That is one table because C has no better way to group them, not because
//! they are one concern.
//!
//! Here they are three:
//!
//! - [`Device`] is registered once and shared. It owns the path namespace: looking an entry up,
//!   creating and removing entries, and producing the objects below. Its operations take `&self`,
//!   because every descriptor opened against it calls them concurrently.
//! - [`File`] is one open file. It has a position and whatever session the device needed, it is
//!   owned by the descriptor table, and its operations take `&mut self` because the table proves
//!   exclusive access before calling in.
//! - [`Dir`] is one open directory walk, owned by the directory table, for the same reasons.
//!
//! Splitting them is what lets a filesystem implement the path operations without pretending its
//! descriptors are interchangeable with a console's.
//!
//! ## Streams keep no object
//!
//! [`Device::write`] and [`Device::read`] look redundant next to [`File`], and they are not. A
//! descriptor reaches a device one of two ways, and the difference is load-bearing:
//!
//! - **Opened by path.** [`Device::open`] produces a [`File`] that the descriptor owns until it is
//!   closed. The position and the session live in that object.
//! - **Bound without a path.** The standard descriptors are already open before anything runs, and
//!   they hold no object at all: each operation resolves the registry slot afresh and calls the
//!   device directly.
//!
//! The second is why a console can take slot 1 over from the null device and have the very next
//! `printf` arrive, without reopening anything. Had a stream descriptor cached an object the way an
//! opened file does, that rebinding would have to hunt down and replace every descriptor already
//! pointing at the slot. Keeping streams objectless is what makes the rebinding free.
//!
//! ## Everything is optional
//!
//! Every operation on all three traits has a default that reports [`DeviceError::Unsupported`], so
//! an implementation writes only what it actually offers, and a caller reaching for the rest gets a
//! clean refusal rather than a stub that lies.

mod dir;
mod error;
mod file;
mod metadata;

use alloc::boxed::Box;
use core::ffi::CStr;

pub use self::{
    dir::{
        Dir,
        DirEntry,
        EntryName,
        InvalidEntryName,
        MAX_NAME_LEN,
    },
    error::DeviceError,
    file::{
        File,
        OpenFlags,
        SeekFrom,
    },
    metadata::{
        FileType,
        Metadata,
        SpaceInfo,
        Timestamps,
    },
};

/// A named place paths resolve into.
///
/// Implementations are registered once, live for the life of the process, and are shared by every
/// descriptor opened against them, so they take `&self` and keep no per-descriptor state. What a
/// descriptor needs lives in the [`File`] or [`Dir`] the device produces.
pub trait Device: Sync {
    /// Name this device is reached by, as the prefix of a path such as `"sdmc:/file"`.
    fn name(&self) -> &'static CStr;

    /// Opens `path`, producing the object that will serve the descriptor.
    ///
    /// `path` is what followed the device name, so it never carries the `"name:"` prefix.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device has no paths to open,
    /// [`DeviceError::NotFound`] when the entry does not exist and `flags` did not ask to create
    /// it, [`DeviceError::AlreadyExists`] when it exists and `flags` demanded it did not, and
    /// [`DeviceError::Io`] when opening failed.
    fn open(&self, path: &CStr, flags: OpenFlags) -> Result<Box<dyn File>, DeviceError> {
        let _ = (path, flags);
        Err(DeviceError::Unsupported)
    }

    /// Writes `buf` to a descriptor bound to this device without a path, returning how many bytes
    /// were consumed.
    ///
    /// This is how the standard descriptors work: nothing opens `stdout`, it is simply already
    /// there, bound to whichever device holds slot 1. Such a descriptor has no object of its own,
    /// which is what lets a console take slot 1 over from the null device and have the next
    /// `printf` reach it. A device that only exists to be written to implements this and leaves
    /// [`Device::open`] alone; a filesystem does the reverse.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device is only reachable by path, or
    /// [`DeviceError::Io`] when it rejected the bytes.
    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::Unsupported)
    }

    /// Reads into `buf` on a descriptor bound to this device without a path, returning how many
    /// bytes were produced.
    ///
    /// The read counterpart of [`Device::write`], and bound by the same reasoning.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device is only reachable by path, or
    /// [`DeviceError::Io`] when it failed to produce bytes.
    fn read(&self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::Unsupported)
    }

    /// Opens `path` as a directory, producing the object that will walk it.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device has no directories,
    /// [`DeviceError::NotFound`] when the path names nothing, [`DeviceError::InvalidPath`] when it
    /// names a file, and [`DeviceError::Io`] when opening failed.
    fn open_dir(&self, path: &CStr) -> Result<Box<dyn Dir>, DeviceError> {
        let _ = path;
        Err(DeviceError::Unsupported)
    }

    /// Reports what `path` names, without opening it.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device reports nothing about its entries,
    /// [`DeviceError::NotFound`] when the path names nothing, and [`DeviceError::Io`] when the
    /// query failed.
    fn metadata(&self, path: &CStr) -> Result<Metadata, DeviceError> {
        let _ = path;
        Err(DeviceError::Unsupported)
    }

    /// Removes the file `path` names.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device is read-only,
    /// [`DeviceError::NotFound`] when the path names nothing, [`DeviceError::InvalidPath`] when it
    /// names a directory, and [`DeviceError::Io`] when the removal failed.
    fn remove_file(&self, path: &CStr) -> Result<(), DeviceError> {
        let _ = path;
        Err(DeviceError::Unsupported)
    }

    /// Creates a directory at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device is read-only,
    /// [`DeviceError::AlreadyExists`] when something is already there, and [`DeviceError::Io`] when
    /// the creation failed.
    fn create_dir(&self, path: &CStr) -> Result<(), DeviceError> {
        let _ = path;
        Err(DeviceError::Unsupported)
    }

    /// Removes the directory `path` names, which must be empty.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device is read-only,
    /// [`DeviceError::NotFound`] when the path names nothing, [`DeviceError::InvalidPath`] when it
    /// names a file or a directory that is not empty, and [`DeviceError::Io`] when the removal
    /// failed.
    fn remove_dir(&self, path: &CStr) -> Result<(), DeviceError> {
        let _ = path;
        Err(DeviceError::Unsupported)
    }

    /// Moves the entry at `from` to `to`.
    ///
    /// Both paths belong to this device: the C standard library refuses a rename that crosses
    /// devices before it reaches here.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device is read-only,
    /// [`DeviceError::NotFound`] when `from` names nothing, [`DeviceError::AlreadyExists`] when
    /// `to` is occupied, and [`DeviceError::Io`] when the move failed.
    fn rename(&self, from: &CStr, to: &CStr) -> Result<(), DeviceError> {
        let _ = (from, to);
        Err(DeviceError::Unsupported)
    }

    /// Makes `path` the working directory for later relative paths on this device.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device has no working directory,
    /// [`DeviceError::NotFound`] when the path names nothing, [`DeviceError::InvalidPath`] when it
    /// names a file, and [`DeviceError::Io`] when the change failed.
    fn set_current_dir(&self, path: &CStr) -> Result<(), DeviceError> {
        let _ = path;
        Err(DeviceError::Unsupported)
    }

    /// Reports how much space the filesystem holding `path` has.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device does not measure space, or
    /// [`DeviceError::Io`] when the query failed.
    fn space_info(&self, path: &CStr) -> Result<SpaceInfo, DeviceError> {
        let _ = path;
        Err(DeviceError::Unsupported)
    }
}

/// Number of registry slots, and so the bound on a [`DeviceId`].
///
/// Fixed by the C standard library, which sizes its own view of the registry with this value.
pub const MAX_DEVICES: usize = 35;

/// Where a device sits in the registry.
///
/// The C standard library indexes devices by this number, and a descriptor records which device
/// backs it. A value of this type names a slot that exists; whether a device occupies that slot is
/// a separate question the registry answers.
///
/// Validation lives in the [`TryFrom<usize>`] impl below, which is the only place the bound is
/// checked. [`DeviceId::from_index_unchecked`] bypasses it for callers that already hold the proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceId(u32);

impl DeviceId {
    /// Names the device in registry slot `index` without checking the bound.
    ///
    /// The caller must ensure `index` is below [`MAX_DEVICES`]. This constructor performs no
    /// validation; an out-of-range slot resolves to no device, so a lookup returns nothing and a
    /// descriptor opened against it is refused.
    pub(crate) const fn from_index_unchecked(index: usize) -> Self {
        Self(index as u32)
    }

    /// Returns the registry slot this names.
    ///
    /// A device implementation needs the number to hand back to the C standard library, which
    /// addresses a registered device by its slot.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Returns the raw slot number, for the C boundary.
    #[cfg(feature = "ffi")]
    pub(crate) const fn as_raw(self) -> u32 {
        self.0
    }
}

impl TryFrom<usize> for DeviceId {
    type Error = InvalidDeviceId;

    fn try_from(index: usize) -> Result<Self, Self::Error> {
        if index >= MAX_DEVICES {
            return Err(InvalidDeviceId(index));
        }
        Ok(Self(index as u32))
    }
}

/// Errors returned when converting a slot number into a [`DeviceId`].
///
/// The number is outside the registry, so it names no slot at all. Nothing was looked up.
#[derive(Debug, thiserror::Error)]
#[error("Slot {0} is outside the device registry")]
pub struct InvalidDeviceId(usize);
