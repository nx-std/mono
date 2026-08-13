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

/// Orders access to the default device, and holds it.
///
/// The lock and what it guards are one static rather than two, because a program that links this
/// crate twice must share both or neither: a borrowed slot guarded by a private lock is a slot two
/// libraries write without ordering. Keeping them together makes that unrepresentable.
///
/// The symbol is spelled out, and `extern-state` swaps this definition for a declaration. See
/// [rust-process-wide-state](../../../docs/code/rust-process-wide-state.md).
#[cfg(not(feature = "extern-state"))]
#[unsafe(no_mangle)]
static DEFAULT_DEVICE: DefaultDevice = DefaultDevice {
    lock: Mutex::new(),
    slot: UnsafeCell::new(None),
};

#[cfg(feature = "extern-state")]
unsafe extern "Rust" {
    /// The default device and its lock, owned by another static library.
    static DEFAULT_DEVICE: DefaultDevice;
}

/// The one default-device slot, however this build reaches it.
fn default_device() -> &'static DefaultDevice {
    #[cfg(not(feature = "extern-state"))]
    {
        &DEFAULT_DEVICE
    }

    #[cfg(feature = "extern-state")]
    // SAFETY: the symbol is defined by the one static library built without `extern-state`, as a
    // `DefaultDevice` from this same source at this same version, so the reference has the type and
    // layout it claims. It is a `static`, so the `'static` lifetime is honest. The lock inside
    // orders access to the slot; a shared reference to the pair races with nothing.
    unsafe {
        &DEFAULT_DEVICE
    }
}

/// Resolves a path to the device that should serve it.
///
/// A `"name:"` prefix names the device; a path without one goes to the default device.
pub fn device_for_path(path: &Path) -> Option<DeviceId> {
    let Some(end) = prefix_end(path) else {
        // SAFETY: the default device slot was bounds-checked when it was set.
        return default_slot().map(DeviceId::from_index_unchecked);
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

    let device = default_device();
    device.lock.lock();
    // SAFETY: the lock is held.
    unsafe { *device.slot.get() = Some(slot) };
    device.lock.unlock();
}

/// Returns the default device slot.
fn default_slot() -> Option<usize> {
    let device = default_device();
    device.lock.lock();
    // SAFETY: the lock is held.
    let slot = unsafe { *device.slot.get() };
    device.lock.unlock();
    slot
}

/// Returns where the device name ends, or `None` when the path carries no `"name:"` prefix.
fn prefix_end(path: &Path) -> Option<usize> {
    path.as_os_str()
        .as_bytes()
        .iter()
        .position(|&byte| byte == b':')
}

/// The default device slot, with the lock that orders access to it.
struct DefaultDevice {
    /// Orders access to `slot`.
    lock: Mutex,
    /// The device paths without a `"name:"` prefix resolve to.
    slot: UnsafeCell<Option<usize>>,
}

// SAFETY: only touched while `LOCK` is held.
unsafe impl Sync for DefaultDevice {}
