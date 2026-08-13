//! Where a mounted image's bytes come from.
//!
//! Three backings, because an image reaches a program three ways. A homebrew `NRO` has one appended
//! to itself, so the image is part of a file sitting on some other mounted device. A packaged
//! program has one as its data partition, which `fsp-srv` hands out as a storage object. And a
//! caller may have opened the file itself and hand over the server-side object rather than a path.
//!
//! Everything above this module addresses the image from its own start, and never learns which of
//! the three it got nor how far into the container the image begins.
//!
//! ## Why a failed read is retried
//!
//! Both backings end in an IPC command that maps the caller's buffer for the server to write into,
//! and the kernel refuses to map a buffer whose memory is in the wrong state. Which buffers those
//! are is not something a caller can test for: the buffer belongs to whoever called `fread`, and it
//! may sit anywhere.
//!
//! So a failed read is tried once more in chunks through a buffer this module owns, and the bytes
//! are copied out. libnx does the same, keyed on the exact result code the kernel returns; the code
//! does not survive the descriptor table's error type, so the retry here is keyed on the failure
//! instead. A read that failed for any other reason fails the second time too, and costs one extra
//! command on a path that was already returning an error.

use alloc::boxed::Box;

use nx_service_fs::{
    FsFile,
    FsStorage,
    ReadOption,
};
use nx_sf::service::DispatchError;
use nx_std_sync::mutex::Mutex;
use nx_sys_fd::device::{
    DeviceError,
    File,
    SeekFrom,
};

/// How much of a retried read is moved at a time.
const BOUNCE_LEN: usize = 0x1000;

/// The bytes one mounted image is read out of.
pub(crate) struct Source {
    /// What holds the image.
    backing: Backing,
    /// How far into the backing the image starts.
    ///
    /// An `NRO` carries its image after its own code and assets, so this is rarely zero for a file
    /// and always zero for a data partition.
    base: u64,
}

/// What holds an image.
enum Backing {
    /// A file on another mounted device.
    ///
    /// The lock is what makes a shared device able to read: the file has a position of its own and
    /// its operations need `&mut`, while every descriptor open on the image reads through the one
    /// source concurrently.
    DeviceFile(Mutex<Box<dyn File>>),

    /// A file object in the `fsp-srv` session's domain, addressed by its id.
    ///
    /// The id rather than a wrapper, because a wrapper borrows the session and a source that lives
    /// as long as a mount cannot hold a borrow of something behind a lock. Each read rebuilds the
    /// wrapper and hands the close obligation straight back, as the filesystem device does for the
    /// objects it names.
    FsFile(u32),

    /// A storage object in the `fsp-srv` session's domain. See [`Backing::FsFile`].
    Storage(u32),
}

impl Source {
    /// Reads an image starting `base` bytes into `file`, which some mounted device opened.
    ///
    /// The source takes over closing the file.
    pub(crate) fn from_device_file(file: Box<dyn File>, base: u64) -> Self {
        Self {
            backing: Backing::DeviceFile(Mutex::new(file)),
            base,
        }
    }

    /// Reads an image starting `base` bytes into the file `object_id` names.
    ///
    /// The caller must hold the close obligation for `object_id`: this source takes it over and
    /// closes it when the source is dropped. Handing the same id here twice would close it twice.
    pub(crate) fn from_raw_file_object_id_unchecked(object_id: u32, base: u64) -> Self {
        Self {
            backing: Backing::FsFile(object_id),
            base,
        }
    }

    /// Reads an image starting `base` bytes into the storage `object_id` names.
    ///
    /// Carries the same obligation as [`Source::from_raw_file_object_id_unchecked`].
    pub(crate) fn from_raw_storage_object_id_unchecked(object_id: u32, base: u64) -> Self {
        Self {
            backing: Backing::Storage(object_id),
            base,
        }
    }

    /// Fills `buf` from `offset` bytes into the image.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Io`] when the bytes could not be produced, including when the
    /// container ends before `buf` is full. A short read is a failure here rather than a count,
    /// because every caller in this crate has already clamped its request to what the image says is
    /// there.
    pub(crate) fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        if buf.is_empty() {
            return Ok(());
        }

        let position = self.base.checked_add(offset).ok_or(DeviceError::Io)?;

