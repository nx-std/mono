//! Which images are mounted, and under what names.
//!
//! Mounting does two things: it loads an image into a [`RomfsDevice`], and it registers that device
//! so a path naming it resolves here. This module keeps the two in step, and holds the devices
//! themselves.
//!
//! A device is never freed. The registry it registers with holds `&'static dyn Device`, so a device
//! that has been registered once must stay reachable forever, which makes unmounting a matter of
//! emptying it rather than dropping it. The table below therefore grows only when a name is mounted
//! that has never been mounted before.
//!
//! ## One entry point per source
//!
//! The functions below differ in where the image comes from, and there is deliberately no function
//! that decides. libnx has one, `romfsMountSelf`, which asks whether the process is an `NSO` or an
//! `NRO` and picks; that question is answered by which runtime crate a binary links, above this
//! one, so the branch belongs there and each entry crate calls the function that is right for it.

use alloc::{
    boxed::Box,
    string::String,
    vec::Vec,
};

use nx_service_fs::{
    FsFile,
    FsService,
    FsStorage,
    NcmStorageId,
};
use nx_sf::service::DispatchError;
use nx_std_path::Path;
use nx_std_sync::mutex::Mutex;
use nx_sys_fd::{
    device::{
        Device as _,
        DeviceError,
        File,
        OpenFlags,
    },
    registry,
};

pub use crate::image::LoadError;
use crate::{
    device::RomfsDevice,
    image::Image,
    source::Source,
};

/// Every device this process has ever mounted, mounted or not.
static DEVICES: Mutex<Vec<&'static RomfsDevice>> = Mutex::new(Vec::new());

/// The name a program's own image is conventionally mounted under.
pub const SELF: &str = "romfs";

/// Mounts the image `offset` bytes into `file`, under `name`.
///
/// The file is one some mounted device opened, so this works over anything the descriptor table can
/// reach rather than only over a filesystem. The mount takes over closing it.
///
/// `name` is the bare device name, without the `":"` that follows it in a path.
///
/// # Errors
///
/// Returns [`MountError::AlreadyMounted`] when the name is taken, [`MountError::Image`] when the
/// bytes are not a romfs image this crate can read, and [`MountError::Registry`] when the
/// descriptor table has no slot left.
pub fn from_device_file(name: &str, file: Box<dyn File>, offset: u64) -> Result<(), MountError> {
    mount(name, Source::from_device_file(file, offset))
}

/// Mounts the image `offset` bytes into `file`, under `name`.
///
/// The file is a server-side object the caller opened through `fsp-srv` directly, rather than one
/// reached through a mounted device. The mount takes over closing it.
///
/// # Errors
///
/// The same as [`from_device_file`].
pub fn from_fs_file(name: &str, file: FsFile<'_>, offset: u64) -> Result<(), MountError> {
    // SAFETY: the wrapper released its close obligation here, and the source is what honours it
    // from now on.
    let source = Source::from_raw_file_object_id_unchecked(file.into_raw_object_id(), offset);
    mount(name, source)
}

/// Mounts the image `offset` bytes into `storage`, under `name`.
///
/// The mount takes over closing the storage.
///
/// # Errors
///
/// The same as [`from_device_file`].
pub fn from_storage(name: &str, storage: FsStorage<'_>, offset: u64) -> Result<(), MountError> {
    // SAFETY: the wrapper released its close obligation here, and the source is what honours it
    // from now on.
    let source = Source::from_raw_storage_object_id_unchecked(storage.into_raw_object_id(), offset);
    mount(name, source)
}

/// Errors returned by the mounts that take a source they were handed.
#[derive(Debug, thiserror::Error)]
pub enum MountError {
    /// An image is already mounted under that name
    ///
    /// Occurs when a caller mounts a name that is in use. Nothing was registered and the mount that
    /// was already there is untouched.
    #[error("An image is already mounted under that name")]
    AlreadyMounted,

