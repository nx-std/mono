//! FFI bindings for the thread information API.
//!
//! This module exposes a C-compatible view of the Horizon `Thread` structure as
//! well as a helper to retrieve a pointer to the calling thread's information
//! block (equivalent to libnx's `threadGetSelf`).

use nx_svc::raw::Handle as RawHandle;

use crate::thread_impl as sys;

/// Retrieves a pointer to the calling thread's information block.
///
/// # Safety
/// 1. The returned pointer is only valid while the calling thread remains
///    alive; dereferencing it after the thread has exited results in undefined
///    behaviour.
/// 2. The caller must ensure that no mutable references coexist with shared
///    references derived from this pointer, upholding Rust's aliasing rules.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread__thread_get_self() -> *mut sys::Thread {
    sys::get_current_thread_info_ptr()
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
unsafe extern "C" fn __nx_sys_thread__thread_get_cur_handle() -> RawHandle {
    sys::get_current_thread_handle().to_raw()
}
