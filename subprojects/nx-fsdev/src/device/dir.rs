//! One open directory walk on a mounted filesystem.
//!
//! The server hands back as many entries as fit the buffer it is given, so a walk is read in
//! batches rather than one entry at a time: a directory of forty entries costs two commands
//! instead of forty. The batch is heap-held because a single entry is 0x310 bytes, most of it the
//! name buffer, and a stack that size is not something a device should take.
//!
//! There is no rewind. Horizon's directory object cannot be reset, and neither the descriptor
//! table nor libnx before it offers one, so [`nx_sys_fd::device::Dir::reset`] is left at its
//! default and a caller asking for it is refused.

use alloc::vec::Vec;

use nx_service_fs::{
    DirectoryEntry,
    FS_MAX_PATH,
};
use nx_sys_fd::device::{
    DeviceError,
    Dir,
    DirEntry,
    EntryName,
    Metadata,
};

use crate::service;

/// How many entries one read asks the server for.
///
/// libnx exposes the same number as a weak symbol a program may override. Nothing in this
/// workspace has ever changed it, so it is fixed here rather than reintroduced as a knob.
const BATCH_LEN: usize = 32;

/// What the server reports for a directory entry.
const ENTRY_TYPE_DIR: i8 = 0;

/// An open directory walk, addressed by the id the server issued for it.
pub struct FsDeviceDir {
    /// Domain object id of the `IDirectory`, until it is closed.
    object_id: Option<u32>,
    /// Entries read from the server and not yet handed out.
    batch: Vec<DirectoryEntry>,
    /// How many of [`Self::batch`] have been handed out.
    taken: usize,
    /// Whether the server has reported the end of the directory.
    exhausted: bool,
}

impl FsDeviceDir {
    /// Adopts the directory `object_id` names.
    ///
    /// The caller must hold the close obligation for `object_id`: this walk takes it over and
    /// closes it on drop.
    pub(crate) fn from_raw_object_id_unchecked(object_id: u32) -> Self {
        Self {
            object_id: Some(object_id),
            batch: Vec::new(),
            taken: 0,
            exhausted: false,
        }
    }

    /// Reads the next batch of entries, reporting whether any arrived.
    fn refill(&mut self) -> Result<bool, DeviceError> {
        let Some(object_id) = self.object_id else {
            return Err(DeviceError::Io);
        };

        self.batch.resize(BATCH_LEN, empty_entry());
        self.taken = 0;

        let read = service::with_dir(object_id, |dir| dir.read(&mut self.batch))?;
        // Clamped, and the server never reports more entries than the buffer held.
        let read = read.max(0) as usize;

        self.batch.truncate(read.min(BATCH_LEN));
        self.exhausted = read < BATCH_LEN;

        Ok(!self.batch.is_empty())
    }
}

impl Dir for FsDeviceDir {
    fn next(&mut self) -> Result<Option<DirEntry>, DeviceError> {
        if self.taken == self.batch.len() && (self.exhausted || !self.refill()?) {
            return Ok(None);
        }

        // SAFETY: the guard above leaves `taken` below `batch.len()`, either because the batch was
        // not exhausted or because a refill replaced it with a non-empty one.
        let entry = self.batch[self.taken];
        self.taken += 1;

        let name = name_of(&entry.name)?;
        let metadata = if entry.entry_type == ENTRY_TYPE_DIR {
            Metadata::dir()
        } else {
            // Clamped, so the cast is from a non-negative `i64`.
            Metadata::file(entry.file_size.max(0) as u64)
        };

        Ok(Some(DirEntry { name, metadata }))
    }
}

impl Drop for FsDeviceDir {
    fn drop(&mut self) {
        if let Some(object_id) = self.object_id.take() {
            service::close_dir(object_id);
        }
    }
}

/// Returns the entry name held in `name`, which the server nul-terminates.
///
/// # Errors
///
/// Returns [`DeviceError::Io`] when the name does not fit what the C caller's buffer can hold.
/// There is nowhere to deliver such an entry, and skipping it silently would report a directory
/// with fewer entries than it has.
fn name_of(name: &[u8; FS_MAX_PATH]) -> Result<EntryName, DeviceError> {
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    EntryName::try_from(&name[..end]).map_err(|_| DeviceError::Io)
}

/// Returns an entry with nothing in it, for the buffer a read fills.
fn empty_entry() -> DirectoryEntry {
    DirectoryEntry {
        name: [0; FS_MAX_PATH],
        pad: [0; 3],
        entry_type: 0,
        pad2: [0; 3],
        file_size: 0,
    }
}
