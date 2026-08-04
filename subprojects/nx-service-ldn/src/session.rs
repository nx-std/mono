//! Pool of cloned IPC domain sessions for concurrent `ldn` calls.
//!
//! libnx's `ldn` service uses an internal `SessionMgr` (`sessionmgrCreate(...,
//! 0x3)`) so long-running commands like `Scan` and `RecvActionFrame` don't
//! serialise unrelated state queries. This module recreates the same behaviour
//! with a fixed-size pool guarded by a mutex/condvar pair from [`nx_std_sync`].
//!
//! Mirrors the pattern in `nx-service-bsd/src/session.rs`; differs only in the
//! default size and in storing cloned **domain** sessions shared between LCS
//! and ICPM sub-objects, so the per-call `DomainObject` view must be opened on
//! the fly from each slot.

use alloc::boxed::Box;

use nx_sf::service::{
    Domain,
    DomainObjectRef,
    DomainRef,
};
use nx_std_sync::{
    condvar::Condvar,
    mutex::Mutex,
};

/// Maximum number of pool slots representable in the free-mask `u32`.
pub(crate) const MAX_SESSIONS: usize = 32;

/// Default pool size for libnx-parity `ldn` connections.
pub(crate) const LDN_POOL_SIZE: usize = 3;

/// Owns the per-session IPC handles plus the bookkeeping needed to hand them
/// out one at a time. Slot ownership is tracked in a `u32` bitset (`free_mask`)
/// where bit `i` set means slot `i` is currently free.
pub(crate) struct SessionPool {
    sessions: Box<[Domain]>,
    state: Mutex<PoolState>,
    cv: Condvar,
}

struct PoolState {
    /// Bit `i` set => slot `i` is free.
    free_mask: u32,
}

impl SessionPool {
    /// Builds a pool over the given domain sessions. Each `Domain` represents a
    /// cloned kernel session sharing the same server-side domain object table;
    /// callers open sub-object views at dispatch time via
    /// [`SessionGuard::open_object_raw`].
    ///
    /// Caller is responsible for ensuring `sessions.len() <= MAX_SESSIONS`;
    /// values beyond that limit are silently truncated to keep the bitset
    /// representable.
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

    /// Acquires an exclusive session slot, blocking until one is free.
    pub(crate) fn acquire(&self) -> SessionGuard<'_> {
        let mut guard = self.state.lock();
        guard = self.cv.wait_while(guard, |state| state.free_mask == 0);

        // SAFETY: free_mask != 0 by the wait_while predicate, so trailing_zeros < 32.
        let slot = guard.free_mask.trailing_zeros() as u8;
        guard.free_mask &= !(1u32 << slot);
        drop(guard);

        SessionGuard { pool: self, slot }
    }
}

/// RAII guard returned by [`SessionPool::acquire`].
pub(crate) struct SessionGuard<'a> {
    pool: &'a SessionPool,
    slot: u8,
}

impl<'a> SessionGuard<'a> {
    /// Returns the domain backing this slot.
    #[inline]
    pub(crate) fn domain(&self) -> DomainRef<'a> {
        self.pool.sessions[self.slot as usize].as_borrowed()
    }

    /// Opens a borrowed view onto a sub-object inside the slot's domain. The
    /// view closes nothing, so the LCS and ICPM sub-objects outlive every call
    /// made through them.
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
}

impl Drop for SessionGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.pool.state.lock();
        state.free_mask |= 1u32 << self.slot;
        drop(state);
        self.pool.cv.notify_one();
    }
}
