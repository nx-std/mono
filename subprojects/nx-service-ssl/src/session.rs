//! Pool of cloned IPC domain sessions for concurrent SSL calls.
//!
//! libnx's `ssl` service uses an internal `SessionMgr`
//! (`sessionmgrCreate(..., num_sessions)`) to allow concurrent IPC dispatch
//! on the domain session. This module recreates the same behaviour with a
//! fixed-size pool guarded by a mutex/condvar pair from [`nx_std_sync`].

use alloc::boxed::Box;

use nx_sf::service::Domain;
use nx_std_sync::{condvar::Condvar, mutex::Mutex};

/// Maximum number of pool slots representable in the free-mask `u32`.
pub(crate) const MAX_SESSIONS: usize = 32;

/// Default pool size matching libnx's maximum (`num_sessions` capped at 4).
pub(crate) const SSL_POOL_SIZE: usize = 4;

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
}

impl Drop for SessionGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.pool.state.lock();
        state.free_mask |= 1u32 << self.slot;
        drop(state);
        self.pool.cv.notify_one();
    }
}
