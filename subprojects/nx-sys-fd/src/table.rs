//! The descriptor table.
//!
//! One slot per descriptor number, in static storage. A descriptor is a name for a device: opening
//! one binds a number to a device, and every operation on that number is forwarded to it.

use core::cell::UnsafeCell;

use nx_sys_sync::Mutex;

use crate::{
    device::{
        DeviceError,
        DeviceId,
    },
    registry,
};

/// Number of descriptors the table can hold.
///
/// Matches the capacity the C standard library used, so a program that ran against the C table
/// cannot run out of descriptors sooner here.
pub const MAX_FD: usize = 1024;

/// The process-wide descriptor table.
static TABLE: Table = Table {
    mutex: Mutex::new(),
    slots: UnsafeCell::new({
        // Descriptors 0, 1 and 2 are open before anything asks, so that early output has somewhere
        // to go. They start on the matching standard device slots.
        let mut slots = [None; MAX_FD];
        // SAFETY: the standard slots are registry constants, so they are in range by
        // construction.
        slots[0] = Some(DeviceId::from_index_unchecked(registry::STD_IN));
        slots[1] = Some(DeviceId::from_index_unchecked(registry::STD_OUT));
        slots[2] = Some(DeviceId::from_index_unchecked(registry::STD_ERR));
        slots
    }),
};

/// An open descriptor.
///
/// A value of this type names a descriptor slot that exists; whether it is open is a separate
/// question the table answers.
///
/// Validation lives in the [`TryFrom<usize>`] impl below, which is the only place the bound is
/// checked. [`Fd::from_number_unchecked`] bypasses it for callers that already hold the proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fd(u32);

impl Fd {
    /// Names descriptor `number` without checking the bound.
    ///
    /// The caller must ensure `number` is below [`MAX_FD`]. This constructor performs no
    /// validation; an out-of-range descriptor is reported as not open by every operation that takes
    /// one.
    pub(crate) const fn from_number_unchecked(number: usize) -> Self {
        Self(number as u32)
    }

    /// Returns the descriptor number.
    pub const fn number(self) -> usize {
        self.0 as usize
    }
}

impl TryFrom<usize> for Fd {
    type Error = InvalidFd;

    fn try_from(number: usize) -> Result<Self, Self::Error> {
        if number >= MAX_FD {
            return Err(InvalidFd(number));
        }
        Ok(Self(number as u32))
    }
}

/// Errors returned when converting a descriptor number into an [`Fd`].
///
/// The number is outside the table, so it names no descriptor at all. Nothing was looked up.
#[derive(Debug, thiserror::Error)]
#[error("Descriptor {0} is outside the table")]
pub struct InvalidFd(usize);

/// Binds the lowest free descriptor number to `device`.
///
/// # Errors
///
/// Returns [`OpenError::NoDevice`] when `device` names no registered device, and
/// [`OpenError::NoDescriptors`] when every slot is in use.
pub fn open(device: DeviceId) -> Result<Fd, OpenError> {
    if registry::get(device).is_none() {
        return Err(OpenError::NoDevice);
    }

    let mut table = TABLE.lock();
    let slots = table.slots();

    let Some(number) = slots.iter().position(Option::is_none) else {
        return Err(OpenError::NoDescriptors);
    };
    slots[number] = Some(device);

    // SAFETY: `number` indexes `slots`, so it is below `MAX_FD` by construction.
    Ok(Fd::from_number_unchecked(number))
}

/// Errors returned by [`open`].
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// The device is not registered
    ///
    /// Occurs when the registry slot named is empty, either because nothing registered there or
    /// because the device was unregistered. No descriptor was taken.
    #[error("No device is registered at that slot")]
    NoDevice,

    /// Every descriptor is in use
    ///
    /// Occurs once the table is full. Nothing was allocated and no slot was disturbed, so the call
    /// is safe to retry after a descriptor is closed.
    #[error("No free descriptors remain")]
    NoDescriptors,
}

/// Releases `fd` and runs its device's close.
///
/// The slot is freed before the device is told, so the device's close runs with the table unlocked
/// and the descriptor number already reusable. A device that blocks in close therefore cannot hold
/// up the table.
///
/// # Errors
///
/// Returns [`CloseError::BadDescriptor`] when `fd` is not open, or [`CloseError::Device`] when the
/// device reported a failure. The descriptor is released either way.
pub fn close(fd: Fd) -> Result<(), CloseError> {
    let Some(device) = take(fd) else {
        return Err(CloseError::BadDescriptor);
    };

    match registry::get(device) {
        Some(registered) => registered.close().map_err(CloseError::Device),
        // The device was unregistered while the descriptor was open; there is nothing to tell.
        None => Ok(()),
    }
}

