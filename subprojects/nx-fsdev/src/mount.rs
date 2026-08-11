//! Which filesystems are mounted, and under what names.
//!
//! Mounting a filesystem does two things: it fills a [`FsDevice`] with the object the server
//! opened, and it registers that device so a path naming it resolves here. This module keeps the
//! two in step, and holds the devices themselves.
//!
//! A device is never freed. The registry it registers with holds `&'static dyn Device`, so a
//! device that has been registered once must stay reachable forever, which makes unmounting a
//! matter of emptying it rather than dropping it. The table below therefore grows only when a name
//! is mounted that has never been mounted before: mounting `sdmc`, unmounting it and mounting it
//! again reuses the device that is already there.

use alloc::{
    boxed::Box,
    string::String,
    vec::Vec,
};
use core::ffi::c_int;

use nx_service_fs::FsFileSystem;
use nx_sf::service::DispatchError;
use nx_std_sync::mutex::Mutex;
use nx_sys_fd::{
    device::{
        Device as _,
        DeviceId,
    },
    registry,
};

use crate::{
    device::FsDevice,
    service,
};

/// Every device this process has ever mounted, mounted or not.
static DEVICES: Mutex<Vec<&'static FsDevice>> = Mutex::new(Vec::new());

/// The name the SD card is mounted under.
pub(crate) const SDMC: &str = "sdmc";

unsafe extern "C" {
    /// libsysbase's `setDefaultDevice`, naming the device a path without a prefix resolves to.
    fn setDefaultDevice(device: c_int);
}

/// Mounts `filesystem` under `name`, registering it so paths can reach it.
///
/// `name` is the bare device name, without the `":"` that follows it in a path.
///
/// Mounting is deliberately not idempotent: a name that is already mounted is refused rather than
/// replaced, which is what libnx does and what a caller checking for a name collision expects.
///
/// # Errors
///
/// Returns [`MountError::AlreadyMounted`] when the name is taken, and
/// [`MountError::RegistryFull`] when the descriptor table has no slot left. The filesystem is
/// closed either way, since nothing else holds it.
pub(crate) fn mount(name: &str, filesystem: FsFileSystem<'_>) -> Result<DeviceId, MountError> {
    let device = device_for(name);
    if device.is_mounted() {
        return Err(MountError::AlreadyMounted);
    }

    device.mount(filesystem);

    match registry::register(device) {
        Ok(id) => Ok(id),
        Err(err) => {
            device.unmount();
            Err(MountError::RegistryFull(err))
        }
    }
}

/// Errors returned by [`mount`].
#[derive(Debug, thiserror::Error)]
pub enum MountError {
    /// A filesystem is already mounted under that name
    ///
    /// Occurs when a caller mounts a name that is in use. Nothing was registered and the mount
    /// that was already there is untouched.
    #[error("A filesystem is already mounted under that name")]
    AlreadyMounted,

    /// The device registry had no slot for this device
    ///
    /// Occurs when every registry slot holds a device. The filesystem was closed and nothing was
    /// left half-mounted.
    #[error("The device registry had no slot for this device")]
    RegistryFull(#[source] registry::RegisterError),
}

/// Mounts the SD card as `sdmc:`, which is where a homebrew program's own files are.
///
/// This is the whole of libnx's `fsdevMountSdmc`: ask the session for the SD card's filesystem,
/// mount it under [`SDMC`], and claim the default device if nothing else has. It is public because
/// the runtime performs this during startup and must reach it as a Rust call rather than through
/// the `fsdev*` C name the linker aliases to this crate.
///
/// # Errors
///
/// Returns [`MountSdmcError::NoSession`] when the runtime has not installed the `fsp-srv`
/// session, [`MountSdmcError::Open`] when the server refused to open the SD card, and
/// [`MountSdmcError::Mount`] when it opened but could not be mounted.
pub fn mount_sdmc() -> Result<(), MountSdmcError> {
    let service = service::get().ok_or(MountSdmcError::NoSession)?;

    let filesystem = service
        .open_sd_card_file_system()
        .map_err(MountSdmcError::Open)?;

    let id = mount(SDMC, filesystem).map_err(MountSdmcError::Mount)?;
    set_default_device_if_first(id.index());

    Ok(())
}

/// Errors returned by [`mount_sdmc`].
#[derive(Debug, thiserror::Error)]
pub enum MountSdmcError {
    /// The `fsp-srv` session has not been installed
    ///
    /// Occurs when the SD card is mounted before the runtime has connected. Nothing was opened.
    #[error("no fsp-srv session is installed")]
    NoSession,

