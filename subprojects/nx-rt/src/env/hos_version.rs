//! Horizon OS Version Management
//!
//! This module provides APIs for querying and managing the Horizon OS version
//! information, including detection of Atmosphere custom firmware.

use core::sync::atomic::{AtomicU32, Ordering};

/// Atmosphere flag bit position (bit 31)
const ATMOSPHERE_BIT: u32 = 1 << 31;

/// Global HOS version storage (mutable at runtime, bit 31 = Atmosphere flag)
static VERSION: AtomicU32 = AtomicU32::new(0);

/// Returns the current Horizon OS version.
///
/// Returns `HosVersion::default()` (0.0.0) if the version has not been set.
pub fn get() -> HosVersion {
    HosVersion::from_u32(VERSION.load(Ordering::Acquire))
}

/// Returns true if running on Atmosphere custom firmware.
pub fn is_atmosphere() -> bool {
    (VERSION.load(Ordering::Acquire) & ATMOSPHERE_BIT) != 0
}

/// Sets the HOS version (internal use only).
///
/// This is called during environment initialization and from FFI.
pub(crate) fn set(version: u32) {
    VERSION.store(version, Ordering::Release);
}

/// Represents a Horizon OS version (major.minor.patch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct HosVersion(u32);

impl HosVersion {
    /// Creates a new HosVersion from major, minor, and patch components.
    #[inline]
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self(((major as u32) << 16) | ((minor as u32) << 8) | (patch as u32))
    }

    /// Creates a HosVersion from a raw packed value.
    #[inline]
    pub const fn from_u32(raw: u32) -> Self {
        Self(raw & !ATMOSPHERE_BIT)
    }

    /// Returns the raw packed version value (without Atmosphere bit).
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the major version component.
    #[inline]
    pub const fn major(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    /// Returns the minor version component.
    #[inline]
    pub const fn minor(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    /// Returns the patch version component.
    #[inline]
    pub const fn patch(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}
