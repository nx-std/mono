//! The descriptor table.
//!
//! One slot per descriptor number, in static storage. A descriptor always names a device, and may
//! additionally own the [`File`] that device produced for it. What the two cases mean, and why an
//! open file is held behind a lock of its own rather than under the table lock, is explained in
//! [`entry`](self::entry).
//!
//! Every operation here follows the same shape: take the table lock, resolve the descriptor to what
//! backs it, release the table lock, and only then call the device or the file. Nothing that can
//! block runs with the table locked.
//!
//! ## Why the storage is two arrays
//!
//! What a descriptor holds is one idea, and a sum type would say so. The table nonetheless stores
//! it as two arrays side by side, and the reason is that the standard descriptors have to be open
//! before any code runs.
//!
//! Filling their slots means writing them in the static initializer, and a value that owns
//! something cannot be assigned there: the assignment would drop whatever the slot held, and a
//! destructor cannot run at compile time. Splitting the device number, which owns nothing, from the
//! open file, which does, lets the first array be written in the initializer while the second stays
//! uniformly empty.
//!
//! The pairing is an invariant this module keeps rather than one the types enforce: a descriptor is
//! open exactly when its device entry is set, and a file entry is meaningful only alongside one.
//! Every access goes through the accessors below, which is what holds the two in step.

mod entry;
mod fd;

use alloc::{
    boxed::Box,
    sync::Arc,
};
use core::cell::UnsafeCell;

use nx_sys_sync::Mutex;

use self::entry::OpenFile;
pub use self::fd::{
    Fd,
    InvalidFd,
    MAX_FD,
};
use crate::{
    device::{
        DeviceError,
        DeviceId,
        File,
        Metadata,
        SeekFrom,
    },
    registry,
};

/// The process-wide descriptor table.
static TABLE: Table = Table {
    mutex: Mutex::new(),
    devices: UnsafeCell::new({
        // Descriptors 0, 1 and 2 are open before anything asks, so that early output has somewhere
        // to go. They start on the matching standard device slots, owning no file of their own.
        let mut devices = [None; MAX_FD];
        // SAFETY: the standard slots are registry constants, so they are in range by
        // construction.
        devices[0] = Some(DeviceId::from_index_unchecked(registry::STD_IN));
        devices[1] = Some(DeviceId::from_index_unchecked(registry::STD_OUT));
        devices[2] = Some(DeviceId::from_index_unchecked(registry::STD_ERR));
        devices
    }),
    files: UnsafeCell::new([const { None }; MAX_FD]),
};

/// Binds the lowest free descriptor number to `device`.
///
/// The descriptor starts owning no file, which is the right state for a stream and the starting
/// state for a path: the C standard library allocates the descriptor first and calls the device's
/// open second, so [`attach`] completes it afterwards.
///
/// # Errors
///
/// Returns [`OpenError::NoDevice`] when `device` names no registered device, and
/// [`OpenError::NoDescriptors`] when every slot is in use.
pub fn open(device: DeviceId) -> Result<Fd, OpenError> {
    if registry::get(device).is_none() {
        return Err(OpenError::NoDevice);
    }

    let mut table = TABLE.lock();
    let devices = table.devices();

    let Some(number) = devices.iter().position(Option::is_none) else {
        return Err(OpenError::NoDescriptors);
    };
    devices[number] = Some(device);

    // SAFETY: `number` indexes `devices`, so it is below `MAX_FD` by construction.
    Ok(Fd::from_number_unchecked(number))
}

/// Errors returned by [`open`].
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The device is not registered
    ///
    /// Occurs when the registry slot named is empty, either because nothing registered there or
    /// because the device was unregistered. No descriptor was taken.
    #[error("No device is registered at that slot")]
    NoDevice,

    /// Every descriptor is in use
    ///
    /// Occurs once the table is full. Nothing was allocated and no slot was disturbed, so the call
    /// is safe to retry after a descriptor is closed.
    #[error("No free descriptors remain")]
    NoDescriptors,
}

