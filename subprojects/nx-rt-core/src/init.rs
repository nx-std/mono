//! Runtime initialization functions.
//!
//! Kind-agnostic startup steps every Switch executable shares. Heap
//! initialization lives here; main-thread TLS setup lives in
//! [`crate::env::main_thread`]. Each entry crate calls these from its own
//! kind-specific init sequence.
//!
//! # A step this build did not take over
//!
//! Two of the steps below sit behind the `sys-virtmem` and `sys-thread`
//! features, so a build with the matching Meson option off reaches them with
//! the crate absent. Such a step calls the C entry point by name rather than
//! skipping: skipping would leave the work undone, and with the feature off
//! that name can only be the C implementation, because the linker fragment
//! that would redirect it is added by the same Meson option that sets the
//! feature.

use crate::env::heap_override;

/// Initialize the allocator heap.
///
/// Uses heap override from loader config if available, otherwise allocates via SVC.
pub fn setup_heap() {
    match heap_override() {
        Some((addr, size)) => {
            // SAFETY: The loader guarantees this region is valid and owned by us.
            unsafe { nx_alloc::global::init_with_heap_override(addr, size) };
        }
        None => {
            nx_alloc::global::init();
        }
    }
}

/// Initializes the reservation map that address-space lookups are served from.
///
/// Reads no kind-specific fact, so every output kind runs the same one.
pub fn setup_virtmem() {
    cfg_select! {
        feature = "sys-virtmem" => {
            nx_sys_virtmem::virtmem::lock().init();
        }
        _ => {
            unsafe extern "C" {
                fn virtmemSetup();
            }

            // SAFETY: no other thread exists yet, which is the only thing this
            // step requires.
            unsafe { virtmemSetup() };
        }
    }
}

/// Fills in the thread bookkeeping for the thread the process started on.
///
/// Runs after the heap, so the bookkeeping it registers has somewhere to live.
/// Reads no kind-specific fact: which thread the process started on is a
/// kernel argument every output kind receives.
pub fn init_main_thread() {
    cfg_select! {
        feature = "sys-thread" => {
            // SAFETY: this is the main thread, the heap is up, and no other
            // thread API has run yet.
            unsafe { nx_sys_thread::thread::init_main_thread() };
        }
        _ => {
            unsafe extern "C" {
                fn __libnx_init_thread();
            }

            // SAFETY: this is the main thread, the heap is up, and no other
            // thread API has run yet.
            unsafe { __libnx_init_thread() };
        }
    }
}
