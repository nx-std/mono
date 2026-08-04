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

    /// Addresses a server-side object for dispatch. The view closes nothing,
    /// so the object outlives the dispatch.
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
    /// when the returned value drops. Use only where the close is owed: a
    /// sub-object wrapper's `Drop`, or a teardown method that consumes it.
    ///
    /// The caller must additionally ensure no other live [`DomainObject`]
    /// addresses the same id within this slot, since a second one closes an id
    /// the server may have reused.
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
