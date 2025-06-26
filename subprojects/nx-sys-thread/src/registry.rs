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

use alloc::boxed::Box;
use core::ptr::NonNull;

use intrusive_collections::{LinkedList, LinkedListLink, intrusive_adapter};
use nx_std_sync::{mutex::Mutex, once_lock::OnceLock};
use nx_svc::debug::{BreakReason, break_event};

use crate::thread_impl::Thread;

/// The main thread
///
/// This is initialized when libnx runtime is initialized.
static MAIN_THREAD: OnceLock<MainThread> = OnceLock::new();

/// A mutex-protected lazy-initialised linked list of [`Thread`]s.
static THREAD_LIST: Mutex<ThreadList> = Mutex::new(ThreadList::new_uninit());

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

/// Registers a freshly initialised [`Thread`] with the global registry.
///
/// In release builds this function is **O(1)**, except for a one-time
/// initialisation cost on the first call. In debug builds it is **O(n)** due
/// to a duplicate-insertion check, where *n* is the number of live threads.
///
/// The caller must guarantee that `thread` is fully initialised, unique (i.e.
/// not already present in the registry), and will remain alive until it is
/// later removed via [`remove`].
///
/// # Panics
///
/// Panics in debug builds if `thread` is already present in the registry.
///
/// # Safety
///
/// Calling this function is **unsafe** because the registry stores a raw
/// pointer to `thread` without any lifetime tracking. The caller must
/// guarantee **all** of the following:
///
/// * `thread` refers to a fully-initialised `Thread` value.
/// * The same `Thread` instance has **not** been inserted before. In release
///   builds, violating this rule is **undefined behaviour**.
/// * The pointed-to `Thread` remains alive (is neither moved nor dropped)
///   until it is later removed via [`remove`].
///
/// Violating the first or third rule results in **undefined behaviour**.
pub unsafe fn insert(thread: &Thread) {
    unsafe { THREAD_LIST.lock().insert(thread) };
}

/// Unregisters a `Thread` from the global registry.
///
/// This function is **O(n)** in the number of live threads.
///
/// # Panics
///
/// Panics in debug builds if `thread` is not present in the registry.
///
/// # Safety
///
/// * The caller must ensure that `thread` was previously inserted with
///   [`insert`] and has not been removed already.
/// * In release builds, attempting to remove a thread that is not in the
///   registry results in **undefined behaviour**.
pub unsafe fn remove(thread: &Thread) {
    unsafe { THREAD_LIST.lock().remove(thread) };
}

/// Runs `f` once for every live thread, while holding the registry mutex.
///
/// This function is **O(n)** in the number of live threads.
///
/// # Safety
///
/// The supplied closure receives a mutable reference to each [`Thread`] in
/// turn, which is only valid for the duration of the call. The caller must
/// guarantee that the closure adheres to several rules:
///
/// * It must **not** panic. Panicking would leave the registry mutex poisoned.
/// * It must **not** re-enter the registry (e.g., by calling [`insert`],
///   [`remove`], or [`for_each`]). This would cause a deadlock.
/// * It must **not** allow the `&mut Thread` reference to escape its scope.
///   Using the reference after the closure returns is **undefined behaviour**.
pub unsafe fn for_each<F>(f: F)
where
    F: FnMut(&mut Thread),
{
    unsafe { THREAD_LIST.lock().for_each(f) };
}

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

/// A lazy-initialised linked list of [`Thread`]s.
struct ThreadList(Option<LinkedList<NodeAdapter>>);

impl ThreadList {
    /// Creates an uninitialised list state.
    const fn new_uninit() -> Self {
        Self(None)
    }

    /// Lazily initialises the inner list and returns a mutable reference to it.
    #[inline]
    fn get_or_init(&mut self) -> &mut LinkedList<NodeAdapter> {
        self.0
            .get_or_insert_with(|| LinkedList::new(NodeAdapter::new()))
    }

    /// Inserts a new thread into the global list.
    ///
    /// The caller must guarantee that `thread` points to a live, fully
    /// initialised [`Thread`] value that will stay alive until it is removed
    /// again with [`ThreadList::remove`].
    ///
    /// In debug builds, a check is performed to ensure that the same `Thread`
    /// is not inserted more than once. In release builds, this check is
    /// omitted; inserting a duplicate `Thread` corrupts the bookkeeping and
    /// results in **undefined behaviour**.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `thread` is already present in the list.
    ///
    /// # Safety
    ///
    /// The caller **must** uphold the following guarantees:
    /// * They hold the global mutex (`THREAD_LIST`) and therefore have
    ///   exclusive access to the underlying intrusive list for the entire
    ///   call.
    /// * `thread` refers to a live, fully-initialised [`Thread`] value that is
    ///   *not* already present in the list. In release builds, violating this
    ///   is **undefined behaviour**.
    /// * The pointed-to `Thread` will remain valid until it is later removed
    ///   via [`ThreadList::remove`].
    ///
    /// Violating the first or third condition also results in **undefined behaviour**.
    #[inline]
    unsafe fn insert(&mut self, thread: &Thread) {
        let list = self.get_or_init();

        // Check if the thread is already present in the list
        #[cfg(debug_assertions)]
        {
            use nx_svc::debug::{BreakReason, break_event};
            if list
                .iter()
                .any(|node| node.thread_ptr() == NonNull::from(thread))
            {
                // The thread is already present in the list. This is a logic
                // error.
                // TODO: Add a proper error message here.
                // panic!("Attempted to insert a duplicate thread into ThreadList");
                break_event(BreakReason::Assert, 0, 0);
            }
        }

        // SAFETY: `thread` is a non-null pointer obtained from the caller.
        // The ThreadList guard we hold ensures no other thread can concurrently
        // remove the corresponding node while we create and link it, so the
        // pointer remains valid for the duration of this call.
        let node = unsafe { Node::new(NonNull::from(thread)) };
        list.push_front(node);
    }

