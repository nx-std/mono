//! Pool of cloned IPC sessions for concurrent socket operations.
//!
//! libnx's `bsd` service uses an internal `SessionMgr` to hand out a free
//! session to each calling thread. Without that, a blocking `recv()` would
//! serialise every other socket call on the process. This module re-creates
//! the same behaviour with a fixed-size pool guarded by a mutex/condvar pair
//! from [`nx_std_sync`].

use alloc::boxed::Box;

use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};
use nx_std_sync::{
    condvar::Condvar,
    mutex::Mutex,
};

/// Maximum number of pool slots representable in the free-mask `u32`.
pub(crate) const MAX_SESSIONS: usize = 32;

/// Owns the per-session IPC handles plus the bookkeeping needed to hand them
/// out one at a time. Slot ownership is tracked in a `u32` bitset (`free_mask`)
/// where bit `i` set means slot `i` is currently free.
pub(crate) struct SessionPool {
    sessions: Box<[Session]>,
    state: Mutex<PoolState>,
    cv: Condvar,
}

struct PoolState {
    /// Bit `i` set => slot `i` is free.
    free_mask: u32,
}

impl SessionPool {
    /// Builds a pool over the given sessions.
    ///
    /// The count is bounded upstream by
    /// [`SessionCount`](crate::SessionCount), which is what keeps the free
    /// mask representable; nothing is clamped here, because a pool larger than
    /// the mask can no longer be built.
    ///
    /// # Panics
    ///
    /// In debug builds, if `sessions` holds more than [`MAX_SESSIONS`]
    /// entries — a bound that was not applied where the count entered.
    pub(crate) fn new(sessions: Box<[Session]>) -> Self {
        debug_assert!(
            sessions.len() <= MAX_SESSIONS,
            "session pool exceeds the free-mask width",
        );

        let n = sessions.len();
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
    ///
    /// The returned guard releases the slot back to the pool on drop and wakes
    /// one waiter (if any). Each slot represents a distinct kernel session, so
    /// concurrent `acquire()` calls from different threads operate on
    /// independent IPC channels.
    pub(crate) fn acquire(&self) -> SessionGuard<'_> {
        let mut guard = self.state.lock();
        guard = self.cv.wait_while(guard, |state| state.free_mask == 0);

        // `free_mask` is non-zero by the `wait_while` predicate above, so
        // `trailing_zeros` is below 32 and names a real slot.
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

impl SessionGuard<'_> {
    /// Returns the IPC session handle associated with the held slot.
    #[inline]
    pub(crate) fn session(&self) -> BorrowedSessionHandle<'_> {
        self.pool.sessions[self.slot as usize].handle()
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
