//! Shared helpers for the pm FFI surface.

use core::{
    cell::UnsafeCell,
    mem::size_of,
};

/// Generic fallback used when no specific result code is available.
pub(super) use nx_sf::error::GENERIC_ERROR;
use nx_sf::error::{
    LibnxError,
    libnx_error,
};
use static_assertions::const_assert_eq;

/// `MAKERESULT(Module_Libnx, LibnxError_IncompatSysVer)` — returned by libnx
/// gating checks when the running firmware predates a command. Mirrors
/// `nx-rt`'s SM FFI for the same condition.
pub(super) const RC_INCOMPAT_SYSVER: u32 = libnx_error(LibnxError::IncompatSysVer);

/// Encodes a major/minor/micro firmware version the same way libnx's
/// `MAKEHOSVERSION` macro does.
#[inline]
pub(super) const fn make_hos_version(major: u8, minor: u8, micro: u8) -> u32 {
    ((major as u32) << 16) | ((minor as u32) << 8) | (micro as u32)
}

unsafe extern "C" {
    /// Provided by libnx (or its `nx-rt` override) at link time.
    pub(super) safe fn hosversionGet() -> u32;
    /// Provided by libnx (or its `nx-rt` override) at link time.
    pub(super) safe fn hosversionIsAtmosphere() -> bool;
}

/// Returns `true` when the running firmware is at least `major.minor.micro`.
#[inline]
pub(super) fn hosversion_at_least(major: u8, minor: u8, micro: u8) -> bool {
    hosversionGet() >= make_hos_version(major, minor, micro)
}

/// Returns `true` when the firmware predates `major.minor.micro`.
#[inline]
pub(super) fn hosversion_before(major: u8, minor: u8, micro: u8) -> bool {
    !hosversion_at_least(major, minor, micro)
}

/// libnx `Event` layout, used by `pmdmntHookTo*` and `pmshellGetProcessEventHandle`.
///
/// Matches `Event` in `libnx/include/switch/kernel/event.h`:
/// ```c
/// typedef struct {
///     Handle revent;     // u32
///     Handle wevent;     // u32
///     bool   autoclear;  // u8
/// } Event;
/// ```
#[repr(C)]
pub(super) struct LibnxEvent {
    pub revent: u32,
    pub wevent: u32,
    pub autoclear: bool,
}
const_assert_eq!(size_of::<LibnxEvent>(), 12);

/// `UnsafeCell` wrapper that asserts `Sync` for static storage. Access is
/// synchronised externally (by the per-service `RwLock` in `state`).
#[repr(transparent)]
pub(super) struct SyncUnsafeCell<T>(UnsafeCell<T>);

// SAFETY: synchronisation is provided by the per-service singleton lock.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub(super) const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub(super) fn get(&self) -> *mut T {
        self.0.get()
    }
}
