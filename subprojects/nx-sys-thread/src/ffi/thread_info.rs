//! FFI bindings for the thread information API.
//!
//! This module exposes a C-compatible view of the Horizon `Thread` structure as
//! well as a helper to retrieve a pointer to the calling thread's information
//! block (equivalent to libnx's `threadGetSelf`).

use core::{
    ffi::c_void,
    ptr::{self, NonNull},
};

use nx_svc::{raw::Handle as RawHandle, thread::Handle};

use crate::thread_impl as sys;

/// C-compatible representation of a Horizon thread information block.
///
/// This layout matches the `Thread` struct defined in
/// `switch/kernel/thread.h` from libnx and is guaranteed to stay in sync via
/// compile-time assertions.
/// Thread information structure
#[derive(Clone)]
#[repr(C)]
pub struct Thread {
    /// The kernel thread handle
    pub handle: RawHandle,
    /// Whether the stack memory is owned by the thread.
    pub stack_mem_owned: bool,
    /// Alignment padding
    _align: [u8; 3],
    /// Pointer to stack memory
    pub mem: *mut c_void,
    /// Pointer to stack memory mirror
    pub mirror: *mut c_void,
    /// Stack memory size
    pub size: usize,
    /// Pointer to the TLS slot array.
    pub tls_slot_array: *mut *mut c_void,
    /// Pointer to the next thread
    ///
    /// NOTE: Not used in nx-sys-thread
    next: *mut Self,
    /// Pointer to the previous thread
    ///
    /// NOTE: Not used in nx-sys-thread
    prev_next: *mut *mut Self,
}

impl From<&sys::Thread> for Thread {
    fn from(thread: &sys::Thread) -> Self {
        let (mem, mirror) = match &thread.stack_mem {
            sys::ThreadStackMem::Owned { mem, mirror, .. } => (mem.as_ptr(), mirror.as_ptr()),
            sys::ThreadStackMem::Provided { mirror, .. } => (ptr::null_mut(), mirror.as_ptr()),
        };

        Thread {
            handle: thread.handle.to_raw(),
            stack_mem_owned: thread.stack_mem.is_owned(),
            _align: [0; 3],
            mem,
            mirror,
            size: thread.stack_mem.size(),
            tls_slot_array: thread.tls_slot_ptr,
            next: ptr::null_mut(),
            prev_next: ptr::null_mut(),
        }
    }
}

impl From<&Thread> for sys::Thread {
    fn from(thread: &Thread) -> Self {
        let stack_mem = if thread.stack_mem_owned {
            #[cfg(debug_assertions)]
            {
                use nx_svc::debug::{BreakReason, break_event};
                if thread.mirror.is_null() {
                    // TODO: Provide a better error message
                    // panic!("Stack memory mirror pointer is null");
                    break_event(BreakReason::Assert, 0, 0);
                }
                if thread.mem.is_null() {
                    // TODO: Provide a better error message
                    // panic!("Stack memory pointer is null");
                    break_event(BreakReason::Assert, 0, 0);
                }
            }

            sys::ThreadStackMem::new_owned(
                unsafe { NonNull::new_unchecked(thread.mem) },
                unsafe { NonNull::new_unchecked(thread.mirror) },
                thread.size,
            )
        } else {
            #[cfg(debug_assertions)]
            {
                use nx_svc::debug::{BreakReason, break_event};
                if thread.mirror.is_null() {
                    // TODO: Provide a better error message
                    // panic!("Stack memory mirror pointer is null");
                    break_event(BreakReason::Assert, 0, 0);
                }
                if !thread.mem.is_null() {
                    // TODO: Provide a better error message
                    // panic!("Stack memory pointer is not null");
                    break_event(BreakReason::Assert, 0, 0);
                }
            }

            sys::ThreadStackMem::new_provided(
                unsafe { NonNull::new_unchecked(thread.mirror) },
                thread.size,
            )
        };

        sys::Thread {
            handle: unsafe { Handle::from_raw(thread.handle) },
            stack_mem,
            tls_slot_ptr: thread.tls_slot_array,
        }
    }
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
unsafe extern "C" fn __nx_sys_thread_get_cur_handle() -> RawHandle {
    sys::get_current_thread_handle().to_raw()
}

#[cfg(test)]
mod tests {
    use static_assertions::const_assert;

    use super::Thread;

    // Assert that the size and alignment of the `Thread` struct is correct
    const_assert!(size_of::<Thread>() == 0x38);
    const_assert!(align_of::<Thread>() == 0x8);
}