    /// Removes a thread from the list.
    ///
    /// This function is **O(n)** in the number of live threads.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the `thread` is not found. This usually
    /// indicates a logical error, such as removing a thread twice.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// * They still hold the global mutex (`THREAD_LIST`).
    /// * `thread` is currently present in the list (i.e. was previously
    ///   inserted and not yet removed).
    ///
    /// In release builds, if the thread is not in the list, the function will
    /// invoke `core::hint::unreachable_unchecked`, leading to **undefined
    /// behaviour**.
    #[inline]
    unsafe fn remove(&mut self, thread: &Thread) {
        let Some(list) = self.0.as_mut() else {
            // The list is not initialised yet. No-op.
            return;
        };

        let thread_ptr = NonNull::from(thread);

        let mut cursor = list.cursor_mut();
        while let Some(node_ref) = cursor.get() {
            if node_ref.thread_ptr() == thread_ptr {
                let _ = cursor.remove();
                return;
            }
            cursor.move_next();
        }

        // In release configuration reaching this point indicates a severe
        // logic error. Mark it as unreachable to allow the optimiser to
        // assume it never happens.
        #[cfg(not(debug_assertions))]
        unsafe {
            core::hint::unreachable_unchecked();
        }
        #[cfg(debug_assertions)]
        {
            use nx_svc::debug::{BreakReason, break_event};
            // Reaching here means the thread was not present in the list. This
            // is a logic error.
            // TODO: Add a proper error message here.
            // panic!("Attempted to remove a non-existent thread from ThreadList");
            break_event(BreakReason::Assert, 0, 0);
        }
    }

    /// Runs `f` on each thread currently present in the list.
    ///
    /// The list's mutex is held for the entire duration of the traversal, so
    /// the closure is executed under exclusive access to every `Thread` in
    /// turn. This function is **O(n)** in the number of live threads.
    ///
    /// # Safety
    ///
    /// * The closure must not panic; unwinding while the mutex is locked would
    ///   poison it and could leave the list in an inconsistent state.
    /// * The closure must not attempt to acquire the global mutex again (e.g.
    ///   by calling `insert`, `remove`, or another `for_each`), otherwise a
    ///   dead-lock will occur.
    /// * The closure **must not** store the `&mut Thread` beyond its
    ///   invocation; the reference is only valid while the mutex is held.
    #[inline]
    unsafe fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Thread),
    {
        let Some(list) = self.0.as_mut() else {
            // The list is not initialised yet. No-op.
            return;
        };

        let mut cursor = list.cursor_mut();
        while let Some(node_ref) = cursor.get() {
            // SAFETY: We hold the global list mutex so the node cannot be
            // removed concurrently. The pointer is therefore guaranteed to be
            // valid for the lifetime of this closure invocation.
            let thread = unsafe { &mut *node_ref.thread.as_ptr() };
            f(thread);
            cursor.move_next();
        }
    }
}

// Generate an adapter so the intrusive list knows how to reach `link` inside `Node`.
intrusive_adapter!(NodeAdapter = Box<Node>: Node { link: LinkedListLink });

/// Wrapper stored inside the intrusive linked list.
///
/// The sole purpose of this structure is to attach a [`LinkedListLink`] to the
/// raw [`Thread`] pointer without altering the ABI of `Thread` itself.
struct Node {
    /// Intrusive list link.
    link: LinkedListLink,
    /// Raw pointer to the thread information block.
    thread: NonNull<Thread>,
}

impl Node {
    /// Creates a new boxed node from a [`Thread`] pointer.
    ///
    /// # Safety
    /// The caller must guarantee that `thread` outlives the returned `Node`.
    #[inline]
    unsafe fn new(thread: NonNull<Thread>) -> Box<Self> {
        Box::new(Self {
            link: LinkedListLink::new(),
            thread,
        })
    }

    /// Returns the raw pointer to the underlying [`Thread`].
    #[inline(always)]
    fn thread_ptr(&self) -> NonNull<Thread> {
        self.thread
    }
}

// SAFETY: `Node` only contains a raw pointer and an intrusive link (which is
// essentially a couple of raw pointers as well).  Moving a `Node` between
// threads does not violate safety invariants because the pointed-to `Thread`
// object itself is never accessed concurrently without first taking
// `THREAD_LIST`'s mutex. Therefore it is safe to mark it as `Send`.
unsafe impl Send for Node {}
