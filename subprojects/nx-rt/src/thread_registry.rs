//! # Thread registry and initialization
//!
//! This module provides the process-wide thread registry and initialization logic
//! for the main thread. It serves as the Rust equivalent of libnx's thread
//! initialization functions.
//!
//! ## Registry Design
//!
//! The registry owns a global collection that tracks every [`Thread`] alive in
//! the current process.
//!
//! • **Storage** – Threads are _not_ inserted directly into the intrusive list
//!   because that would change the C ABI of [`Thread`]. Instead, every entry
//!   is a heap-allocated `Box<Node>` that contains the intrusive
//!   [`LinkedListLink`] and a raw, non-null pointer back to the real [`Thread`]
//!   object. The allocation is created on insertion and destroyed immediately
//!   after removal.
//!
//! • **Global access** – A `static` `Mutex<ThreadList>` called
//!   `THREAD_LIST` serializes all mutations.
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
//! [`LinkedListLink`]: intrusive_collections::LinkedListLink

use core::ptr::{self, NonNull};

use nx_std_sync::once_lock::OnceLock;
use nx_svc::mem;
use nx_sys_mem::buf::BufferRef;
use nx_sys_thread::{Thread, ThreadStackMem};
use nx_sys_thread_tls as tls_region;

use crate::env;

/// The main thread
///
/// This is initialized when libnx runtime is initialized.
static MAIN_THREAD: OnceLock<MainThread> = OnceLock::new();

