//! A mounted image, as the descriptor table sees it.
//!
//! [`RomfsDevice`] is what a path such as `"romfs:/config.json"` resolves into. It holds the image
//! it was mounted with and the working directory relative paths are joined onto, and it turns each
//! of the descriptor table's path operations into a walk over the image's tables.
//!
//! A device exists whether or not it is mounted. The registry it registers with holds
//! `&'static dyn Device`, so a device cannot be freed once registered; making it emptiable instead
//! is what lets a mount end without leaking, and what lets the same name be mounted again into the
//! device that is already there.
//!
//! ## The image is shared, the device is not held
//!
//! An open file has to keep reading after the call that opened it returned, and it must not pin the
//! device's lock to do so. So the image lives behind an [`Arc`] that an operation clones out under
//! the lock and works with after releasing it, and an open descriptor keeps a clone of its own.
//!
//! That is also what lets a program unmount an image while a file on it is still open: the device
//! empties, later lookups fail, and the descriptor already open reads on until it is closed. libnx
//! frees the tables on unmount instead, and a descriptor still open then reads freed memory.
//!
//! ## Only the working directory moves
//!
//! An image is fixed once loaded, so no lookup mutates it and any number of descriptors can walk it
//! at once. The working directory is the one piece of state that changes, which is why it sits
//! beside the image rather than inside it: changing it must not wait behind a read, and it belongs
//! to the mount rather than to the image, which is what libnx also does.

mod dir;
mod file;

use alloc::{
    boxed::Box,
    sync::Arc,
};

use nx_std_path::Path;
use nx_std_sync::mutex::Mutex;
use nx_sys_fd::device::{
    Device,
    DeviceError,
    Dir,
    File,
    Metadata,
    OpenFlags,
};

use self::{
    dir::RomfsDir,
    file::RomfsFile,
};
use crate::image::{
    Image,
    ROOT,
};

/// A romfs image reachable under one name.
pub(crate) struct RomfsDevice {
    /// The name paths address this device by, without the `":"` that follows it in a path.
    name: &'static str,
    /// The image this device is mounted on, or `None` while it is not mounted.
    image: Mutex<Option<Arc<Image>>>,
    /// Where relative paths start, as an offset into the image's directory table.
    cwd: Mutex<u32>,
}

impl RomfsDevice {
    /// Creates an unmounted device reachable by `name`.
    pub(crate) const fn new(name: &'static str) -> Self {
        Self {
            name,
            image: Mutex::new(None),
            cwd: Mutex::new(ROOT),
        }
    }

    /// Mounts `image`, resetting the working directory to the root.
    ///
    /// The device takes over the source the image was loaded from; it is released once this mount
    /// ends and every descriptor opened against it has closed.
    pub(crate) fn mount(&self, image: Image) {
        *self.image.lock() = Some(Arc::new(image));
        *self.cwd.lock() = ROOT;
    }

    /// Unmounts the device.
    ///
    /// Unmounting a device that is not mounted does nothing.
    pub(crate) fn unmount(&self) {
        let _ = self.image.lock().take();
    }

    /// Reports whether this device currently holds an image.
    pub(crate) fn is_mounted(&self) -> bool {
        self.image.lock().is_some()
    }

    /// Returns the image this device is mounted on, for use after the lock is released.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NotFound`] when the device is not mounted, which is what every
    /// operation on an unmounted device reports.
    fn mounted(&self) -> Result<Arc<Image>, DeviceError> {
        self.image
            .lock()
            .as_ref()
            .map(Arc::clone)
            .ok_or(DeviceError::NotFound)
    }

    /// Resolves `path` against this device's working directory.
    ///
    /// Returns the image the operation should read, the directory the path stopped at, and the
    /// component left over. See [`Image::walk`] for what `consume_last` decides.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::NotFound`] when the device is not mounted, and whatever
    /// [`Image::walk`] rejected the path with.
    fn locate<'p>(
        &self,
        path: &'p Path,
        consume_last: bool,
    ) -> Result<(Arc<Image>, u32, &'p [u8]), DeviceError> {
        let image = self.mounted()?;
        let cwd = *self.cwd.lock();

        let (dir, rest) = image.walk(cwd, path.as_os_str().as_bytes(), consume_last)?;
        Ok((image, dir, rest))
    }
}

impl Device for RomfsDevice {
    fn name(&self) -> &'static str {
        self.name
    }

    fn open(&self, path: &Path, flags: OpenFlags) -> Result<Box<dyn File>, DeviceError> {
        // An image is written once by whoever built it and never again, so each of these is refused
        // rather than attempted. libnx answers the same request with `EROFS`.
        if flags.write || flags.append || flags.truncate {
            return Err(DeviceError::Unsupported);
        }

        let (image, dir, name) = self.locate(path, false)?;
        // The walk consumed the whole path, so what it names is a directory rather than a file.
        if name.is_empty() {
            return Err(DeviceError::InvalidPath);
        }

        let Some(file) = image.find_file(dir, name)? else {
            // Asking to create the file is not a different kind of missing entry here. Reporting it
            // as one would suggest that a second attempt with the file in place could succeed,
            // which on a read-only image it cannot.
            return Err(if flags.create {
                DeviceError::Unsupported
            } else {
                DeviceError::NotFound
            });
        };

        // The caller asked for a file that was not to exist, and it does.
        if flags.create && flags.exclusive {
            return Err(DeviceError::AlreadyExists);
        }

        let (offset, size) = image.contents_of(file)?;
        Ok(Box::new(RomfsFile::new(image, offset, size)))
    }

    fn open_dir(&self, path: &Path) -> Result<Box<dyn Dir>, DeviceError> {
        let (image, dir, _) = self.locate(path, true)?;
        Ok(Box::new(RomfsDir::new(image, dir)?))
    }

    fn metadata(&self, path: &Path) -> Result<Metadata, DeviceError> {
        let (image, dir, name) = self.locate(path, false)?;

        // The walk consumed the whole path, so the directory it stopped at is what was named.
        if name.is_empty() {
            return Ok(Metadata::dir());
        }

        if image.find_dir(dir, name)?.is_some() {
            return Ok(Metadata::dir());
        }

        let Some(file) = image.find_file(dir, name)? else {
            return Err(DeviceError::NotFound);
        };
        Ok(Metadata::file(image.file_size_at(file)?))
    }

    fn set_current_dir(&self, path: &Path) -> Result<(), DeviceError> {
        let (_, dir, _) = self.locate(path, true)?;

        *self.cwd.lock() = dir;
        Ok(())
    }
}
