//! # nx-std-fs
//!
//! Files and directories, addressed by path.
//!
//! This crate is the front door [`nx_sys_fd`] has no opinion about. The descriptor table below it
//! already defines what a filesystem can do: [`nx_sys_fd::device::Device`] declares `open`,
//! `rename`, `remove_file` and the rest, and [`nx_sys_fd::device::File`] declares the operations on
//! an open one. What it does not define is how a caller *reaches* one of those from a path, because
//! the table's own callers arrive holding a descriptor somebody else opened.
//!
//! That gap is this crate. It resolves `"sdmc:/switch/a.nro"` to the device registered as `sdmc`,
//! hands the rest of the path to it, and wraps what comes back in a type that closes on drop.
//!
//! ## Where this sits
//!
//! It backs `std::fs`, the way [`nx_std_path`] backs `std::path` and `nx-std-sync` backs
//! `std::sync`. The layer beneath it, [`nx_sys_fd`], is the platform layer `std::sys` names; the
//! split between the two is the same one `std` draws, and the names follow it.
//!
//! The C standard library reaches the same devices by a different road: newlib calls `libsysbase`,
//! which looks a descriptor up in the table and dispatches into the device. Neither road goes
//! through the other. What they share is the device and the path convention, which is why the
//! resolution both need sits in [`nx_sys_fd::path`] rather than in either caller.
//!
//! ## What is not here
//!
//! Nothing that needs a working directory of its own. A relative path is resolved by the device
//! that serves it, against the working directory that device holds, which is the arrangement the C
//! library already established: `chdir("sdmc:/a")` moves the SD card's directory and leaves every
//! other mount where it was.
//!
//! ## no-std
//!
//! The crate is `#![no_std]` and uses `alloc` for the boxed file a device hands back; the umbrella
//! `nx-std` crate owns the single `#[global_allocator]`.
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// A device hands back its open file as a `Box<dyn File>`.
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc as _;

use alloc::boxed::Box;

use nx_std_path::Path;
// The types this crate's signatures are written in terms of. Re-exported under their own names, so
// that naming one costs a caller neither a dependency on the layer below nor a second name for a
// type that already has one.
pub use nx_sys_fd::device::{
    DeviceError,
    FileType,
    Metadata,
    SeekFrom,
};
use nx_sys_fd::{
    device::{
        Device,
        OpenFlags,
    },
    path,
    registry,
};

/// An open file, closed when this value is dropped.
///
/// Wraps the file a device handed back. The device's own type closes the server-side object it
/// names when it is dropped, so this holds nothing beyond the box and adds no destructor of its
/// own.
pub struct File(Box<dyn nx_sys_fd::device::File>);

impl File {
    /// Opens a file for reading.
    ///
    /// # Errors
    ///
    /// [`Error::NoDevice`] when no device serves `path`, and [`Error::Device`] when the device
    /// refused, which for a read-only open means the file does not exist.
    pub fn open(path: &Path) -> Result<Self, Error> {
        OpenOptions::new().read(true).open(path)
    }

    /// Opens a file for writing, creating it and discarding anything already there.
    ///
    /// # Errors
    ///
    /// As [`Self::open`]; the device refuses when the path names a directory or cannot be created.
    pub fn create(path: &Path) -> Result<Self, Error> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
    }

    /// Reads into `buf` from the current position, reporting how much arrived.
    ///
    /// A count of zero means the end of the file.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] when the device refused the read.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.0.read(buf).map_err(Error::Device)
    }

    /// Writes `buf` at the current position, reporting how much the device took.
    ///
    /// A short count is an ordinary outcome; [`Self::write_all`] is the form that finishes the job.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] when the device refused the write, which is how a full card reports
    /// itself.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.0.write(buf).map_err(Error::Device)
    }

    /// Writes every byte of `buf`, however many writes that takes.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] as [`Self::write`], and [`Error::WriteZero`] when the device stops taking
    /// bytes without refusing. Some of `buf` may already be on the card either way.
    pub fn write_all(&mut self, buf: &[u8]) -> Result<(), Error> {
        let mut left = buf;

        while !left.is_empty() {
            match self.write(left)? {
                // The device took nothing and reported no failure, so writing again would loop
                // forever on the same bytes.
                0 => return Err(Error::WriteZero),
                taken => left = &left[taken..],
            }
        }

        Ok(())
    }

    /// Moves the read and write position, reporting where it landed.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] when the device refused, which includes a position it cannot address.
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        self.0.seek(pos).map_err(Error::Device)
    }

    /// Resizes the file, padding with zeroes when it grows.
    ///
    /// Taking the room up front is what makes a card that has none fail here rather than part-way
    /// through a long write.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] when the device refused, which for a growth means the card had no room.
    pub fn set_len(&mut self, len: u64) -> Result<(), Error> {
        self.0.set_len(len).map_err(Error::Device)
    }

    /// Reports what the filesystem knows about the open file.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] when the device refused.
    pub fn metadata(&self) -> Result<Metadata, Error> {
        self.0.metadata().map_err(Error::Device)
    }

    /// Puts everything written so far where a later open would find it.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] when the device could not place what it was holding, which is how a card
    /// that filled up part-way through reports itself.
    pub fn sync(&mut self) -> Result<(), Error> {
        self.0.sync().map_err(Error::Device)
    }

    /// Closes the file, reporting what the device said.
    ///
    /// Dropping does the same thing and discards the answer, which is the right behaviour for a
    /// file going out of scope. This is for the caller that is owed a verdict, and a caller writing
    /// something it intends to launch is owed one: the last of a write is not on the card until the
    /// close says so.
    ///
    /// # Errors
    ///
    /// [`Error::Device`] when the device could not place the last of what it held. The file is
    /// closed either way.
    pub fn close(mut self) -> Result<(), Error> {
        self.0.close().map_err(Error::Device)
    }
}

