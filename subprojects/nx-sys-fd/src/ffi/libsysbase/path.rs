//! Resolving a path to the device that serves it.
//!
//! The C standard library addresses a device by writing its name into the path: `"sdmc:/save.dat"`
//! names the device registered as `sdmc`, and `"/save.dat"` goes to whichever device was made the
//! default. Splitting that apart is the first thing every path-taking entry point does.
//!
//! It lives in its own module because both callers need it and neither can own it: the entry points
//! resolve a path before dispatching, and the shims resolve it again because the operations that
//! take a path are handed no other clue about which device they are being called for.

use core::{
    cell::UnsafeCell,
    ffi::CStr,
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
pub fn device_for_path(path: &CStr) -> Option<DeviceId> {
    match prefix_end(path) {
        Some(end) => registry::find_by_name_bytes(&path.to_bytes()[..end]),
        // SAFETY: the default device slot was bounds-checked when it was set.
        None => default_device().map(DeviceId::from_index_unchecked),
    }
}

/// Returns `path` with any `"name:"` prefix removed.
///
/// What remains is the device's own business, and it is what a [`crate::device::Device`] is handed.
/// The result borrows from `path`: stripping a prefix only moves the start, so the original nul
/// still terminates it.
pub fn strip_device_prefix(path: &CStr) -> &CStr {
    let Some(end) = prefix_end(path) else {
        return path;
    };

    // SAFETY: `end` indexes the colon inside `path`, so advancing past it stays within the same
    // allocation and lands on or before the original nul terminator, which still terminates the
    // remainder.
    unsafe { CStr::from_ptr(path.as_ptr().add(end + 1)) }
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
fn prefix_end(path: &CStr) -> Option<usize> {
    path.to_bytes().iter().position(|&byte| byte == b':')
}

/// Storage for the default device slot.
struct DefaultDevice(UnsafeCell<Option<usize>>);

// SAFETY: only touched while `LOCK` is held.
unsafe impl Sync for DefaultDevice {}