/// Sets the global record for the process' main [`Thread`].
///
/// This function **must** be invoked exactly once during program start-up,
/// typically by the runtime right after the main thread has been fully
/// initialized but **before** any additional threads are spawned or the
/// thread registry is otherwise accessed.
///
/// # Panics
///
/// Panics if this function is called more than once.
///
/// # Safety
///
/// The caller must uphold the following guarantees:
///   * `thread` refers to the currently executing main thread and is **fully**
///     initialized.
///   * The provided [`Thread`] value lives for the entire lifetime of the
///     process (it is stored globally and later returned by [`main_thread`]).
pub unsafe fn set_main_thread(thread: Thread<BufferRef<'static>>) {
    if MAIN_THREAD.set(MainThread(thread)).is_err() {
        panic!("Main thread already set: MAIN_THREAD_ALREADY_SET");
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
pub unsafe fn main_thread() -> &'static MainThread {
    MAIN_THREAD
        .get()
        .expect("Main thread not set: MAIN_THREAD_NOT_SET")
}

// TODO: Thread registry list functionality temporarily disabled for dyn slots initialization
/// Initializes the process-wide representation of the *main* thread.
///
/// This function performs the Rust equivalent of libnx's `__libnx_init_thread`.
/// It must be invoked exactly **once** during start-up, before any other call
/// into the `nx-sys-thread` crate that relies on the global thread registry or
/// per-thread TLS helpers.
///
/// ## Initialization Steps
///
/// The function performs the following operations in sequence:
///
/// 1. **Retrieve main thread handle** – Calls `envGetMainThreadHandle()` to get
///    the kernel handle for the currently executing thread.
///
/// 2. **Discover stack boundaries** – Uses `svcQueryMemory` on a local variable
///    to determine the stack region's base address and size. For the main thread,
///    this memory is provided by the kernel and not owned by the application.
///
/// 3. **Calculate TLS slot array pointer** – Computes the address of the dynamic
///    TLS slot array at `TPIDRRO_EL0 + 0x108`.
///
/// 4. **Create and store Thread structure** – Assembles a fully-initialized
///    [`Thread`] instance and stores it in the global main thread registry.
///
/// 5. **Register with thread list** – Adds the main thread to the process-wide
///    thread registry for cleanup operations and enumeration APIs.
///
/// 6. **Update ThreadVars** – Sets `ThreadVars.thread_info_ptr` to ensure
///    compatibility with libnx functions like `threadGetSelf()`.
///
/// ## Error Handling
///
/// Kernel service failures (such as `svcQueryMemory` errors) trigger debug
/// break events instead of returning error codes. This matches the behavior
/// expected during process initialization where such failures are fatal.
///
/// # Safety
///
/// This function is `unsafe` because it performs low-level system initialization
/// and the caller must guarantee:
///
/// * **Single initialization** – Must be called exactly once per process. Multiple
///   calls result in undefined behavior due to registry corruption and aliasing.
///
/// * **Correct runtime state** – The libnx C runtime must be properly initialized:
///   - Thread-Local Storage (TLS) must be in a valid state
///   - `envGetMainThreadHandle()` must return a valid handle
///   - The calling thread must be the actual main thread
///
/// * **Execution context** – Must be called from the main thread during process
///   startup, before any additional threads are created or other `nx-sys-thread`
///   APIs are used.
///
/// * **Memory safety** – The function performs raw pointer operations and memory
///   queries that assume a valid process memory layout.
///
/// Violating these requirements may result in process termination, memory
/// corruption, or undefined behavior.
pub unsafe fn init_main_thread() {
    let thread = {
        let stack_mem = {
            // Query the memory region that contains a local stack variable. This is
            // the same technique the C implementation uses to discover the current
            // thread's stack boundaries.
            let stack_marker: u8 = 0;

            let (stack_mem_base_addr, stack_mem_size) = mem::query_memory(
                &stack_marker as *const _ as usize,
            )
            .map(|(mem_info, ..)| (mem_info.addr, mem_info.size))
            .expect("Kernel memory query failed during initialization: INIT_MEMORY_QUERY_FAILED");

            let stack_mem_ptr = NonNull::new(stack_mem_base_addr as *mut _)
                .expect("Stack memory base address is null: NULL_STACK_BASE_ADDRESS");

            // For the main thread, the stack memory is provided by the kernel and
            // not owned by the application
            unsafe { ThreadStackMem::from_provided(stack_mem_ptr, stack_mem_size) }
        };

        // TODO: Add support for dynamic TLS slots initialization
        // Initialize TLS slots here:
        // let tls_slots = unsafe { Slots::from_ptr(tls_region::slots_ptr()) };

        Thread {
            handle: env::main_thread_handle(),
            stack_mem,
            // TODO: Add tls_slots field initialization
        }
    };

    // SAFETY: This is the first and only initialization of the main thread.
    // The caller guarantees this function is called exactly once.
    unsafe { set_main_thread(thread) };

    // SAFETY: The main thread was just successfully stored in the registry above.
    let main_thread_ptr = unsafe { main_thread() };

    // TODO: Thread registry list functionality temporarily disabled for dyn slots initialization
    // Register the main thread with the global thread list to enable cleanup
    // operations (TLS destructor walks, thread enumeration, etc.).
    // SAFETY: The main thread is fully initialized and will remain valid for
    // the lifetime of the process.
    // unsafe { insert(&*main_thread_ptr) };

    // Update ThreadVars to maintain compatibility with libnx C functions.
    // This ensures threadGetSelf() and related APIs work correctly.
    // SAFETY: main_thread_ptr is valid for the lifetime of the process.
    unsafe { tls_region::set_thread_info_ptr(ptr::from_ref(main_thread_ptr).cast_mut()) }
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
///   reference. Creating aliasing references is **undefined behavior**.
pub unsafe fn main_thread_ptr() -> NonNull<MainThread> {
    let thread = MAIN_THREAD
        .get()
        .expect("Main thread not set: MAIN_THREAD_NOT_SET");

    NonNull::from(thread)
}

/// A _new-type_ wrapper around `Thread` to safely mark it as Send + Sync.
///
/// This is safe specifically for the main thread because it's initialized
/// once and then treated as read-only for the lifetime of the process.
pub struct MainThread(Thread<BufferRef<'static>>);

impl core::ops::Deref for MainThread {
    type Target = Thread<BufferRef<'static>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// SAFETY: The main thread info is initialized once at startup and then
// becomes effectively read-only. The OnceLock ensures safe initialization.
// Access from other threads is safe because the data doesn't change.
unsafe impl Send for MainThread {}
// SAFETY: The inner `Thread` is read-only after initialization; no interior
// mutability exists, so concurrent `&MainThread` references are data-race free.
unsafe impl Sync for MainThread {}
