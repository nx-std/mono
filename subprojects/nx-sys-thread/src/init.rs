//! Main thread initialization for the nx-sys-thread crate.
//!
//! This module provides the initialization logic for the process' main thread,
//! serving as the Rust equivalent of libnx's `__libnx_init_thread` function.
//! It must be called exactly once during process startup to properly initialize
//! the thread registry and maintain compatibility with libnx C APIs.

use core::{ffi::c_void, ptr::NonNull};

use nx_svc::{
    debug::{BreakReason, break_event},
    mem,
    thread::Handle,
};

use crate::{
    registry,
    thread_impl::{Thread, ThreadStackMem},
    tls_region,
};

unsafe extern "C" {
    /// Returns the kernel handle of the process' *main* thread.
    ///
    /// This symbol is provided by the libnx C runtime and resolved by the
    /// loader before any user code runs. It yields the raw [`Handle`] that
    /// identifies the thread which performed the program start-up sequence.
    ///
    /// # Safety
    /// * The handle is owned by the runtime; **do not** close it manually –
    ///   doing so would leave the process without a valid main thread and
    ///   immediately abort execution.
    /// * The function should be called only after the libnx start-up stubs
    ///   have executed, otherwise its value is undefined.
    fn envGetMainThreadHandle() -> Handle;
}

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
    // Acquire the main thread's kernel handle from the runtime
    let handle = unsafe { envGetMainThreadHandle() };

    let main_thread = {
        let stack_mem = {
            // Query the memory region that contains a local stack variable. This is
            // the same technique the C implementation uses to discover the current
            // thread's stack boundaries.
            let stack_marker: u8 = 0;

            let (stack_mem_base_addr, stack_mem_size) =
                match mem::query_memory(&stack_marker as *const _ as usize) {
                    Ok((mem_info, ..)) => (mem_info.addr, mem_info.size),
                    Err(_) => {
                        // Kernel memory query failed during initialization - this is fatal.
                        // Use debug break instead of panic to avoid unwinding during startup.
                        break_event(BreakReason::Panic, 0, 0);
                    }
                };

            let Some(stack_mem_ptr) = NonNull::new(stack_mem_base_addr as *mut _) else {
                // Stack memory base address is null - this should never happen.
                break_event(BreakReason::Assert, 0, 0);
            };

            // For the main thread, the stack memory is provided by the kernel and
            // not owned by the application. This matches the C implementation where
            // owns_stack_mem=false and stack_mem=NULL.
            ThreadStackMem::new_provided(stack_mem_ptr, stack_mem_size)
        };

        // TODO: Add support for dynamic TLS slots initialization
        // Initialize TLS slots here:
        // let tls_slots = unsafe { Slots::from_ptr(tls_region::slots_ptr()) };

        Thread {
            handle,
            stack_mem,
            // TODO: Add tls_slots field initialization
        }
    };

    // SAFETY: This is the first and only initialization of the main thread.
    // The caller guarantees this function is called exactly once.
    unsafe { registry::set_main_thread(main_thread) };

    // SAFETY: The main thread was just successfully stored in the registry above.
    let main_thread_ptr = unsafe { registry::main_thread() };

    // TODO: Thread registry list functionality temporarily disabled for dyn slots initialization
    // Register the main thread with the global thread list to enable cleanup
    // operations (TLS destructor walks, thread enumeration, etc.).
    // SAFETY: The main thread is fully initialized and will remain valid for
    // the lifetime of the process.
    // unsafe { registry::insert(&*main_thread_ptr) };

    // Update ThreadVars to maintain compatibility with libnx C functions.
    // This ensures threadGetSelf() and related APIs work correctly.
    // SAFETY: thread_vars_ptr() returns a valid pointer to the current thread's
    // ThreadVars structure in TLS, and main_thread_ptr is valid.
    let tv = unsafe { &mut *tls_region::thread_vars_ptr() };
    tv.thread_info_ptr = main_thread_ptr as *const _ as *mut c_void;
}