/// Gives `fd` the file that will serve it from now on.
///
/// This is the second half of opening a path: the descriptor already exists, and this is what the
/// device's open produced for it.
///
/// # Errors
///
/// Returns [`AttachError::BadDescriptor`] when `fd` is not open, and
/// [`AttachError::AlreadyAttached`] when it already owns a file. The new file is dropped in either
/// case, with the table unlocked.
pub fn attach(fd: Fd, file: Box<dyn File>) -> Result<(), AttachError> {
    let number = fd.number();
    if number >= MAX_FD {
        return Err(AttachError::BadDescriptor);
    }

    let open_file = Arc::new(OpenFile::new(file));
    let mut rejected = None;

    // The file is moved into the slot only on the accepting path. On either rejection it stays
    // owned by this function and is dropped on the way out, with the table lock already gone.
    {
        let mut table = TABLE.lock();
        if table.devices()[number].is_none() {
            rejected = Some(AttachError::BadDescriptor);
        } else if table.files()[number].is_some() {
            rejected = Some(AttachError::AlreadyAttached);
        } else {
            table.files()[number] = Some(open_file);
        }
    }

    match rejected {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// Errors returned by [`attach`].
#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    /// The descriptor is not open
    ///
    /// Occurs when the descriptor was released between being allocated and the device's open
    /// finishing.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The descriptor already owns a file
    ///
    /// Occurs when a device's open ran twice for one descriptor, which the C standard library does
    /// not do. The descriptor keeps the file it had.
    #[error("Descriptor already holds an open file")]
    AlreadyAttached,
}

/// Releases `fd` and closes whatever it held.
///
/// The slot is freed before the file is told, so the close runs with the table unlocked and the
/// descriptor number already reusable. A file that blocks in close therefore cannot hold up the
/// table.
///
/// A stream descriptor owns nothing, so closing one releases nothing and always succeeds.
///
/// # Errors
///
/// Returns [`CloseError::BadDescriptor`] when `fd` is not open, or [`CloseError::File`] when the
/// file reported a failure. The descriptor is released either way.
pub fn close(fd: Fd) -> Result<(), CloseError> {
    let Some((_, file)) = take_entry(fd) else {
        return Err(CloseError::BadDescriptor);
    };

    match file {
        None => Ok(()),
        Some(file) => file.lock().file().close().map_err(CloseError::File),
    }
}

/// Errors returned by [`close`].
#[derive(Debug, thiserror::Error)]
pub enum CloseError {
    /// The descriptor is not open
    ///
    /// Occurs when the number was never opened, or was closed already. Nothing was released.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The file failed to release what it held
    ///
    /// The descriptor number is free regardless, so this reports what the file could not finish
    /// rather than a reason to retry the close.
    #[error("File failed to close")]
    File(#[source] DeviceError),
}

/// Writes `buf` to whatever backs `fd`, returning how many bytes it consumed.
///
/// # Errors
///
/// Returns [`WriteError::BadDescriptor`] when `fd` is not open, [`WriteError::NoDevice`] when a
/// stream descriptor's device is no longer registered, or [`WriteError::Device`] with whatever the
/// device or file reported.
pub fn write(fd: Fd, buf: &[u8]) -> Result<usize, WriteError> {
    match target_of(fd) {
        None => Err(WriteError::BadDescriptor),
        Some(Target::File(file)) => file.lock().file().write(buf).map_err(WriteError::Device),
        Some(Target::Stream(device)) => match registry::get(device) {
            None => Err(WriteError::NoDevice),
            Some(registered) => registered.write(buf).map_err(WriteError::Device),
        },
    }
}

/// Errors returned by [`write`].
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The descriptor is not open
    ///
    /// Nothing was written.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The device backing the descriptor is no longer registered
    ///
    /// Occurs when a device is unregistered while stream descriptors on it are still open. Nothing
    /// was written.
    #[error("No device is registered for that descriptor")]
    NoDevice,

    /// The device could not take the bytes
    #[error("Device failed to write")]
    Device(#[source] DeviceError),
}

/// Reads from whatever backs `fd` into `buf`, returning how many bytes it produced.
///
/// # Errors
///
/// Returns [`ReadError::BadDescriptor`] when `fd` is not open, [`ReadError::NoDevice`] when a
/// stream descriptor's device is no longer registered, or [`ReadError::Device`] with whatever the
/// device or file reported.
pub fn read(fd: Fd, buf: &mut [u8]) -> Result<usize, ReadError> {
    match target_of(fd) {
        None => Err(ReadError::BadDescriptor),
        Some(Target::File(file)) => file.lock().file().read(buf).map_err(ReadError::Device),
        Some(Target::Stream(device)) => match registry::get(device) {
            None => Err(ReadError::NoDevice),
            Some(registered) => registered.read(buf).map_err(ReadError::Device),
        },
    }
}

/// Errors returned by [`read`].
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The descriptor is not open
    ///
    /// Nothing was read.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The device backing the descriptor is no longer registered
    ///
    /// Occurs when a device is unregistered while stream descriptors on it are still open. Nothing
    /// was read.
    #[error("No device is registered for that descriptor")]
    NoDevice,

    /// The device could not produce bytes
    #[error("Device failed to read")]
    Device(#[source] DeviceError),
}

/// Moves the position of the file behind `fd`, returning where it ended up.
///
/// # Errors
///
/// Returns [`SeekError::BadDescriptor`] when `fd` is not open, [`SeekError::NotAFile`] when it is a
/// stream, which has no position, or [`SeekError::File`] with whatever the file reported.
pub fn seek(fd: Fd, pos: SeekFrom) -> Result<u64, SeekError> {
    match target_of(fd) {
        None => Err(SeekError::BadDescriptor),
        Some(Target::Stream(_)) => Err(SeekError::NotAFile),
        Some(Target::File(file)) => file.lock().file().seek(pos).map_err(SeekError::File),
    }
}

/// Errors returned by [`seek`].
#[derive(Debug, thiserror::Error)]
pub enum SeekError {
    /// The descriptor is not open
    ///
    /// The position is unchanged.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The descriptor owns no file
    ///
    /// Occurs on a stream descriptor, which reaches its device directly and has no position to
    /// move.
    #[error("Descriptor does not hold a file")]
    NotAFile,

    /// The file could not move its position
    #[error("File failed to seek")]
    File(#[source] DeviceError),
}

/// Reports what the file behind `fd` is and how large it is.
///
/// # Errors
///
/// Returns [`MetadataError::BadDescriptor`] when `fd` is not open, [`MetadataError::NotAFile`] when
/// it is a stream, or [`MetadataError::File`] with whatever the file reported.
pub fn metadata(fd: Fd) -> Result<Metadata, MetadataError> {
    match target_of(fd) {
        None => Err(MetadataError::BadDescriptor),
        Some(Target::Stream(_)) => Err(MetadataError::NotAFile),
        Some(Target::File(file)) => file.lock().file().metadata().map_err(MetadataError::File),
    }
}

/// Errors returned by [`metadata`].
#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    /// The descriptor is not open
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The descriptor owns no file
    ///
    /// Occurs on a stream descriptor, which has no entry to report on.
    #[error("Descriptor does not hold a file")]
    NotAFile,

    /// The file could not report on itself
    #[error("File failed to report metadata")]
    File(#[source] DeviceError),
}

/// Resizes the file behind `fd` to `len` bytes.
///
/// # Errors
///
/// Returns [`SetLenError::BadDescriptor`] when `fd` is not open, [`SetLenError::NotAFile`] when it
/// is a stream, or [`SetLenError::File`] with whatever the file reported.
pub fn set_len(fd: Fd, len: u64) -> Result<(), SetLenError> {
    match target_of(fd) {
        None => Err(SetLenError::BadDescriptor),
        Some(Target::Stream(_)) => Err(SetLenError::NotAFile),
        Some(Target::File(file)) => file.lock().file().set_len(len).map_err(SetLenError::File),
    }
}

/// Errors returned by [`set_len`].
#[derive(Debug, thiserror::Error)]
pub enum SetLenError {
    /// The descriptor is not open
    ///
    /// Nothing was resized.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The descriptor owns no file
    ///
    /// Occurs on a stream descriptor, which has no length to set.
    #[error("Descriptor does not hold a file")]
    NotAFile,

    /// The file could not be resized
    #[error("File failed to resize")]
    File(#[source] DeviceError),
}

/// Commits what has been written to the file behind `fd`.
///
/// # Errors
///
/// Returns [`SyncError::BadDescriptor`] when `fd` is not open, [`SyncError::NotAFile`] when it is a
/// stream, or [`SyncError::File`] with whatever the file reported.
pub fn sync(fd: Fd) -> Result<(), SyncError> {
    match target_of(fd) {
        None => Err(SyncError::BadDescriptor),
        Some(Target::Stream(_)) => Err(SyncError::NotAFile),
        Some(Target::File(file)) => file.lock().file().sync().map_err(SyncError::File),
    }
}

/// Errors returned by [`sync`].
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// The descriptor is not open
    ///
    /// Nothing was committed.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The descriptor owns no file
    ///
    /// Occurs on a stream descriptor, which has nothing of its own to commit.
    #[error("Descriptor does not hold a file")]
    NotAFile,

    /// The file could not be committed
    #[error("File failed to sync")]
    File(#[source] DeviceError),
}

/// Runs `op` against the file behind `fd`, returning what it produced.
///
/// This is how a device reaches state only it knows its files hold. The operations above are the
/// ones every device performs the same way; a socket layer's descriptor is not one of them, and the
/// C functions that need it (`send`, `bind`, `listen`, …) are not device operations at all. `op`
/// receives the file as a trait object and downcasts it to the type the device produced, which is
/// what [`File`] requiring [`Any`](core::any::Any) is for.
///
/// The file's lock is held for as long as `op` runs, so `op` copies out what it needs and returns.
/// Doing the work inside it would serialize against every other operation on the same descriptor,
/// which for a socket means a receive blocking a send on the same connection.
///
/// # Errors
///
/// Returns [`WithFileError::BadDescriptor`] when `fd` is not open, and [`WithFileError::NotAFile`]
/// when it is a stream descriptor, which owns no file to reach.
pub fn with_file<T>(fd: Fd, op: impl FnOnce(&mut dyn File) -> T) -> Result<T, WithFileError> {
    match target_of(fd) {
        None => Err(WithFileError::BadDescriptor),
        Some(Target::Stream(_)) => Err(WithFileError::NotAFile),
        Some(Target::File(file)) => {
            let mut guard = file.lock();
            Ok(op(guard.file()))
        }
    }
}

/// Errors returned by [`with_file`].
#[derive(Debug, thiserror::Error)]
pub enum WithFileError {
    /// The descriptor is not open
    ///
    /// `op` did not run.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The descriptor owns no file
    ///
    /// Occurs on a stream descriptor, which reaches its device directly. `op` did not run.
    #[error("Descriptor does not hold a file")]
    NotAFile,
}

/// Returns the device backing `fd`.
pub fn device_of(fd: Fd) -> Option<DeviceId> {
    let number = fd.number();
    if number >= MAX_FD {
        return None;
    }

    TABLE.lock().devices()[number]
}

/// Frees `fd`, returning the device it named.
///
/// Whatever the descriptor owned is dropped here, with the table already unlocked.
#[cfg(feature = "ffi")]
pub(crate) fn take(fd: Fd) -> Option<DeviceId> {
    take_entry(fd).map(|(device, _)| device)
}

/// What an operation on a descriptor should call into.
enum Target {
    /// The descriptor names a device directly.
    Stream(DeviceId),
    /// The descriptor owns an open file.
    File(Arc<OpenFile>),
}

/// Resolves `fd` to what backs it, releasing the table lock before the caller uses it.
///
/// An open file is handed back as a new handle rather than a reference, so the caller can operate
/// on it long after the table lock is gone and a concurrent close cannot drop it mid-operation.
fn target_of(fd: Fd) -> Option<Target> {
    let number = fd.number();
    if number >= MAX_FD {
        return None;
    }

    let mut table = TABLE.lock();
    let device = table.devices()[number]?;

    match table.files()[number].as_ref() {
        Some(file) => Some(Target::File(Arc::clone(file))),
        None => Some(Target::Stream(device)),
    }
}

/// Empties the slot `fd` names, returning the device it named and the file it owned.
///
/// Both travel out of the lock, so an open file is dropped by the caller with the table unlocked.
fn take_entry(fd: Fd) -> Option<(DeviceId, Option<Arc<OpenFile>>)> {
    let number = fd.number();
    if number >= MAX_FD {
        return None;
    }

    let mut table = TABLE.lock();
    let device = table.devices()[number].take()?;
    let file = table.files()[number].take();

    Some((device, file))
}

/// The descriptor table.
struct Table {
    mutex: Mutex,
    devices: UnsafeCell<[Option<DeviceId>; MAX_FD]>,
    files: UnsafeCell<[Option<Arc<OpenFile>>; MAX_FD]>,
}

// SAFETY: every access to `devices` and `files` goes through `mutex`, and the table is never moved.
unsafe impl Sync for Table {}

impl Table {
    /// Locks the table for the lifetime of the returned guard.
    fn lock(&self) -> Locked<'_> {
        self.mutex.lock();
        Locked(self)
    }
}

/// Exclusive access to the table's slots, unlocking on drop.
struct Locked<'a>(&'a Table);

impl Locked<'_> {
    /// Returns the device each descriptor names, where a set entry means the descriptor is open.
    fn devices(&mut self) -> &mut [Option<DeviceId>; MAX_FD] {
        // SAFETY: holding this guard means the table lock is held, so no other reference exists.
        unsafe { &mut *self.0.devices.get() }
    }

    /// Returns the file each descriptor owns, which is meaningful only where a device is set.
    fn files(&mut self) -> &mut [Option<Arc<OpenFile>>; MAX_FD] {
        // SAFETY: holding this guard means the table lock is held, so no other reference exists.
        unsafe { &mut *self.0.files.get() }
    }
}

impl Drop for Locked<'_> {
    fn drop(&mut self) {
        self.0.mutex.unlock();
    }
}