    /// The bytes are not an image this crate can read
    ///
    /// Occurs when the header is not a romfs header, or describes tables that cannot be loaded.
    /// Nothing was registered and the source was released.
    #[error("The image could not be read")]
    Image(#[source] LoadError),

    /// The device registry had no slot for this device
    ///
    /// Occurs when every registry slot holds a device. The image was released and nothing was left
    /// half-mounted.
    #[error("The device registry had no slot for this device")]
    Registry(#[source] registry::RegisterError),
}

/// Mounts the image `offset` bytes into the file at `path`, under `name`.
///
/// `path` names a file on some already-mounted device, prefix and all: `"sdmc:/game.nro"`. Which
/// device that is, and what kind of storage sits behind it, is the descriptor table's business and
/// not this crate's.
///
/// This is the route a homebrew `NRO` takes to its own image, and the reason it is public rather
/// than only reachable through the C surface: the runtime crate that knows an `NRO` keeps its image
/// this way calls it directly.
///
/// # Errors
///
/// Returns [`PathMountError::NoDevice`] when no mounted device serves the path,
/// [`PathMountError::Open`] when the file could not be opened, and [`PathMountError::Mount`] when
/// it opened but the image could not be mounted.
pub fn from_device_path(name: &str, path: &Path, offset: u64) -> Result<(), PathMountError> {
    let id = nx_sys_fd::path::device_for_path(path).ok_or(PathMountError::NoDevice)?;
    let device = registry::get(id).ok_or(PathMountError::NoDevice)?;

    let file = device
        .open(nx_sys_fd::path::strip_device_prefix(path), READ_ONLY)
        .map_err(PathMountError::Open)?;

    from_device_file(name, file, offset).map_err(PathMountError::Mount)
}

/// How an image's own container is opened: for reading, and nothing else.
const READ_ONLY: OpenFlags = OpenFlags {
    read: true,
    write: false,
    append: false,
    create: false,
    exclusive: false,
    truncate: false,
};

/// Errors returned by [`from_device_path`].
#[derive(Debug, thiserror::Error)]
pub enum PathMountError {
    /// No mounted device serves that path
    ///
    /// Occurs when the path's prefix names nothing, or names a device that has since been
    /// unmounted. Nothing was opened.
    #[error("No mounted device serves that path")]
    NoDevice,

    /// The file could not be opened
    ///
    /// Occurs when the path names nothing on the device serving it, or the device refused to open
    /// it. Nothing was mounted.
    #[error("The file holding the image could not be opened")]
    Open(#[source] DeviceError),

    /// The file opened but the image could not be mounted
    ///
    /// Occurs when the name is taken, the bytes are not an image, or the descriptor table is full.
    /// The file was closed.
    #[error("failed to mount the image")]
    Mount(#[source] MountError),
}

/// Mounts the running program's own data partition under `name`.
///
/// This is how a packaged program reaches its image: the partition is a storage object the server
/// opens for whoever asks, so nothing has to say which program is meant.
///
/// # Errors
///
/// Returns [`OpenError::NoSession`] when the runtime has not installed the `fsp-srv` session,
/// [`OpenError::Open`] when the server refused to open the partition, and [`OpenError::Mount`] when
/// it opened but could not be mounted.
pub fn from_current_process(name: &str) -> Result<(), OpenError> {
    open_and_mount(name, |service| {
        service.open_data_storage_by_current_process()
    })
}

/// Mounts the data partition of the program `program_id` names, under `name`.
///
/// Reaching another program's data needs the permission for it in the process's own descriptor;
/// without it the server refuses.
///
/// # Errors
///
/// The same as [`from_current_process`].
pub fn from_program(name: &str, program_id: u64) -> Result<(), OpenError> {
    open_and_mount(name, |service| {
        service.open_data_storage_by_program_id(program_id)
    })
}

/// Mounts the system data archive `data_id` names on `storage_id`, under `name`.
///
/// # Errors
///
/// The same as [`from_current_process`].
pub fn from_data_archive(
    name: &str,
    data_id: u64,
    storage_id: NcmStorageId,
) -> Result<(), OpenError> {
    open_and_mount(name, |service| {
        service.open_data_storage_by_data_id(data_id, storage_id)
    })
}

/// Errors returned by the mounts that open a storage object first.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The `fsp-srv` session has not been installed
    ///
    /// Occurs when an image is mounted before the runtime has connected. Nothing was opened.
    #[error("no fsp-srv session is installed")]
    NoSession,

    /// The storage could not be opened
    ///
    /// Occurs when the server refused the command, which is what a program with no data partition,
    /// or without the permission to reach another program's, is answered with. Nothing was mounted.
    #[error("failed to open the storage holding the image")]
    Open(#[source] DispatchError),

    /// The storage opened but could not be mounted
    ///
    /// Occurs when the name is taken, the bytes are not an image, or the descriptor table is full.
    /// The storage was closed.
    #[error("failed to mount the image")]
    Mount(#[source] MountError),
}

/// Unmounts whatever is mounted under `name`.
///
/// Unmounting is deliberately not idempotent: a second call reports that nothing was mounted, which
/// is the answer libnx gives.
///
/// A descriptor still open on the image keeps reading it until it is closed. libnx frees the tables
/// here instead, and a descriptor open across the call then reads memory that has been handed back.
///
/// # Errors
///
/// Returns [`NotMounted`] when nothing is mounted under that name.
pub fn unmount(name: &str) -> Result<(), NotMounted> {
    let Some(device) = find(name) else {
        return Err(NotMounted);
    };

    if let Some(id) = registry::find_by_name(name) {
        registry::unregister(id);
    }
    device.unmount();

    Ok(())
}

/// Error returned by [`unmount`].
///
/// Nothing is mounted under the name, so there was nothing to act on.
#[derive(Debug, thiserror::Error)]
#[error("No image is mounted under that name")]
pub struct NotMounted;

/// Returns the device mounted under `name`.
///
/// A device that has been mounted before but is not mounted now is not a match: the table keeps
/// every device it has ever created, so being present in it says nothing about being reachable.
pub(crate) fn find(name: &str) -> Option<&'static RomfsDevice> {
    DEVICES
        .lock()
        .iter()
        .copied()
        .find(|device| device.name() == name && device.is_mounted())
}

/// Opens a storage object through the installed session and mounts the image in it.
///
/// The three storage-backed mounts differ only in which command opens the object, so the session
/// lookup, the mount and the error shape live here once.
fn open_and_mount(
    name: &str,
    open: impl for<'svc> FnOnce(&'svc FsService) -> Result<FsStorage<'svc>, DispatchError>,
) -> Result<(), OpenError> {
    let Some(service) = nx_fsdev::service::get() else {
        return Err(OpenError::NoSession);
    };

    let storage = open(&service).map_err(OpenError::Open)?;

    // A data partition is the whole of its storage, so the image starts at the beginning.
    from_storage(name, storage, 0).map_err(OpenError::Mount)
}

/// Loads the image in `source` and registers it under `name`.
///
/// Mounting is deliberately not idempotent: a name that is already mounted is refused rather than
/// replaced, which is what a caller checking for a name collision expects.
///
/// This is where the two devices in libnx disagree with each other, and where this crate follows
/// the filesystem one. libnx's `fsdevMountDevice` refuses a name that is taken; its romfs mount
/// never looks, and takes a fresh slot whose registration then *replaces* the first device rather
/// than being rejected, because that is what `AddDevice` does with a name it already holds. The
/// first mount is left registered nowhere, holding a file nothing will close, and every descriptor
/// already open on it now reads through a device the table no longer points at.
fn mount(name: &str, source: Source) -> Result<(), MountError> {
    let device = device_for(name);
    if device.is_mounted() {
        return Err(MountError::AlreadyMounted);
    }

    // The image is loaded before the device is touched, so a source that turns out not to hold one
    // is dropped here and leaves no half-filled device behind.
    let image = Image::load(source).map_err(MountError::Image)?;
    device.mount(image);

    match registry::register(device) {
        Ok(_) => Ok(()),
        Err(err) => {
            device.unmount();
            Err(MountError::Registry(err))
        }
    }
}

/// Returns the device for `name`, creating it the first time that name is seen.
///
/// The device outlives the call because the registry demands it, so the allocation is deliberate
/// and made once per name.
fn device_for(name: &str) -> &'static RomfsDevice {
    let mut devices = DEVICES.lock();
    if let Some(device) = devices.iter().copied().find(|device| device.name() == name) {
        return device;
    }

    let name: &'static str = Box::leak(String::from(name).into_boxed_str());
    let device: &'static RomfsDevice = Box::leak(Box::new(RomfsDevice::new(name)));
    devices.push(device);

    device
}
