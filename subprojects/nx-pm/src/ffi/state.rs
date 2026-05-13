//! Singletons backing the `pm:*` FFI surface.
//!
//! Each sub-service owns an independent `RwLock<Option<_>>` slot for the
//! connected `Pm*Service`, plus a `MaybeUninit<Service>` shadow buffer used as
//! the return value of `pm*GetServiceSession`. A private SM session is created
//! lazily on the first `pmXxxInitialize` and lives until the program exits
//! (matches libnx's global SM session lifetime).

use core::mem::MaybeUninit;

use nx_service_pm::{PmBmService, PmDmntService, PmInfoService, PmShellService};
use nx_service_sm::SmService;
use nx_sf::ffi::Service;
use nx_std_sync::rwlock::RwLock;

use super::common::SyncUnsafeCell;

/// Private SM session opened on demand by the pm FFI. Never torn down.
pub(super) static SM: RwLock<Option<SmService>> = RwLock::new(None);

/// Acquires or returns the cached SM session. Re-uses the existing connection
/// when one is already established.
pub(super) fn ensure_sm() -> Result<(), nx_service_sm::ConnectError> {
    {
        let guard = SM.read();
        if guard.is_some() {
            return Ok(());
        }
    }

    let mut guard = SM.write();
    if guard.is_some() {
        return Ok(());
    }
    let sm = nx_service_sm::connect()?;
    *guard = Some(sm);
    Ok(())
}

pub(super) static BM: RwLock<Option<PmBmService>> = RwLock::new(None);
pub(super) static DMNT: RwLock<Option<PmDmntService>> = RwLock::new(None);
pub(super) static INFO: RwLock<Option<PmInfoService>> = RwLock::new(None);
pub(super) static SHELL: RwLock<Option<PmShellService>> = RwLock::new(None);

pub(super) static BM_SRV: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());
pub(super) static DMNT_SRV: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());
pub(super) static INFO_SRV: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());
pub(super) static SHELL_SRV: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

/// Populates a `Service` shadow buffer with a non-owning view of `session`.
///
/// `own_handle = 0`, `object_id = 0` (libnx's "override" mode) — the Rust
/// singleton retains exclusive ownership of the kernel handle. The shadow
/// buffer's `Service` must not call `serviceClose` on the cached pointer.
#[inline]
pub(super) fn write_shadow(
    slot: &SyncUnsafeCell<MaybeUninit<Service>>,
    session: &nx_sf::service::Session,
) {
    let service = Service {
        session: session.handle(),
        own_handle: 0,
        object_id: 0,
        pointer_buffer_size: session.pointer_buffer_size(),
    };
    // SAFETY: called while holding the service's write lock, so no other
    // thread is reading the shadow buffer.
    unsafe { slot.get().cast::<Service>().write(service) };
}

/// Zeroes a `Service` shadow buffer on exit so a stray reader sees an
/// `INVALID_HANDLE` rather than a freed kernel handle.
#[inline]
pub(super) fn clear_shadow(slot: &SyncUnsafeCell<MaybeUninit<Service>>) {
    // SAFETY: called while holding the service's write lock.
    unsafe { slot.get().write(MaybeUninit::zeroed()) };
}
