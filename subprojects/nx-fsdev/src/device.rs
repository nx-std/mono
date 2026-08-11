//! A mounted filesystem, as the descriptor table sees it.
//!
//! [`FsDevice`] is what a path such as `"sdmc:/save.dat"` resolves into. It holds the id of the
//! `IFileSystem` it was mounted with and the working directory relative paths are joined onto, and
//! it turns each of the descriptor table's operations into one `fsp-srv` command.
//!
//! A device exists whether or not it is mounted. The registry it registers with holds
//! `&'static dyn Device`, so a device cannot be freed once registered; making it emptiable instead
//! is what lets a mount end without leaking, and what lets the same name be mounted again into the
//! device that is already there.
//!
//! Nothing here holds the session. Every operation borrows it for the length of one command
//! through [`crate::service`], which is also where the wrapper for an object id is rebuilt.

use alloc::{
    boxed::Box,
    vec::Vec,
};

use nx_service_fs::{
    CreateOption,
    DirEntryType,
    DirOpenMode,
    FsDir,
    FsFile,
    FsFileSystem,
    OpenMode,
};
use nx_std_path::{
    OsString,
    Path,
    PathBuf,
};
use nx_std_sync::mutex::Mutex;
use nx_sys_fd::device::{
    Device,
    DeviceError,
    Dir,
    File,
    Metadata,
    OpenFlags,
    SpaceInfo,
    Timestamps,
};

mod dir;
mod file;

use self::{
    dir::FsDeviceDir,
    file::FsDeviceFile,
};
use crate::{
    path::FsPath,
    service,
};

/// A filesystem reachable under one name.
pub struct FsDevice {
    /// The name paths address this device by, without the `":"` that follows it in a path.
    name: &'static str,
    /// What this device is mounted on, or `None` while it is not mounted.
    state: Mutex<Option<Mounted>>,
}

/// What a device holds while it is mounted.
struct Mounted {
    /// Domain object id of the `IFileSystem` every command is addressed to.
    filesystem: u32,
    /// Working directory relative paths are joined onto: absolute, without a trailing slash.
    cwd: PathBuf,
}

impl FsDevice {
    /// Creates an unmounted device reachable by `name`.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            state: Mutex::new(None),
        }
    }

    /// Mounts `filesystem`, resetting the working directory to the root.
    ///
    /// The device takes over closing the filesystem: the wrapper's close obligation is released
    /// here and honoured by [`FsDevice::unmount`].
    pub fn mount(&self, filesystem: FsFileSystem<'_>) {
        let mut state = self.state.lock();
        if let Some(previous) = state.take() {
            service::close_filesystem(previous.filesystem);
        }

        *state = Some(Mounted {
            filesystem: filesystem.into_raw_object_id(),
            cwd: PathBuf::from("/"),
        });
    }

    /// Unmounts the device, closing the filesystem it held.
    ///
    /// Unmounting a device that is not mounted does nothing.
    pub fn unmount(&self) {
        let taken = self.state.lock().take();
        if let Some(mounted) = taken {
            service::close_filesystem(mounted.filesystem);
        }
    }

    /// Reports whether this device currently holds a filesystem.
    pub fn is_mounted(&self) -> bool {
        self.state.lock().is_some()
    }

    /// Returns the id of the filesystem this device is mounted on.
    ///
    /// Crate-internal: the id is what the C surface needs to describe the mount to a caller, and
    /// handing it further would put a second closer within reach of the one this device owes.
    pub(crate) fn filesystem(&self) -> Option<u32> {
        self.state.lock().as_ref().map(|mounted| mounted.filesystem)
    }

    /// Resolves `path` against this device's working directory.
    ///
    /// Returns the filesystem the command should address and the buffer to send. This is the first
    /// step of every operation below, and the only one that touches the device's own state: the
    /// lock is released before any command is dispatched, so a walk that blocks on the server does
    /// not hold up a second descriptor on the same device.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NotFound`] when the device is not mounted, and whatever
    /// [`FsPath::create`] rejected the path with.
    pub fn locate(&self, path: &Path) -> Result<(u32, FsPath), DeviceError> {
        let state = self.state.lock();
        let Some(mounted) = state.as_ref() else {
            return Err(DeviceError::NotFound);
        };

        let resolved = FsPath::create(&mounted.cwd, path)?;
        Ok((mounted.filesystem, resolved))
    }

    /// Commits every write made through this device to the underlying storage.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NotFound`] when the device is not mounted, or [`DeviceError::Io`]
    /// when the commit failed.
    pub fn commit(&self) -> Result<(), DeviceError> {
        let Some(filesystem) = self.filesystem() else {
            return Err(DeviceError::NotFound);
        };

        service::with_filesystem(filesystem, |fs| fs.commit())
    }

    /// Creates a file of `size` bytes at `path`, with the concatenation option `option` asks for.
    ///
    /// This is the sized create the C surface exposes directly; [`Device::open`] creates an empty
    /// file instead.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::AlreadyExists`] when the path is taken, and [`DeviceError::Io`] when
    /// the creation failed.
    pub fn create_file(
        &self,
        path: &Path,
        size: i64,
        option: CreateOption,
    ) -> Result<(), DeviceError> {
        let (filesystem, path) = self.locate(path)?;
        service::with_filesystem(filesystem, |fs| fs.create_file(path.as_buf(), size, option))
    }

    /// Removes the directory at `path` and everything under it.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NotFound`] when the path names nothing, and [`DeviceError::Io`] when
    /// the removal failed.
    pub fn remove_dir_all(&self, path: &Path) -> Result<(), DeviceError> {
        let (filesystem, path) = self.locate(path)?;
        service::with_filesystem(filesystem, |fs| {
            fs.delete_directory_recursively(path.as_buf())
        })
    }
}

