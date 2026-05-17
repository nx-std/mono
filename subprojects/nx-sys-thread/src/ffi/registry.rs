//! Process-global side registry anchoring a C-owned thread handle to its
//! pinned Rust core object.
//!
//! The libnx `thread.h` and devkitPro/libsysbase newlib pthread ABIs both hand
//! the C caller a small `#[repr(C)]` handle struct it owns, while the
//! authoritative runtime object is a distinct, larger, non-`repr(C)` Rust core
//! type that must stay heap-pinned at a fixed address. The thread-lifecycle
//! adapters therefore need a recoverable `C handle -> pinned core` association;
//! a [`SideRegistry`] is it (Resolved Question #5).
//!
//! It is deliberately **not** the live-thread registry ([`crate::thread_list`])
//! and must not be conflated with it. A live-thread entry is unlinked the
//! instant a thread terminates, but `threadWaitForExit`/`threadClose` operate
//! on already-exited threads — so a side-registry entry lives across the whole
//! C ownership window (`create` -> `close`), independent of thread liveness.
//! An entry's presence is itself the double-close / double-join guard: a stale
//! handle whose thread was already closed fails the lookup, which the adapter
//! maps to an error code instead of dereferencing a dangling pointer.

use alloc::vec::Vec;
use core::cell::UnsafeCell;

use nx_sys_sync::Mutex;

/// A process-global map from a C-owned handle to the pinned Rust core object
/// that backs it.
///
/// `K` is the C handle pointer the caller owns (`*mut LibnxThread` for the
/// libnx adapter, `*mut LibsysbasePthread` for libsysbase); `V` is the pinned
/// core object or its owning handle.
pub(super) struct SideRegistry<K, V> {
    /// Serializes every registry mutation and lookup.
    mutex: Mutex,
    /// `key -> value` entries. Only ever read or written while `mutex` is held.
    entries: UnsafeCell<Vec<(K, V)>>,
}

// SAFETY: every read or write of `entries` is bracketed by `mutex.lock()` /
// `mutex.unlock()`, so the `UnsafeCell` is never accessed concurrently — the
// data race a bare `static` of an interior-mutable type would invite cannot
// occur. The stored keys are process-global heap pointers to C handle structs,
// and the values are pinned process-global core objects (or `Arc`-shared
// handles to them), never thread-local storage — so an entry is sound to
// observe or reclaim from a thread other than the one that inserted it.
unsafe impl<K, V> Sync for SideRegistry<K, V> {}

impl<K: Copy + Eq, V> SideRegistry<K, V> {
    /// Creates an empty registry.
    pub(super) const fn new() -> Self {
        Self {
            mutex: Mutex::new(),
            entries: UnsafeCell::new(Vec::new()),
        }
    }

    /// Inserts a `key -> value` entry.
    pub(super) fn insert(&self, key: K, value: V) {
        self.mutex.lock();
        // SAFETY: `mutex` is held, so `entries` is exclusively this thread's.
        unsafe { (*self.entries.get()).push((key, value)) };
        self.mutex.unlock();
    }

    /// Removes the entry for `key`, returning its value, or `None` if absent.
    pub(super) fn remove(&self, key: K) -> Option<V> {
        self.mutex.lock();
        // SAFETY: `mutex` is held, so `entries` is exclusively this thread's.
        let entries = unsafe { &mut *self.entries.get() };
        let removed = entries
            .iter()
            .position(|(k, _)| *k == key)
            .map(|idx| entries.swap_remove(idx).1);
        self.mutex.unlock();
        removed
    }

    /// Returns the first key whose value satisfies `pred`, or `None`.
    ///
    /// `pred` runs while the registry is locked, so it must not re-enter the
    /// registry; in practice it is a cheap pointer comparison.
    pub(super) fn find_key(&self, mut pred: impl FnMut(&V) -> bool) -> Option<K> {
        self.mutex.lock();
        // SAFETY: `mutex` is held, so `entries` is exclusively this thread's.
        let entries = unsafe { &*self.entries.get() };
        let key = entries.iter().find(|(_, v)| pred(v)).map(|(k, _)| *k);
        self.mutex.unlock();
        key
    }
}

impl<K: Copy + Eq, V: Copy> SideRegistry<K, V> {
    /// Returns a copy of the value for `key`, leaving the entry in place.
    ///
    /// The libnx lifecycle adapters use this so the SVC call can run after the
    /// registry unlocks: the `Copy` `V` (`NonNull<ThreadControl>`) keeps
    /// pointing at the pinned pointee, which stays valid only *while its entry
    /// lives*. A concurrent `remove` (`threadClose`) frees that pointee, so a
    /// value obtained here is sound past the unlock only under the caller-side
    /// single-owner contract — the same handle is never operated on
    /// concurrently (see the `ffi::libnx` concurrency contract).
    pub(super) fn get(&self, key: K) -> Option<V> {
        self.mutex.lock();
        // SAFETY: `mutex` is held, so `entries` is exclusively this thread's.
        let entries = unsafe { &*self.entries.get() };
        let value = entries.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        self.mutex.unlock();
        value
    }
}
