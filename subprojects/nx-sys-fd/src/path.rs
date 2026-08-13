//! Resolving a path to the device that serves it.
//!
//! A device is addressed by writing its name into the path: `"sdmc:/save.dat"` names the device
//! registered as `sdmc`, and `"/save.dat"` goes to whichever device was made the default. Splitting
//! that apart is the first thing every path-taking operation does.
//!
//! The convention comes from the C standard library, which is how a device is addressed there, but
//! the naming is the platform's rather than C's: a caller reaching the descriptor table from Rust
//! addresses a device the same way and needs the same split. So this sits beside the table rather
//! than inside the C surface, where all three of its callers can reach it — the entry points, which
//! resolve a path before dispatching; the shims, which resolve it again because an operation taking
//! a path is handed no other clue about which device it is being called for; and a Rust caller
//! opening a path of its own.

use core::cell::UnsafeCell;

use nx_std_path::{
    OsStr,
    Path,
};
use nx_sys_sync::Mutex;

use crate::{
    device::{
        DeviceId,
        MAX_DEVICES,
    },
    registry,
};

/// Orders access to the default device.
static LOCK: Mutex = Mutex::new();

/// The device paths without a `"name:"` prefix resolve to.
static DEFAULT_DEVICE: DefaultDevice = DefaultDevice(UnsafeCell::new(None));

/// Resolves a path to the device that should serve it.
///
/// A `"name:"` prefix names the device; a path without one goes to the default device.
pub fn device_for_path(path: &Path) -> Option<DeviceId> {
    let Some(end) = prefix_end(path) else {
        // SAFETY: the default device slot was bounds-checked when it was set.
        return default_device().map(DeviceId::from_index_unchecked);
    };

    // A device registers under a name that is text, so a prefix that is not UTF-8 matches nothing
    // and resolves to no device. Reading it as text here costs the same answer and keeps the
    // registry from having to compare raw bytes.
    let name = core::str::from_utf8(&path.as_os_str().as_bytes()[..end]).ok()?;
    registry::find_by_name(name)
}

/// Returns `path` with any `"name:"` prefix removed.
///
/// What remains is the device's own business, and it is what a [`crate::device::Device`] is handed.
pub fn strip_device_prefix(path: &Path) -> &Path {
    let Some(end) = prefix_end(path) else {
        return path;
    };

    Path::new(OsStr::from_bytes(&path.as_os_str().as_bytes()[end + 1..]))
}

/// Sets the default device slot.
pub fn set_default_device(slot: usize) {
    if slot >= MAX_DEVICES {
        return;
    }

    LOCK.lock();
    // SAFETY: the lock is held.
    unsafe { *DEFAULT_DEVICE.0.get() = Some(slot) };
    LOCK.unlock();
}

/// Returns the default device slot.
fn default_device() -> Option<usize> {
    LOCK.lock();
    // SAFETY: the lock is held.
    let slot = unsafe { *DEFAULT_DEVICE.0.get() };
    LOCK.unlock();
    slot
}

/// Returns where the device name ends, or `None` when the path carries no `"name:"` prefix.
fn prefix_end(path: &Path) -> Option<usize> {
    path.as_os_str()
        .as_bytes()
        .iter()
        .position(|&byte| byte == b':')
}

/// Storage for the default device slot.
struct DefaultDevice(UnsafeCell<Option<usize>>);

// SAFETY: only touched while `LOCK` is held.
unsafe impl Sync for DefaultDevice {}
