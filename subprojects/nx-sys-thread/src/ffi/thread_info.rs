//! FFI bindings for the thread information API.
//!
//! This module exposes a C-compatible view of the Horizon `Thread` structure as
//! well as a helper to retrieve a pointer to the calling thread's information
//! block (equivalent to libnx's `threadGetSelf`).

use core::ffi::c_void;

use nx_svc::raw::Handle;

use crate::thread_impl as sys;

/// C-compatible representation of a Horizon thread information block.
///
/// This layout matches the `Thread` struct defined in
/// `switch/kernel/thread.h` from libnx and is guaranteed to stay in sync via
/// compile-time assertions.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Thread {
    /// Kernel handle identifying the thread.
    handle: Handle,
    /// Whether the stack memory has been allocated automatically.
    owns_stack_mem: bool,
    /// Explicit padding so that the next pointer field remains 8-byte aligned.
    _pad: [u8; 7],
    /// Pointer to the stack memory region.
    stack_mem: *mut c_void,
    /// Pointer to the mirrored stack memory region.
    stack_mirror: *mut c_void,
    /// Size (in bytes) of the stack.
    stack_sz: usize,
    /// Pointer to the TLS slot array.
    tls_array: *mut *mut c_void,
    /// Singly-linked list pointer to the next thread.
    next: *mut Thread,
    /// Back-pointer used to unlink the thread from the list.
    prev_next: *mut *mut Thread,
}

/// Retrieves a pointer to the calling thread's information block.
///
/// # Safety
/// 1. The returned pointer is only valid while the calling thread remains
///    alive; dereferencing it after the thread has exited results in undefined
///    behaviour.
/// 2. The caller must ensure that no mutable references coexist with shared
///    references derived from this pointer, upholding Rust's aliasing rules.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_get_self() -> *mut Thread {
    sys::get_current_thread_info_ptr() as *mut Thread
}

/// Retrieves the raw kernel handle associated with the calling thread.
///
/// This mirrors libnx's `threadGetCurHandle` and simply forwards the value
/// stored in the thread‐local storage.
///
/// # Safety
/// The returned handle is a plain value; dereferencing or otherwise using it
/// after the thread has exited is undefined behaviour, but callers typically
/// treat it as an opaque token and hand it to kernel services.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_get_cur_handle() -> Handle {
    sys::get_current_thread_handle().to_raw()
}

#[cfg(test)]
mod tests {
    use static_assertions::{const_assert, const_assert_eq};

    use super::Thread as FfiThread;
    use crate::thread_impl::Thread;

    // Assert that the C and Rust representations of a thread share the exact same
    // size and alignment so that simple pointer casts are valid.
    const_assert_eq!(size_of::<FfiThread>(), size_of::<Thread>());
    const_assert_eq!(align_of::<FfiThread>(), align_of::<Thread>());
}