    /// The SD card's filesystem could not be opened
    ///
    /// Occurs when the server refused the command, which is what a console with no card inserted
    /// answers. Nothing was mounted.
    #[error("failed to open the SD card filesystem")]
    Open(#[source] DispatchError),

    /// The filesystem opened but could not be mounted
    ///
    /// Occurs when `sdmc` is already mounted or the descriptor table is full. The filesystem was
    /// closed.
    #[error("failed to mount the SD card")]
    Mount(#[source] MountError),
}

/// Unmounts whatever is mounted under `name`, closing its filesystem.
///
/// Unmounting is deliberately not idempotent: a second call reports that nothing was mounted,
/// which is the answer libnx gives and the one `fsdevUnmountDevice` turns into its `-1`.
///
/// # Errors
///
/// Returns [`NotMounted`] when nothing is mounted under that name.
pub(crate) fn unmount(name: &str) -> Result<(), NotMounted> {
    let Some(device) = find(name) else {
        return Err(NotMounted);
    };

    if let Some(id) = registry::find_by_name(name) {
        registry::unregister(id);
    }
    device.unmount();

    Ok(())
}

/// Error returned by [`unmount`] and the commands that address a mount by name.
///
/// Nothing is mounted under the name, so there was nothing to act on.
#[derive(Debug, thiserror::Error)]
#[error("No filesystem is mounted under that name")]
pub(crate) struct NotMounted;

/// Unmounts every mounted device.
///
/// This is what runs on the way out of a process, so it reports nothing: a device that fails to
/// unmount is being torn down anyway.
pub fn unmount_all() {
    let devices = DEVICES.lock().clone();
    for device in devices {
        if device.is_mounted() {
            // A device that stopped being mounted between the test and the call is one this loop
            // wanted gone anyway, and there is no caller left to report it to: this runs while the
            // process is being torn down.
            let _ = unmount(device.name());
        }
    }
}

/// Returns the device mounted under `name`.
///
/// A device that has been mounted before but is not mounted now is not a match: the table keeps
/// every device it has ever created, so being present in it says nothing about being reachable.
pub(crate) fn find(name: &str) -> Option<&'static FsDevice> {
    DEVICES
        .lock()
        .iter()
        .copied()
        .find(|device| device.name() == name && device.is_mounted())
}

/// Returns how many devices are currently mounted.
pub(crate) fn mounted_count() -> usize {
    DEVICES
        .lock()
        .iter()
        .filter(|device| device.is_mounted())
        .count()
}

/// Points the default device at `slot` when nothing else is mounted.
///
/// A path without a `"name:"` prefix resolves to the default device, which starts out as the null
/// device that discards everything. libnx claims it for the first filesystem mounted, and a
/// program that opens `"/file"` before mounting anything explicit relies on that.
pub(crate) fn set_default_device_if_first(slot: usize) {
    if mounted_count() > 1 {
        return;
    }

    // SAFETY: the slot came from the registry, which is what this entry point indexes. It is below
    // `MAX_DEVICES` and so well inside `c_int`.
    unsafe { setDefaultDevice(slot as c_int) };
}

/// Returns the device for `name`, creating it the first time that name is seen.
///
/// The device outlives the call because the registry demands it, so the allocation is deliberate
/// and made once per name.
fn device_for(name: &str) -> &'static FsDevice {
    let mut devices = DEVICES.lock();
    if let Some(device) = devices.iter().copied().find(|device| device.name() == name) {
        return device;
    }

    let name: &'static str = Box::leak(String::from(name).into_boxed_str());
    let device: &'static FsDevice = Box::leak(Box::new(FsDevice::new(name)));
    devices.push(device);

    device
}
