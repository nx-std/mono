//! ThreadVars - Thread Variables Structure and Operations
//!
//! This module provides the `ThreadVars` structure and functions for accessing
//! thread-specific variables stored in the Thread-local Region (TLR).

use core::{ffi::c_void, ptr};

use super::tls::get_tlr_ptr;

// TODO: Import from nx-svc
pub type Handle = u32;

/// Size of the ThreadVars structure  
///
/// The [`ThreadVars`] structure is exactly 32 bytes (0x20) long and is stored at the end
/// of the thread's TLS segment within the Thread Local Region.
pub const THREAD_VARS_SIZE: usize = 0x20;

/// Size of the Thread Local Region (TLR)
pub const TLR_SIZE: usize = 0x200;

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

    /// Thread handle, for mutexes
    pub handle: Handle,

    /// Pointer to the current thread (if exists)
    pub thread_ptr: *mut c_void,

    /// Pointer to this thread's newlib state
    pub reent: *mut c_void,

    /// Pointer to this thread's thread-local segment
    // Offset must be TLS+0x1F8 for __aarch64_read_tp
    pub tls_tp: *mut c_void,
}

/// Returns a mutable reference to the `ThreadVars` structure for the current thread.
#[inline]
pub fn get_thread_vars() -> *mut ThreadVars {
    unsafe { get_tlr_ptr().add(TLR_SIZE - THREAD_VARS_SIZE) as *mut ThreadVars }
}

/// Returns the current thread's handle.
///
/// Get the `Handle` for the current thread from the Thread-Local Region (TLR).
pub fn get_current_thread_handle() -> Handle {
    unsafe {
        let thread_vars = get_thread_vars();
        ptr::read_volatile(ptr::addr_of!((*thread_vars).handle))
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::const_assert_eq;

    use super::{THREAD_VARS_SIZE, ThreadVars};

    // Assert that the size of the `ThreadVars` struct is 0x20 bytes
    const_assert_eq!(size_of::<ThreadVars>(), THREAD_VARS_SIZE);
}
