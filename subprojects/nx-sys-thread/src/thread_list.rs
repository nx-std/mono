//! Process-wide live-thread registry.
//!
//! This module owns the intrusive, doubly-linked list of every live thread
//! created or adopted by `nx-sys-thread`, following the musl libc model of a
//! global live-thread list (musl's `struct pthread { prev, next, ... }`).
//!
//! # Why a registry exists
//!
//! Runtime TSD keys keep destructor metadata in a process-global table while
//! storing values in each thread's flat per-thread slot array. Because those
//! arrays carry no per-key generation state, deleting a key must actively
//! clear that slot from *every* live thread. Walking this registry is the
//! mechanism that makes [`crate::tsd`] key deletion possible. The registry is
//! also how thread-lifecycle code locates the current thread without separate
//! global bookkeeping.
//!
//! # Ownership of the links
//!
//! The list links live in [`ThreadControl::prev`]/[`ThreadControl::next`], but
//! they belong to *this* module: only the operations below, all of which run
//! under [`struct@THREAD_MUTEX`], may read or write them. The libnx ABI
//! adapter mirrors links into `LibnxThread.next`/`prev_next` separately; this
//! core registry remains the source of truth.
//!
//! The links are `AtomicPtr`, but only so a concurrent `&ThreadControl`
//! observer stays sound across this module's writes: no typed `&ThreadControl`
//! may be held across the window in which the target thread can self-mutate
//! `state`/`prev`/`next`, so a concurrently-live `ThreadControl` is reached
//! through a raw pointer or via these atomic fields. `THREAD_MUTEX`, not the
//! atomic ordering, supplies all happens-before, so every access below uses
//! [`Ordering::Relaxed`].

use core::{
    ptr::null_mut,
    sync::atomic::Ordering,
};

use nx_sys_sync::Mutex;

use crate::thread::ThreadControl;

/// Head of the process-wide live-thread list, or null when no thread is
/// registered.
///
/// Only ever read or written while [`struct@THREAD_MUTEX`] is held.
static mut THREAD_LIST: *mut ThreadControl = null_mut();

/// Guards every registry operation, serializing list mutation and iteration.
static THREAD_MUTEX: Mutex = Mutex::new();

/// Registers `thread` by prepending it to the live-thread list head.
///
/// # Safety
///
/// - `thread` must point to a valid [`ThreadControl`] that stays live for as
///   long as it remains registered.
/// - `thread` must not already be registered; inserting the same node twice
///   corrupts the list.
pub unsafe fn insert(thread: *mut ThreadControl) {
    THREAD_MUTEX.lock();

    // SAFETY: `THREAD_MUTEX` is held, so `THREAD_LIST` and every node's links
    // are exclusively ours. The caller guarantees `thread` is a valid,
    // not-yet-registered `ThreadControl`, and the old head (if any) is a node
    // previously inserted through this same path.
    unsafe {
        let head = THREAD_LIST;
        (*thread).prev.store(null_mut(), Ordering::Relaxed);
        (*thread).next.store(head, Ordering::Relaxed);
        if !head.is_null() {
            (*head).prev.store(thread, Ordering::Relaxed);
        }
        THREAD_LIST = thread;
    }

    THREAD_MUTEX.unlock();
}

/// Unregisters `thread`, unlinking it from both neighbors.
///
/// The removed node's links are reset to null so a stale pointer cannot dangle
/// back into the list.
///
/// # Safety
///
/// `thread` must point to a valid [`ThreadControl`] that is currently
/// registered via [`insert`].
pub unsafe fn remove(thread: *mut ThreadControl) {
    THREAD_MUTEX.lock();

    // SAFETY: `THREAD_MUTEX` is held, so the list shape is exclusively ours.
    // The caller guarantees `thread` is currently registered, so its `prev`
    // and `next` links — and the neighbors they name — are valid.
    unsafe {
        let prev = (*thread).prev.load(Ordering::Relaxed);
        let next = (*thread).next.load(Ordering::Relaxed);

        if prev.is_null() {
            // `thread` was the list head.
            THREAD_LIST = next;
        } else {
            (*prev).next.store(next, Ordering::Relaxed);
        }
        if !next.is_null() {
            (*next).prev.store(prev, Ordering::Relaxed);
        }

        (*thread).prev.store(null_mut(), Ordering::Relaxed);
        (*thread).next.store(null_mut(), Ordering::Relaxed);
    }

    THREAD_MUTEX.unlock();
}

/// Invokes `f` once for every registered thread while the registry is locked.
///
/// This is the iteration primitive behind musl libc-style TSD key deletion:
/// `f` typically clears one slot from each thread's TSD array.
///
/// # Safety
///
/// `f` must not call [`insert`], [`remove`], or [`for_each`]: [`struct@THREAD_MUTEX`]
/// is held for the whole walk and is not reentrant, so re-entering deadlocks.
/// `f` must likewise not mutate any node's registry links. Each pointer handed
/// to `f` is a valid, currently-registered [`ThreadControl`].
pub unsafe fn for_each<F: FnMut(*mut ThreadControl)>(mut f: F) {
    THREAD_MUTEX.lock();

    // SAFETY: `THREAD_MUTEX` is held for the whole walk, so the list shape is
    // stable. `next` is captured before each `f` call so the cursor stays
    // valid; `f` is contracted (see `# Safety`) not to mutate the links.
    unsafe {
        let mut node = THREAD_LIST;
        while !node.is_null() {
            let next = (*node).next.load(Ordering::Relaxed);
            f(node);
            node = next;
        }
    }

    THREAD_MUTEX.unlock();
}
