//! C FFI bindings for nx-sys-thread-tls.

use core::ffi::c_void;

use crate::ThreadVars;

/// C FFI: Returns a raw pointer to the Thread Local Storage (TLS) block.
///
/// This is the Rust equivalent of libnx's `armGetTls()` function.
///
/// # Safety
///
/// The returned pointer is valid for the lifetime of the current thread.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_tls__arm_get_tls() -> *mut c_void {
    crate::get_ptr().cast()
}

/// C FFI: Returns a pointer to the current thread's `ThreadVars` structure.
///
/// This is the Rust equivalent of libnx's `getThreadVars()` function.
///
/// # Safety
///
/// The returned pointer is valid for the lifetime of the current thread.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_tls__get_thread_vars() -> *mut ThreadVars {
    crate::thread_vars_ptr()
}
