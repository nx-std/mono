//! ThreadVars - Thread Variables Structure and Operations
//!
//! This module provides the `ThreadVars` structure and functions for accessing
//! thread-specific variables stored in the Thread-local Region (TLR).

use core::{ffi::c_void, ptr};

use nx_svc::thread::Handle;

use crate::tls::{THREAD_VARS_SIZE, TLS_SIZE, get_tls_ptr};

/// Magic value used to check if the ThreadVars struct is initialized
pub const THREAD_VARS_MAGIC: u32 = 0x21545624; // ASCII: "!TV$"

/// Thread vars structure
///
/// This structure is stored at the end of the thread's TLS segment.
///
/// It is exactly [`THREAD_VARS_SIZE`] bytes long (0x20 bytes).
#[derive(Debug)]
#[repr(C)]
pub struct ThreadVars {
    /// Magic value used to check if the struct is initialized
    pub magic: u32,

    /// Thread handle
    pub handle: Handle,

    /// Pointer to the current thread (if exists)
    pub thread_ptr: *mut c_void,

    /// Pointer to this thread's newlib state
    pub reent: *mut c_void,

    /// Pointer to this thread's thread-local segment
    // Offset must be TLS+0x1F8
    pub tls_tp: *mut c_void,
}

/// Returns a mutable reference to the `ThreadVars` structure for the current thread.
#[inline]
pub fn thread_vars_ptr() -> *mut ThreadVars {
    let tls_ptr = get_tls_ptr();
    unsafe { tls_ptr.add(TLS_SIZE - THREAD_VARS_SIZE) as *mut ThreadVars }
}

/// Returns the current thread's handle.
///
/// Get the `Handle` for the current thread from the Thread-Local Storage (TLS) region.
#[inline]
pub fn get_current_thread_handle() -> Handle {
    let thread_vars = thread_vars_ptr();
    unsafe { ptr::read_volatile(&raw const (*thread_vars).handle) }
}

#[cfg(test)]
mod tests {
    use static_assertions::const_assert_eq;

    use super::{THREAD_VARS_SIZE, ThreadVars};

    // Assert that the size of the `ThreadVars` struct is 0x20 bytes
    const_assert_eq!(size_of::<ThreadVars>(), THREAD_VARS_SIZE);
}
