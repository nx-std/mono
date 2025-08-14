use core::ptr;

use nx_svc::thread::Handle;

use super::stackmem::{PageAlignedBuffer, ThreadStackMem};
use crate::tls_region;

/// Thread information structure
pub struct Thread<S = PageAlignedBuffer> {
    /// The kernel thread handle
    pub handle: Handle,

    /// Stack memory information.
    pub stack_mem: ThreadStackMem<S>,
    // TODO: Add support for dynamic TLS slots
    // Add field: pub tls_slots: Option<Slots>
}

// TODO: Add support for dynamic TLS slots methods
// Implement:
// - slot_get(&self, mod_id: usize) -> Result<*mut c_void, SlotGetError>
// - slot_set(&mut self, mod_id: usize, value: *mut c_void) -> Result<(), SlotSetError>

/// Returns a raw pointer to the [`Thread`] information structure representing the
/// calling thread.
///
/// This is the Rust counterpart of libnx's `threadGetSelf` declared in
/// `switch/kernel/thread.h` and provides direct access to the per-thread data that
/// Horizon keeps in Thread Local Storage (TLS).
///
/// # Returns
/// A mutable raw pointer to the current thread's [`Thread`] structure. The
/// structure lives inside the TLS block of the running thread and remains valid
/// for the entire lifetime of that thread.
///
/// # Safety
/// * The returned pointer is only meaningful while the thread is alive; it must
///   not be dereferenced after the thread has exited.
/// * Using the pointer concurrently from multiple contexts without proper
///   synchronisation can lead to undefined behaviour because the kernel may
///   mutate some of the fields.
/// * The caller is responsible for ensuring that aliasing rules are not
///   violated when creating references from the raw pointer.
pub fn get_current_thread_info_ptr() -> *mut Thread {
    let tv_ptr = tls_region::thread_vars_ptr();

    // SAFETY: The current thread's information is stored in the TLS.
    // Use `read_volatile` to avoid the compiler re-ordering or eliminating the read.
    unsafe { ptr::read_volatile(&raw const (*tv_ptr).thread_info_ptr) }.cast()
}

/// Returns the [`Handle`] of the calling thread.
///
/// This is the Rust counterpart of libnx's `threadGetCurHandle` declared in
/// `switch/kernel/thread.h` and provides direct access to the raw kernel
/// handle associated with the running thread.
///
/// # Returns
/// The [`Handle`] identifying the current thread. The handle is managed by the
/// kernel and remains valid for the entire lifetime of the thread.
///
/// # Safety
/// This function is intrinsically safe because it only reads the handle value
/// stored in the thread's TLS block and returns a copy. No shared mutable
/// state is accessed and no invariants can be violated.
pub fn get_current_thread_handle() -> Handle {
    let tv_ptr = tls_region::thread_vars_ptr();

    // SAFETY: The current thread's handle is stored in the TLS.
    // Use `read_volatile` to avoid the compiler re-ordering or eliminating the read.
    unsafe { ptr::read_volatile(&raw const (*tv_ptr).handle) }
}