        match self.read_direct(position, buf) {
            Ok(()) => Ok(()),
            Err(_) => self.read_bounced(position, buf),
        }
    }

    /// Fills `buf` in one read, straight into the caller's buffer.
    ///
    /// `position` is measured from the start of the backing, not the start of the image.
    fn read_direct(&self, position: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        match &self.backing {
            Backing::DeviceFile(file) => {
                let mut file = file.lock();
                file.seek(SeekFrom::Start(position))?;

                let mut filled = 0;
                while filled < buf.len() {
                    let read = file.read(&mut buf[filled..])?;
                    if read == 0 {
                        return Err(DeviceError::Io);
                    }
                    filled += read;
                }
                Ok(())
            }
            Backing::FsFile(object_id) => {
                let position = as_position(position)?;
                let read = with_session(|service| {
                    // SAFETY: `object_id` was issued by the server inside this session's domain, and
                    // only an explicit close ends it. The obligation is handed straight back below.
                    let object = FsFile::from_raw_object_id_unchecked(service, *object_id);
                    let result = object.read(position, buf, buf.len() as u64, ReadOption::NONE);
                    let _ = object.into_raw_object_id();
                    result
                })?;

                // Unlike a storage read, this one reports how much it moved, and a file that ends
                // before the image said it would is a container that does not hold what it claims.
                if read < buf.len() as u64 {
                    return Err(DeviceError::Io);
                }
                Ok(())
            }
            Backing::Storage(object_id) => {
                let position = as_position(position)?;
                // The command either delivers every byte asked for or fails, so there is no short
                // read to loop over.
                with_session(|service| {
                    // SAFETY: as for the file above.
                    let object = FsStorage::from_raw_object_id_unchecked(service, *object_id);
                    let result = object.read(position, buf, buf.len() as u64);
                    let _ = object.into_raw_object_id();
                    result
                })
            }
        }
    }

    /// Fills `buf` in chunks through a buffer this module owns.
    fn read_bounced(&self, position: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        let mut bounce = [0u8; BOUNCE_LEN];

        for (chunk_index, chunk) in buf.chunks_mut(BOUNCE_LEN).enumerate() {
            let staging = &mut bounce[..chunk.len()];
            // Every chunk but the last is exactly `BOUNCE_LEN`, so where this one starts is its
            // index times that.
            let chunk_position = position + (chunk_index * BOUNCE_LEN) as u64;
            self.read_direct(chunk_position, staging)?;
            chunk.copy_from_slice(staging);
        }

        Ok(())
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        // A device file closes itself when the box drops; a server-side object has to be told.
        // Dropping the rebuilt wrapper is what sends the close, so these are the two places an id
        // is not handed back. A session that is already gone took the object with it.
        let _ = with_session(|service| {
            match self.backing {
                // SAFETY: the id was issued by the server inside this session's domain, this source
                // held the only close obligation for it, and the source is being dropped, so it is
                // closed exactly once.
                Backing::FsFile(object_id) => {
                    drop(FsFile::from_raw_object_id_unchecked(service, object_id))
                }
                // SAFETY: as for the file above.
                Backing::Storage(object_id) => {
                    drop(FsStorage::from_raw_object_id_unchecked(service, object_id))
                }
                Backing::DeviceFile(_) => {}
            }
            Ok(())
        });
    }
}

/// Runs `f` against the installed `fsp-srv` session.
///
/// Every way a read can fail arrives as [`DeviceError::Io`], and that is not a mapping waiting to
/// be filled in: a read-only device reports nothing else. The codes a read produces are "past the
/// end of the container" and "that buffer cannot be mapped", and both mean the same thing to a
/// caller here, which is that the bytes the image promised did not arrive. The path errors a C
/// caller branches on (`ENOENT` and the like) are decided in [`crate::image`] out of the tables,
/// before any command is built.
///
/// # Errors
///
/// Returns [`DeviceError::Io`] when no session is installed or the command failed.
fn with_session<R>(
    f: impl FnOnce(&nx_service_fs::FsService) -> Result<R, DispatchError>,
) -> Result<R, DeviceError> {
    let Some(service) = nx_fsdev::service::get() else {
        return Err(DeviceError::Io);
    };

    f(&service).map_err(|_| DeviceError::Io)
}

/// Returns `position` as the signed offset the storage commands take.
///
/// # Errors
///
/// Returns [`DeviceError::Io`] when the position is past what a command can address, which means
/// the image claimed data further out than the container can hold.
fn as_position(position: u64) -> Result<i64, DeviceError> {
    i64::try_from(position).map_err(|_| DeviceError::Io)
}
