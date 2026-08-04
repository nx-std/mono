//! Pool of cloned IPC domain sessions for concurrent `htcs` calls.
//!
//! libnx's `htcs` service uses an internal `SessionMgr`
//! (`sessionmgrCreate(..., num_sessions)`) to allow concurrent IPC dispatch
//! on the domain session. This module recreates the same behaviour with a
//! fixed-size pool guarded by a mutex/condvar pair from [`nx_std_sync`].

use alloc::boxed::Box;

use nx_sf::service::{
    Domain,
    DomainObject,
    DomainObjectRef,
    DomainRef,
};
use nx_std_sync::{
    condvar::Condvar,
    mutex::Mutex,
};

/// Maximum number of pool slots representable in the free-mask `u32`.
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
    pub(crate) fn domain(&self) -> DomainRef<'a> {
        self.pool.sessions[self.slot as usize].as_borrowed()
    }

    /// Addresses `raw_object_id` inside this pool slot's domain. The view
    /// closes nothing, so the socket outlives every call made through it.
    ///
    /// The caller must ensure `raw_object_id` names a live server-side object
    /// in this pool slot's domain; a stale id is answered with an error by the
    /// request it reaches.
    #[inline]
    pub(crate) fn open_object_unchecked(&self, raw_object_id: u32) -> Option<DomainObjectRef<'a>> {
        // SAFETY: the liveness of `raw_object_id` is this function's own
        // precondition, forwarded to its caller; the view closes nothing, so a
        // stale id costs a rejected request rather than a wrong close.
        DomainObjectRef::from_raw_unchecked(self.domain(), raw_object_id)
    }

    /// Takes on the obligation to close the server-side object, which happens
    /// when the returned value drops. Use only where the close is owed, which
    /// for a socket means the teardown methods that consume it.
    ///
    /// The caller must additionally ensure no other live [`DomainObject`]
    /// addresses the same id within this slot, since a second one closes an id
    /// the server may have reused. The [`SessionGuard`] free-mask makes the
    /// slot exclusive, which is what discharges that for a single guard.
    #[inline]
    pub(crate) fn open_object_for_close_unchecked(
        &self,
        raw_object_id: u32,
    ) -> Option<DomainObject<'a>> {
        // SAFETY: both halves of the precondition - a live id, and no other
        // owner for it in this slot - are this function's own, forwarded to its
        // caller.
        DomainObject::from_raw_unchecked(self.domain(), raw_object_id)
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