/// How a file is to be opened.
///
/// Mirrors `std::fs::OpenOptions`: each method records one intention and [`Self::open`] acts on all
/// of them together, so the combinations a device rejects are rejected once, by the device.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
}

impl OpenOptions {
    /// A set of options that asks for nothing.
    ///
    /// Opening on it fails: a caller has to say whether it intends to read or to write.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the file may be read.
    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Sets whether the file may be written.
    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Sets whether every write goes to the end, whatever the position.
    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Sets whether the file is created when it does not exist.
    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Sets whether the open fails when the file already exists.
    ///
    /// Implies [`Self::create`], as `std` does: the pair asks for a file that did not exist a
    /// moment ago, and creating is half of that.
    pub fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    /// Sets whether the existing contents are discarded on open.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Opens `path` with these options.
    ///
    /// # Errors
    ///
    /// [`Error::NoDevice`] when no device serves `path`, and [`Error::Device`] when the device
    /// refused the combination or the file.
    pub fn open(self, path: &Path) -> Result<File, Error> {
        let (device, relative) = resolve(path)?;

        let flags = OpenFlags {
            read: self.read,
            write: self.write,
            append: self.append,
            create: self.create || self.create_new,
            exclusive: self.create_new,
            truncate: self.truncate,
        };

        device
            .open(relative, flags)
            .map(File)
            .map_err(Error::Device)
    }
}

/// Creates a directory.
///
/// Creates the last component only; a parent that does not exist is a failure rather than something
/// this creates on the way.
///
/// # Errors
///
/// [`Error::NoDevice`] when no device serves `path`, and [`Error::Device`] when the device refused,
/// which includes the directory already existing.
pub fn create_dir(path: &Path) -> Result<(), Error> {
    let (device, relative) = resolve(path)?;
    device.create_dir(relative).map_err(Error::Device)
}

/// Removes a file.
///
/// # Errors
///
/// As [`create_dir`]; the device refuses when the path names no file, or names a directory.
pub fn remove_file(path: &Path) -> Result<(), Error> {
    let (device, relative) = resolve(path)?;
    device.remove_file(relative).map_err(Error::Device)
}

/// Removes an empty directory.
///
/// # Errors
///
/// As [`create_dir`]; the device refuses when the directory is not empty.
pub fn remove_dir(path: &Path) -> Result<(), Error> {
    let (device, relative) = resolve(path)?;
    device.remove_dir(relative).map_err(Error::Device)
}

/// Renames an entry, replacing nothing.
///
/// # Errors
///
/// [`Error::NoDevice`] when no device serves either path, [`Error::CrossDevice`] when they are
/// served by different ones, and [`Error::Device`] when the device refused — which it does when
/// `to` already exists, since this replaces nothing.
pub fn rename(from: &Path, to: &Path) -> Result<(), Error> {
    let (from_device, from_relative) = resolve(from)?;
    let (to_device, to_relative) = resolve(to)?;

    // A rename is one filesystem's operation on its own entries. Two devices have no shared
    // filesystem to perform it on, and moving the bytes across instead would be a copy wearing a
    // rename's name: it would take time proportional to the file and could half-finish.
    if !core::ptr::eq(from_device, to_device) {
        return Err(Error::CrossDevice);
    }

    from_device
        .rename(from_relative, to_relative)
        .map_err(Error::Device)
}

/// Reports what the filesystem knows about an entry.
///
/// # Errors
///
/// As [`create_dir`]; the device refuses when the path names nothing.
pub fn metadata(path: &Path) -> Result<Metadata, Error> {
    let (device, relative) = resolve(path)?;
    device.metadata(relative).map_err(Error::Device)
}

/// Errors returned by this crate's operations.
///
/// One type across the surface rather than one per operation, which is what `std` does with
/// `io::Error`: every operation resolves a path the same way and then asks a device, so each fails
/// the same two ways, and the remaining variants belong to the one operation that can produce them.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No device serves the path
    ///
    /// Occurs when the `"name:"` prefix names nothing that is mounted, and when the path carries no
    /// prefix and no default device has been set. Nothing was asked of any filesystem.
    #[error("No device serves the path")]
    NoDevice,

    /// The device refused the operation
    ///
    /// Carries the condition the device reported, which is where the distinction between a missing
    /// file, a full card and a path that will not fit lives.
    #[error("The device refused the operation")]
    Device(#[source] DeviceError),

    /// The two paths are served by different devices
    ///
    /// Occurs only for [`rename`]. Nothing was moved, and both entries are where they were.
    #[error("The paths are served by different devices")]
    CrossDevice,

    /// The device stopped taking bytes without refusing
    ///
    /// Occurs only for [`File::write_all`], when a write reports that it took nothing. Some of the
    /// buffer may already be on the card; how much is what the preceding writes returned.
    #[error("The device accepted no more bytes")]
    WriteZero,
}

/// Resolves a path to the device that serves it and the path that device is handed.
///
/// Shared by every operation here, which all begin this way.
fn resolve(path: &Path) -> Result<(&'static dyn Device, &Path), Error> {
    let Some(id) = path::device_for_path(path) else {
        return Err(Error::NoDevice);
    };
    let Some(device) = registry::get(id) else {
        return Err(Error::NoDevice);
    };

    Ok((device, path::strip_device_prefix(path)))
}
