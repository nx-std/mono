//! FFI bindings for the `nx-sys-thread` crate
//!
//! # References
//! - [switchbrew/libnx: switch/arm/tls.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/tls.h)

use core::ffi::c_void;

use crate::{
    thread_vars::{self, Handle, ThreadVars},
    tls,
};

//<editor-fold desc="switch/arm/tls.h">

/// Gets the thread-local storage (TLS) buffer.
///
/// This function reads the `tpidrro_el0` system register, which holds the
/// read-only thread pointer for the current thread.
///
/// Returns a pointer to the thread-local storage buffer.
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_get_ptr() -> *mut c_void {
    tls::get_tlr_ptr()
}

//</editor-fold>

//<editor-fold desc="source/internal.h">

/// Returns a mutable reference to the `ThreadVars` structure for the current thread.
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_get_thread_vars() -> *mut ThreadVars {
    thread_vars::get_thread_vars()
}

/// Returns the current thread's handle.
///
/// Get the `Handle` for the current thread from the TLR.
///
/// The thread handle is used for mutexes.
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_get_current_thread_handle() -> Handle {
    thread_vars::get_current_thread_handle()
}

//</editor-fold>
