//! FFI bindings for the `nx-sys-thread` crate
//!
//! # References
//! - [switchbrew/libnx: switch/arm/tls.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/tls.h)
//! - [switchbrew/libnx: internal.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/internal.h)

use core::ffi::c_void;

use crate::tls::{self, ThreadVars};

/// Gets the thread-local storage (TLS) buffer.
///
/// This function reads the `tpidrro_el0` system register, which holds the
/// read-only thread pointer for the current thread.
///
/// Returns a pointer to the thread-local storage buffer.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_get_ptr() -> *mut c_void {
    tls::get_ptr()
}

/// Returns a mutable reference to the `ThreadVars` structure for the current thread.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_get_thread_vars() -> *mut ThreadVars {
    tls::thread_vars_ptr()
}

/// Returns the start offset (in bytes) of the initialised TLS data (`.tdata`/`.tbss`) within a
/// thread's TLS block. Mirrors the behaviour of `getTlsStartOffset()` from the original C
/// implementation.
#[inline]
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_get_tls_start_offset() -> usize {
    tls::static_tls_data_start_offset()
}