/// Errors returned by [`close`].
#[derive(Debug, thiserror::Error)]
pub enum CloseError {
    /// The descriptor is not open
    ///
    /// Occurs when the number was never opened, or was closed already. Nothing was released.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The device failed to release the descriptor
    ///
    /// The descriptor number is free regardless, so this reports what the device could not finish
    /// rather than a reason to retry the close.
    #[error("Device failed to close the descriptor")]
    Device(#[source] DeviceError),
}

/// Writes `buf` to the device behind `fd`, returning how many bytes it consumed.
///
/// # Errors
///
/// Returns [`WriteError::BadDescriptor`] when `fd` is not open, [`WriteError::NoDevice`] when its
/// device is no longer registered, or [`WriteError::Device`] with whatever the device reported.
pub fn write(fd: Fd, buf: &[u8]) -> Result<usize, WriteError> {
    let Some(device) = device_of(fd) else {
        return Err(WriteError::BadDescriptor);
    };
    let Some(registered) = registry::get(device) else {
        return Err(WriteError::NoDevice);
    };

    registered.write(buf).map_err(WriteError::Device)
}

/// Errors returned by [`write`].
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The descriptor is not open
    ///
    /// Nothing was written.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The device backing the descriptor is no longer registered
    ///
    /// Occurs when a device is unregistered while descriptors on it are still open. Nothing was
    /// written.
    #[error("No device is registered for that descriptor")]
    NoDevice,

    /// The device could not take the bytes
    #[error("Device failed to write")]
    Device(#[source] DeviceError),
}

/// Reads from the device behind `fd` into `buf`, returning how many bytes it produced.
///
/// # Errors
///
/// Returns [`ReadError::BadDescriptor`] when `fd` is not open, [`ReadError::NoDevice`] when its
/// device is no longer registered, or [`ReadError::Device`] with whatever the device reported.
pub fn read(fd: Fd, buf: &mut [u8]) -> Result<usize, ReadError> {
    let Some(device) = device_of(fd) else {
        return Err(ReadError::BadDescriptor);
    };
    let Some(registered) = registry::get(device) else {
        return Err(ReadError::NoDevice);
    };

    registered.read(buf).map_err(ReadError::Device)
}

/// Errors returned by [`read`].
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The descriptor is not open
    ///
    /// Nothing was read.
    #[error("Descriptor is not open")]
    BadDescriptor,

    /// The device backing the descriptor is no longer registered
    ///
    /// Occurs when a device is unregistered while descriptors on it are still open. Nothing was
    /// read.
    #[error("No device is registered for that descriptor")]
    NoDevice,

    /// The device could not produce bytes
    #[error("Device failed to read")]
    Device(#[source] DeviceError),
}

/// Returns the device backing `fd`.
pub fn device_of(fd: Fd) -> Option<DeviceId> {
    let number = fd.number();
    if number >= MAX_FD {
        return None;
    }

    TABLE.lock().slots()[number]
}

/// Frees `fd`, returning the device it named.
pub(crate) fn take(fd: Fd) -> Option<DeviceId> {
    let number = fd.number();
    if number >= MAX_FD {
        return None;
    }

    TABLE.lock().slots()[number].take()
}

/// The descriptor table.
struct Table {
    mutex: Mutex,
    slots: UnsafeCell<[Option<DeviceId>; MAX_FD]>,
}

// SAFETY: every access to `slots` goes through `mutex`, and the table is never moved.
unsafe impl Sync for Table {}

impl Table {
    /// Locks the table for the lifetime of the returned guard.
    fn lock(&self) -> Locked<'_> {
        self.mutex.lock();
        Locked(self)
    }
}

/// Exclusive access to the table's slots, unlocking on drop.
struct Locked<'a>(&'a Table);

impl Locked<'_> {
    /// Returns the slots this guard has exclusive access to.
    fn slots(&mut self) -> &mut [Option<DeviceId>; MAX_FD] {
        // SAFETY: holding this guard means the table lock is held, so no other reference exists.
        unsafe { &mut *self.0.slots.get() }
    }
}

impl Drop for Locked<'_> {
    fn drop(&mut self) {
        self.0.mutex.unlock();
    }
}
