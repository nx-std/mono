//! # Thread registry (process-wide)
//!
//! This module owns a global collection that tracks every [`Thread`] that is
//! alive in the current process.
//!
//! ## Design at a glance
//!
//! • **Storage** – Threads are _not_ inserted directly into the intrusive list
//!   because that would change the C ABI of [`Thread`]. Instead, every entry
//!   is a heap-allocated `Box<Node>` that contains the intrusive
//!   [`LinkedListLink`] and a raw, non-null pointer back to the real [`Thread`]
//!   object. The allocation is created on insertion and destroyed immediately
//!   after removal.
//!
//! • **Global access** – A `static` `Mutex<ThreadList>` called
//!   `THREAD_LIST` serialises all mutations.
//!
//! • **Thread-safety** – User code can only access the underlying [`Thread`]s
//!   while the global mutex is held, guaranteeing data-race freedom even
//!   though the list itself stores raw pointers.
//!
//! The public API purposefully avoids returning raw pointers or references
//! that outlive the mutex guard. If you need to operate on every live thread,
//! pass a closure to [`for_each`] _while holding the guard_ so that borrow
//! rules remain intact.
//!
//! None of the public functions allocate except when `insert` needs to create
//! a new `Node`. All operations are `O(n)` in the number of live threads, but
//! the collections are small in practice so this has not been a concern.
//!
//! [`Thread`]: super::info::Thread
//! [`LinkedListLink`]: intrusive_collections::LinkedListLink

use core::ptr::NonNull;

use nx_std_sync::once_lock::OnceLock;
use nx_svc::debug::{BreakReason, break_event};

use crate::thread_impl::Thread;

/// The main thread
///
/// This is initialized when libnx runtime is initialized.
static MAIN_THREAD: OnceLock<MainThread> = OnceLock::new();

/// Sets the global record for the process' main [`Thread`].
///
/// This function **must** be invoked exactly once during program start-up,
/// typically by the runtime right after the main thread has been fully
/// initialised but **before** any additional threads are spawned or the
/// thread registry is otherwise accessed.
///
/// # Panics
///
/// Panics if this function is called more than once.
///
/// # Safety
///
/// * The caller must uphold the following guarantees:
///   * `thread` refers to the currently executing main thread and is **fully**
///     initialised.
///   * The provided [`Thread`] value lives for the entire lifetime of the
///     process (it is stored globally and later returned by [`main_thread`]).
pub unsafe fn set_main_thread(thread: Thread) {
    if MAIN_THREAD.set(MainThread::new(thread)).is_err() {
        // TODO: Add a proper error message here.
        // panic!("Main thread already set");
        break_event(BreakReason::Panic, 0, 0);
    }
}

/// Returns a shared reference to the process' main [`Thread`].
///
/// The returned reference has a `'static` lifetime because the underlying
/// `Thread` is stored globally.
///
/// # Panics
///
/// Panics if the main thread has not yet been registered via [`set_main_thread`].
///
/// # Safety
///
/// * Do **not** concurrently obtain mutable access to the same `Thread` while
///   holding the returned shared reference; doing so is **undefined behaviour**.
pub unsafe fn main_thread() -> &'static Thread {
    let Some(thread) = MAIN_THREAD.get() else {
        // TODO: Add a proper error message here.
        // panic!("Main thread not set");
        break_event(BreakReason::Panic, 0, 0);
    };

    thread
}

/// Returns a raw pointer to the process' main [`Thread`].
///
/// The returned pointer is guaranteed to be non-null and is valid for the
/// entire lifetime of the process.
///
/// # Panics
///
/// Panics if the main thread has not yet been registered via [`set_main_thread`].
///
/// # Safety
///
/// * The caller must ensure that no other references (shared or mutable) to the
///   `Thread` exist when dereferencing the returned pointer to create a mutable
///   reference. Creating aliasing references is **undefined behaviour**.
pub unsafe fn main_thread_ptr() -> NonNull<Thread> {
    let Some(thread) = MAIN_THREAD.get() else {
        // TODO: Add a proper error message here.
        // panic!("Main thread not set");
        break_event(BreakReason::Panic, 0, 0);
    };

    NonNull::from(&thread.0)
}

// TODO: Thread registry list functionality temporarily disabled for dyn slots initialization
// The insert, remove, and for_each functions have been removed.

// A wrapper around `Thread` to safely mark it as Send + Sync.
// This is safe specifically for the main thread because it's initialized
// once and then treated as read-only for the lifetime of the process.
struct MainThread(Thread);

impl MainThread {
    /// Creates a new `MainThread` wrapper from a `Thread`.
    fn new(thread: Thread) -> Self {
        Self(thread)
    }
}

impl core::ops::Deref for MainThread {
    type Target = Thread;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// SAFETY: The main thread info is initialized once at startup and then
// becomes effectively read-only. The OnceLock ensures safe initialization.
// Access from other threads is safe because the data doesn't change.
unsafe impl Send for MainThread {}
unsafe impl Sync for MainThread {}
