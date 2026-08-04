//! What it means to be a device.
//!
//! A device supplies the behavior behind an open descriptor: how bytes are written, how they are
//! read, what happens on close. Implementing one is the point of this crate, and it is done in
//! Rust: a device implements [`Device`] and registers itself, and never sees a descriptor number,
//! a raw pointer, or an error number.
//!
//! Every operation has a default that reports [`DeviceError::Unsupported`], so a device implements
//! only what it actually offers. A console implements [`Device::write`] and nothing else.

use core::ffi::CStr;

/// Behavior behind an open descriptor.
///
/// Implementations are registered once, live for the life of the process, and are shared by every
/// descriptor opened against them, so they take `&self` and keep nothing per descriptor.
pub trait Device: Sync {
    /// Name this device is reached by, as the prefix of a path such as `"sdmc:/file"`.
    fn name(&self) -> &'static CStr;

    /// Writes `buf`, returning how many bytes were consumed.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device cannot be written to, or
    /// [`DeviceError::Io`] when it rejected the bytes.
    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::Unsupported)
    }

    /// Reads into `buf`, returning how many bytes were produced.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device cannot be read from, or
    /// [`DeviceError::Io`] when it failed to produce bytes.
    fn read(&self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::Unsupported)
    }

    /// Releases whatever the descriptor held.
    ///
    /// Runs with no table lock held, so it may block.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceError::Unsupported`] when the device cannot be closed explicitly, or
    /// [`DeviceError::Io`] when releasing failed.
    fn close(&self) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// Errors returned by the [`Device`] operations.
///
/// Shared by all three because any of them can fail either way: an operation the device does not
/// implement, or one it attempted and could not complete.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// The device does not implement this operation
    ///
    /// Occurs when a caller reaches for an operation the device left at its default. Nothing was
    /// attempted, so the descriptor is unchanged and the call is safe to skip.
    #[error("Operation is not supported by the device")]
    Unsupported,

    /// The device attempted the operation and failed
    ///
    /// Occurs when the underlying device rejected the request. How much was completed before the
    /// failure is the device's business; callers should treat the descriptor's position as
    /// unknown.
    #[error("Device reported an I/O failure")]
    Io,
}

/// Number of registry slots, and so the bound on a [`DeviceId`].
///
/// Fixed by the C standard library, which sizes its own view of the registry with this value.
pub const MAX_DEVICES: usize = 35;

/// Where a device sits in the registry.
///
/// The C standard library indexes devices by this number, and a descriptor records which device
/// backs it. A value of this type names a slot that exists; whether a device occupies that slot is
/// a separate question the registry answers.
///
/// Validation lives in the [`TryFrom<usize>`] impl below, which is the only place the bound is
/// checked. [`DeviceId::from_index_unchecked`] bypasses it for callers that already hold the proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceId(u32);

impl DeviceId {
    /// Names the device in registry slot `index` without checking the bound.
    ///
    /// The caller must ensure `index` is below [`MAX_DEVICES`]. This constructor performs no
    /// validation; an out-of-range slot resolves to no device, so a lookup returns nothing and a
    /// descriptor opened against it is refused.
    pub(crate) const fn from_index_unchecked(index: usize) -> Self {
        Self(index as u32)
    }

    /// Returns the registry slot this names.
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }

    /// Returns the raw slot number, for the C boundary.
    #[cfg(feature = "ffi")]
    pub(crate) const fn as_raw(self) -> u32 {
        self.0
    }
}

impl TryFrom<usize> for DeviceId {
    type Error = InvalidDeviceId;

    fn try_from(index: usize) -> Result<Self, Self::Error> {
        if index >= MAX_DEVICES {
            return Err(InvalidDeviceId(index));
        }
        Ok(Self(index as u32))
    }
}

/// Errors returned when converting a slot number into a [`DeviceId`].
///
/// The number is outside the registry, so it names no slot at all. Nothing was looked up.
#[derive(Debug, thiserror::Error)]
#[error("Slot {0} is outside the device registry")]
pub struct InvalidDeviceId(usize);
