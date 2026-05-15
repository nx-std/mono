use alloc::boxed::Box;
use core::mem::ManuallyDrop;

use nx_sf::service::{Domain, DomainObject};
use nx_std_sync::{condvar::Condvar, mutex::Mutex};

pub(crate) const MAX_SESSIONS: usize = 32;

pub(crate) struct SessionPool {
    sessions: Box<[Domain]>,
    state: Mutex<PoolState>,
    cv: Condvar,
}

struct PoolState {
    free_mask: u32,
}

impl SessionPool {
    pub(crate) fn new(sessions: Box<[Domain]>) -> Self {
        let n = core::cmp::min(sessions.len(), MAX_SESSIONS);
        let free_mask = if n >= MAX_SESSIONS {
            u32::MAX
        } else {
            (1u32 << n) - 1
        };
        Self {
            sessions,
            state: Mutex::new(PoolState { free_mask }),
            cv: Condvar::new(),
        }
    }

    pub(crate) fn acquire(&self) -> SessionGuard<'_> {
        let mut guard = self.state.lock();
        guard = self.cv.wait_while(guard, |state| state.free_mask == 0);

        let slot = guard.free_mask.trailing_zeros() as u8;
        guard.free_mask &= !(1u32 << slot);
        drop(guard);

        SessionGuard { pool: self, slot }
    }
}

pub(crate) struct SessionGuard<'a> {
    pool: &'a SessionPool,
    slot: u8,
}

impl<'a> SessionGuard<'a> {
    #[inline]
    pub(crate) fn domain(&self) -> &'a Domain {
        &self.pool.sessions[self.slot as usize]
    }

    /// Mints a transient [`DomainObject`] for dispatch, wrapped in
    /// [`ManuallyDrop`] so the server-side object is NOT closed when the
    /// dispatch is done.
    ///
    /// # Safety
    ///
    /// The caller must ensure `raw_object_id` is a live server-side object
    /// in this pool slot's [`Domain`] and that no other live `DomainObject`
    /// addresses the same id concurrently within this slot.
    #[inline]
    pub(crate) unsafe fn open_transient(
        &self,
        raw_object_id: u32,
    ) -> Option<ManuallyDrop<DomainObject<'a>>> {
        unsafe { self.domain().open_object_raw(raw_object_id) }.map(ManuallyDrop::new)
    }

    /// Mints a [`DomainObject`] that WILL close the server-side object when
    /// dropped. Use only in `Drop` impls of sub-object wrappers.
    ///
    /// # Safety
    ///
    /// Same as [`open_transient`](Self::open_transient).
    #[inline]
    pub(crate) unsafe fn open_for_close(&self, raw_object_id: u32) -> Option<DomainObject<'a>> {
        unsafe { self.domain().open_object_raw(raw_object_id) }
    }
}

impl Drop for SessionGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.pool.state.lock();
        state.free_mask |= 1u32 << self.slot;
        drop(state);
        self.pool.cv.notify_one();
    }
}