impl Device for FsDevice {
    fn name(&self) -> &'static str {
        self.name
    }

    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Box<dyn File>, DeviceError> {
        // Appending to a descriptor that cannot be written is the one flag combination the C
        // standard library forwards without rejecting first.
        if flags.append && !flags.write {
            return Err(DeviceError::InvalidPath);
        }

        let (filesystem, path) = self.locate(path)?;

        // Why the create failed, kept until the open below says whether it mattered.
        let mut create_failure = None;
        if flags.create {
            let created = service::with_filesystem(filesystem, |fs| {
                fs.create_file(path.as_buf(), 0, CreateOption::empty())
            });

            match created {
                // With O_EXCL the caller asked for the entry not to exist, so a create that failed
                // is the answer and there is nothing left to try.
                Err(err) if flags.exclusive => return Err(err),
                // Without it, an entry that is already there is exactly what the caller asked to
                // open, so the failure is only interesting if the open then fails too.
                Err(err) => create_failure = Some(err),
                Ok(()) => {}
            }
        }

        let mut mode = OpenMode::empty();
        if flags.read {
            mode |= OpenMode::READ;
        }
        if flags.write {
            // `APPEND` here is the server's permission to extend the file, which every writable
            // descriptor needs; positioning writes at the end is this crate's own bookkeeping.
            mode |= OpenMode::WRITE | OpenMode::APPEND;
        }

        // The wrapper the command produces borrows the service, which is only borrowed for the
        // length of the command, so what leaves the closure is the id rather than the wrapper.
        let opened = service::with_filesystem(filesystem, |fs| {
            fs.open_file(path.as_buf(), mode)
                .map(FsFile::into_raw_object_id)
        });

        let object_id = match opened {
            Ok(object_id) => object_id,
            // The open failed on a path this call was asked to create, so what went wrong is the
            // create, and reporting the open instead would blame the path for not existing rather
            // than say why it could not be made.
            Err(err) => return Err(create_failure.unwrap_or(err)),
        };

        if flags.write
            && flags.truncate
            && let Err(err) = service::with_file(object_id, |file| file.set_size(0))
        {
            service::close_file(object_id);
            return Err(err);
        }

        // SAFETY: the open above issued `object_id` and released its close obligation, so this file
        // is the only thing that will close it.
        Ok(Box::new(FsDeviceFile::from_raw_object_id_unchecked(
            object_id, flags,
        )))
    }

    fn open_dir(&self, path: &Path) -> Result<Box<dyn Dir>, DeviceError> {
        let (filesystem, path) = self.locate(path)?;

        let object_id = service::with_filesystem(filesystem, |fs| {
            fs.open_directory(
                path.as_buf(),
                DirOpenMode::READ_DIRS | DirOpenMode::READ_FILES,
            )
            .map(FsDir::into_raw_object_id)
        })?;

        // SAFETY: as in `open` above.
        Ok(Box::new(FsDeviceDir::from_raw_object_id_unchecked(
            object_id,
        )))
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, DeviceError> {
        let (filesystem, path) = self.locate(path)?;

        let entry_type =
            service::with_filesystem(filesystem, |fs| fs.get_entry_type(path.as_buf()))?;
        if entry_type == DirEntryType::Dir {
            return Ok(Metadata::dir());
        }

        // Horizon reports a file's size through the file itself, so the only way to answer is to
        // open it. It is opened read-only and closed before returning, so a caller asking about a
        // path holds nothing afterwards.
        let object_id = service::with_filesystem(filesystem, |fs| {
            fs.open_file(path.as_buf(), OpenMode::READ)
                .map(FsFile::into_raw_object_id)
        })?;
        let size = service::with_file(object_id, |file| file.get_size());
        service::close_file(object_id);

        let size = size?;
        // Clamped, so the cast is from a non-negative `i64`.
        let metadata = Metadata::file(size.max(0) as u64);

        // The timestamps are advisory: a filesystem that does not keep them fails this command,
        // and a caller gets the entry without them rather than nothing at all.
        //
        // TODO: Re-interpret the stamps through the time service's timezone rule, as libnx does.
        //  The filesystem keeps them in its own local time, so the raw values reported here are
        //  off by whatever the console's timezone is.
        let Ok(stamps) =
            service::with_filesystem(filesystem, |fs| fs.get_file_time_stamp_raw(path.as_buf()))
        else {
            return Ok(metadata);
        };
        if stamps.is_valid == 0 {
            return Ok(metadata);
        }

        Ok(metadata.with_timestamps(Timestamps {
            created: stamps.created,
            modified: stamps.modified,
            accessed: stamps.accessed,
        }))
    }

    fn remove_file(&self, path: &Path) -> Result<(), DeviceError> {
        let (filesystem, path) = self.locate(path)?;
        service::with_filesystem(filesystem, |fs| fs.delete_file(path.as_buf()))
    }

    fn create_dir(&self, path: &Path) -> Result<(), DeviceError> {
        let (filesystem, path) = self.locate(path)?;
        service::with_filesystem(filesystem, |fs| fs.create_directory(path.as_buf()))
    }

    fn remove_dir(&self, path: &Path) -> Result<(), DeviceError> {
        let (filesystem, path) = self.locate(path)?;
        service::with_filesystem(filesystem, |fs| fs.delete_directory(path.as_buf()))
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), DeviceError> {
        let (filesystem, from) = self.locate(from)?;
        let (_, to) = self.locate(to)?;

        // Horizon renames a file and a directory through different commands, so what is being
        // moved has to be established first.
        let entry_type =
            service::with_filesystem(filesystem, |fs| fs.get_entry_type(from.as_buf()))?;
        service::with_filesystem(filesystem, |fs| match entry_type {
            DirEntryType::Dir => fs.rename_directory(from.as_buf(), to.as_buf()),
            DirEntryType::File => fs.rename_file(from.as_buf(), to.as_buf()),
        })
    }

    fn set_current_dir(&self, path: &Path) -> Result<(), DeviceError> {
        let (filesystem, resolved) = self.locate(path)?;

        // A working directory that names a file, or nothing at all, would fail every relative path
        // built from it afterwards with an error naming the wrong path. It is checked here so the
        // failure names the directory the caller actually asked for.
        let entry_type =
            service::with_filesystem(filesystem, |fs| fs.get_entry_type(resolved.as_buf()))?;
        if entry_type != DirEntryType::Dir {
            return Err(DeviceError::InvalidPath);
        }

        let mut cwd = Vec::from(resolved.as_bytes());
        // The root is the one directory that keeps its slash, because dropping it would leave an
        // empty prefix that every relative path would be joined onto.
        while cwd.len() > 1 && cwd.ends_with(b"/") {
            cwd.pop();
        }
        let cwd = PathBuf::from(OsString::from(cwd));

        let mut state = self.state.lock();
        let Some(mounted) = state.as_mut() else {
            // The device was unmounted while the directory was being checked, so there is no
            // working directory left to move. Reporting it beats writing into a mount that is gone.
            return Err(DeviceError::NotFound);
        };
        mounted.cwd = cwd;

        Ok(())
    }

    fn space_info(&self, path: &Path) -> Result<SpaceInfo, DeviceError> {
        let (filesystem, path) = self.locate(path)?;

        let free = service::with_filesystem(filesystem, |fs| fs.get_free_space(path.as_buf()))?;
        let total = service::with_filesystem(filesystem, |fs| fs.get_total_space(path.as_buf()))?;

        // Horizon reports bytes rather than blocks, so the block is one byte and the counts are
        // the byte counts unchanged. libnx reports the same figures the same way.
        // Both figures are clamped, so each cast is from a non-negative `i64`.
        Ok(SpaceInfo {
            block_size: 1,
            total_blocks: total.max(0) as u64,
            free_blocks: free.max(0) as u64,
        })
    }
}
