//! The device registry.
//!
//! Devices register once and are then reachable by index, which is how a descriptor records what
//! backs it, and by name, which is how a path such as `"sdmc:/file"` is resolved.
//!
//! Slots 0, 1 and 2 belong to stdin, stdout and stderr. Registration hands out slots from 3 upward,
//! so taking over stdout means [`bind_at`] rather than [`register`].

use core::cell::UnsafeCell;

use nx_sys_sync::Mutex;

use crate::device::{
    Device,
    DeviceError,
    DeviceId,
    MAX_DEVICES,
};

/// Registry slot for stdin.
pub const STD_IN: usize = 0;
/// Registry slot for stdout.
pub const STD_OUT: usize = 1;
/// Registry slot for stderr.
pub const STD_ERR: usize = 2;

/// First slot [`register`] will hand out.
const FIRST_DYNAMIC_DEVICE: usize = 3;

/// The process-wide device registry.
static REGISTRY: Registry = Registry {
    mutex: Mutex::new(),
    devices: UnsafeCell::new({
        // The standard slots start on the null device, so output produced before anything has
        // registered is discarded rather than dispatched through an empty slot.
        let mut devices = [None; MAX_DEVICES];
        devices[STD_IN] = Some(&NULL_DEVICE as &'static dyn Device);
        devices[STD_OUT] = Some(&NULL_DEVICE as &'static dyn Device);
        devices[STD_ERR] = Some(&NULL_DEVICE as &'static dyn Device);
        devices
    }),
};

/// The device the standard slots start on.
static NULL_DEVICE: NullDevice = NullDevice;

/// Registers `device`, returning the slot it took.
///
/// Registering a device whose name matches one already registered replaces it in place, so
/// registering twice does not exhaust the registry.
///
/// # Errors
///
/// Returns [`RegisterError`] when every slot from 3 upward is taken.
pub fn register(device: &'static dyn Device) -> Result<DeviceId, RegisterError> {
    let Some(index) = free_slot_for(device.name()) else {
        return Err(RegisterError);
    };
    bind_at(index, device);
    // SAFETY: `index` came from `free_slot_for`, which only yields slots inside the registry.
    Ok(DeviceId::from_index_unchecked(index))
}

/// Errors returned by [`register`].
///
/// The registry is full: every slot from 3 upward holds a device with a different name. Nothing
/// was registered and no slot was disturbed.
#[derive(Debug, thiserror::Error)]
#[error("The device registry has no free slot")]
pub struct RegisterError;

/// Binds `device` to `index`, replacing whatever was there.
///
/// This is how a console takes over stdout: the standard descriptors are already open against
/// slots 0, 1 and 2, so binding one redirects them without reopening anything. It is also how the
/// C boundary places an adapter at the slot the adapter itself occupies.
///
/// Binding an index outside the registry does nothing.
pub fn bind_at(index: usize, device: &'static dyn Device) {
    if index >= MAX_DEVICES {
        return;
    }

    REGISTRY.lock().devices()[index] = Some(device);
}

/// Returns the slot [`register`] would hand out for a device named `name`.
///
/// Reuses the slot of an already registered device with the same name, so registering twice
/// replaces rather than exhausting the registry.
///
/// Returns `None` when every slot from 3 upward is taken.
pub fn free_slot_for(name: &str) -> Option<usize> {
    let mut registry = REGISTRY.lock();
    registry
        .devices()
        .iter()
        .enumerate()
        .skip(FIRST_DYNAMIC_DEVICE)
        .find(|(_, slot)| match slot {
            None => true,
            Some(registered) => registered.name() == name,
        })
        .map(|(index, _)| index)
}

/// Unregisters whatever occupies `id`.
pub fn unregister(id: DeviceId) {
    let index = id.index();
    if index >= MAX_DEVICES {
        return;
    }
    REGISTRY.lock().devices()[index] = None;
}

/// Returns the device registered at `id`.
pub fn get(id: DeviceId) -> Option<&'static dyn Device> {
    let index = id.index();
    if index >= MAX_DEVICES {
        return None;
    }
    REGISTRY.lock().devices()[index]
}

/// Returns the device registered under `name`.
///
/// `name` is the bare device name, without the `":"` that follows it in a path, so the `"name:"`
/// prefix the C boundary splits off a path can be matched directly.
pub fn find_by_name(name: &str) -> Option<DeviceId> {
    let mut registry = REGISTRY.lock();
    registry
        .devices()
        .iter()
        .position(|slot| slot.is_some_and(|device| device.name() == name))
        // SAFETY: `position` indexes the registry, so the slot is in range by construction.
        .map(DeviceId::from_index_unchecked)
}

/// Accepts writes and discards them.
///
/// Stands in for a real device on the standard slots until something binds one, so that a `printf`
/// before the console is up is a no-op rather than a fault.
struct NullDevice;

impl Device for NullDevice {
    fn name(&self) -> &'static str {
        "stdnull"
    }

    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError> {
        Ok(buf.len())
    }
}

/// The registered devices.
struct Registry {
    mutex: Mutex,
    devices: UnsafeCell<[Option<&'static dyn Device>; MAX_DEVICES]>,
}

// SAFETY: every access to `devices` goes through `mutex`, and the registry is never moved.
unsafe impl Sync for Registry {}

impl Registry {
    /// Locks the registry for the lifetime of the returned guard.
    fn lock(&self) -> Locked<'_> {
        self.mutex.lock();
        Locked(self)
    }
}

/// Exclusive access to the registered devices, unlocking on drop.
struct Locked<'a>(&'a Registry);

impl Locked<'_> {
    /// Returns the registered devices.
    fn devices(&mut self) -> &mut [Option<&'static dyn Device>; MAX_DEVICES] {
        // SAFETY: holding this guard means the registry lock is held, so no other reference exists.
        unsafe { &mut *self.0.devices.get() }
    }
}

impl Drop for Locked<'_> {
    fn drop(&mut self) {
        self.0.mutex.unlock();
    }
}
