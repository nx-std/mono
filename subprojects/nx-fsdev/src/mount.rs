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
    ffi::CString,
    vec::Vec,
};
use core::ffi::CStr;

use nx_service_fs::FsFileSystem;
use nx_std_sync::mutex::Mutex;
use nx_sys_fd::{
    device::{
        Device as _,
        DeviceId,
    },
    registry,
};

use crate::device::FsDevice;

/// Every device this process has ever mounted, mounted or not.
static DEVICES: Mutex<Vec<&'static FsDevice>> = Mutex::new(Vec::new());

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
pub fn mount(name: &CStr, filesystem: FsFileSystem<'_>) -> Result<DeviceId, MountError> {
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

/// Unmounts whatever is mounted under `name`, closing its filesystem.
///
/// Unmounting is deliberately not idempotent: a second call reports that nothing was mounted,
/// which is the answer libnx gives and the one `fsdevUnmountDevice` turns into its `-1`.
///
/// # Errors
///
/// Returns [`NotMounted`] when nothing is mounted under that name.
pub fn unmount(name: &CStr) -> Result<(), NotMounted> {
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
pub struct NotMounted;

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
pub fn find(name: &CStr) -> Option<&'static FsDevice> {
    find_by_bytes(name.to_bytes())
}

/// Returns the device whose name is `name`, which carries no trailing nul.
///
/// The C boundary splits the `"name:"` prefix off a path, which leaves a slice of the path rather
/// than a string of its own.
pub fn find_by_bytes(name: &[u8]) -> Option<&'static FsDevice> {
    DEVICES
        .lock()
        .iter()
        .copied()
        .find(|device| device.name().to_bytes() == name && device.is_mounted())
}

/// Returns how many devices are currently mounted.
pub fn mounted_count() -> usize {
    DEVICES
        .lock()
        .iter()
        .filter(|device| device.is_mounted())
        .count()
}

/// Returns the device for `name`, creating it the first time that name is seen.
///
/// The device outlives the call because the registry demands it, so the allocation is deliberate
/// and made once per name.
fn device_for(name: &CStr) -> &'static FsDevice {
    let mut devices = DEVICES.lock();
    if let Some(device) = devices.iter().copied().find(|device| device.name() == name) {
        return device;
    }

    let name: &'static CStr = Box::leak(CString::from(name).into_boxed_c_str());
    let device: &'static FsDevice = Box::leak(Box::new(FsDevice::new(name)));
    devices.push(device);

    device
}
