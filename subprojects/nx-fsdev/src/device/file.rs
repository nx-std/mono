//! One open file on a mounted filesystem.
//!
//! The server's file object has no position: every read and write carries the offset it acts at.
//! The position a C caller expects therefore lives here, in the object the descriptor owns, and is
//! advanced by whatever each command reports it moved.

use nx_service_fs::{
    ReadOption,
    WriteOption,
};
use nx_sys_fd::device::{
    DeviceError,
    File,
    Metadata,
    OpenFlags,
    SeekFrom,
};

use crate::service;

/// An open file, addressed by the id the server issued for it.
pub struct FsDeviceFile {
    /// Domain object id of the `IFile`, until it is closed.
    object_id: Option<u32>,
    /// Where the next read or write acts, in bytes from the start.
    offset: u64,
    /// Every write goes to the end of the file, regardless of [`Self::offset`].
    append: bool,
}

impl FsDeviceFile {
    /// Adopts the file `object_id` names, opened with `flags`.
    ///
    /// The caller must hold the close obligation for `object_id`: this file takes it over, and
    /// closes it when the descriptor closes or the file is dropped. Handing the same id here twice
    /// would close it twice.
    pub(crate) fn from_raw_object_id_unchecked(object_id: u32, flags: OpenFlags) -> Self {
        Self {
            object_id: Some(object_id),
            offset: 0,
            append: flags.append,
        }
    }

    /// Returns the id of the file, or reports that it has already been closed.
    fn object_id(&self) -> Result<u32, DeviceError> {
        self.object_id.ok_or(DeviceError::Io)
    }

    /// Returns the current size of the file.
    fn size(&self) -> Result<u64, DeviceError> {
        let size = service::with_file(self.object_id()?, |file| file.get_size())?;
        // Clamped, so the cast is from a non-negative `i64`.
        Ok(size.max(0) as u64)
    }
}

impl File for FsDeviceFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let object_id = self.object_id()?;
        // The offset is bounded by the file size the server reports, which it reports as an `i64`,
        // so it always fits back into one.
        let offset = self.offset as i64;
        let read = service::with_file(object_id, |file| {
            file.read(offset, buf, buf.len() as u64, ReadOption::NONE)
        })?;

        self.offset += read;
        // Bounded by `buf.len()`, which is a `usize` already.
        Ok(read as usize)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, DeviceError> {
        let object_id = self.object_id()?;
        if self.append {
            self.offset = self.size()?;
        }

        // Bounded as in `read` above.
        let offset = self.offset as i64;
        service::with_file(object_id, |file| {
            file.write(offset, buf, buf.len() as u64, WriteOption::NONE)
        })?;

        // The command either writes everything or fails, so there is no short write to report.
        self.offset += buf.len() as u64;
        Ok(buf.len())
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, DeviceError> {
        // Every position the commands can address fits an `i64`, so a request that does not is
        // rejected here rather than wrapping into a position naming a different byte.
        let offset = match pos {
            SeekFrom::Start(offset) => as_position(offset)?,
            SeekFrom::Current(delta) => as_position(self.offset)?
                .checked_add(delta)
                .ok_or(DeviceError::InvalidPath)?,
            SeekFrom::End(delta) => as_position(self.size()?)?
                .checked_add(delta)
                .ok_or(DeviceError::InvalidPath)?,
        };

        if offset < 0 {
            return Err(DeviceError::InvalidPath);
        }

        // Seeking past the end is allowed and is how a sparse write is asked for; the file only
        // grows when something is written there.
        self.offset = offset as u64;
        Ok(self.offset)
    }

    fn metadata(&self) -> Result<Metadata, DeviceError> {
        Ok(Metadata::file(self.size()?))
    }

    fn set_len(&mut self, len: u64) -> Result<(), DeviceError> {
        let len = i64::try_from(len).map_err(|_| DeviceError::InvalidPath)?;
        service::with_file(self.object_id()?, |file| file.set_size(len))
    }

    fn sync(&mut self) -> Result<(), DeviceError> {
        service::with_file(self.object_id()?, |file| file.flush())
    }

    fn close(&mut self) -> Result<(), DeviceError> {
        if let Some(object_id) = self.object_id.take() {
            service::close_file(object_id);
        }
        Ok(())
    }
}

/// Returns `offset` as the signed position the commands take.
///
/// # Errors
///
/// Returns [`DeviceError::InvalidPath`] when the position is past what a command can address.
fn as_position(offset: u64) -> Result<i64, DeviceError> {
    i64::try_from(offset).map_err(|_| DeviceError::InvalidPath)
}

impl Drop for FsDeviceFile {
    fn drop(&mut self) {
        // A file dropped without being closed still holds a server-side object, which nothing else
        // will ever close. The descriptor table closes first and drops second, so this only runs
        // for a file that never reached a descriptor.
        if let Some(object_id) = self.object_id.take() {
            service::close_file(object_id);
        }
    }
}
