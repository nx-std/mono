//! ThreadVars

use core::{ffi::c_void, ptr};

use nx_cpu::tls::__nx_cpu_get_tls;
use static_assertions::const_assert_eq;
pub type Handle = u32;

pub const THREADVARS_MAGIC: u32 = 0x21545624; // ASCII: !TV$

/// Size of the Thread-Local Region segment
///
/// ## References
/// - [Switchbrew Wiki: Thread-Local Region](https://switchbrew.org/wiki/Thread_Local_Region)
const TLR_SIZE: usize = 0x200;

/// Size of the `ThreadVars` structure
const THREAD_VARS_SIZE: usize = 0x20;

/// ThreadVars structure
///
/// This structure is stored at the end of the thread's TLS segment.
///
/// It is exactly [`THREAD_VARS_SIZE`] bytes long (0x20 bytes).
#[repr(C)]
#[derive(Debug)]
pub struct ThreadVars {
    /// Magic value used to check if the struct is initialized
    magic: u32,

    /// Thread handle, for mutexes
    handle: Handle,

    /// Pointer to the current thread (if exists)
    thread_ptr: *mut c_void,

    /// Pointer to this thread's newlib state
    reent: *mut c_void,

    /// Pointer to this thread's thread-local segment
    // Offset must be TLS+0x1F8 for __aarch64_read_tp
    tls_tp: *mut c_void,
}

// Assert that the size of the `ThreadVars` struct is 0x20 bytes
const_assert_eq!(size_of::<ThreadVars>(), THREAD_VARS_SIZE);

/// Returns a mutable reference to the `ThreadVars` structure for the current thread.
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_thread_get_thread_vars() -> *mut ThreadVars {
    unsafe {
        let tls = __nx_cpu_get_tls();
        tls.add(TLR_SIZE - THREAD_VARS_SIZE) as *mut ThreadVars
    }
}

/// Returns the current thread's handle.
///
/// Get the `Handle` for the current thread from the TLR.
///
/// The thread handle is used for mutexes.
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_thread_get_current_thread_handle() -> Handle {
    unsafe {
        // Calculate the address of the thread handle: TLS + 0x1E4
        let tls = __nx_cpu_get_tls();
        let handle_ptr = tls.add(TLR_SIZE - THREAD_VARS_SIZE + 4) as *const Handle;

        ptr::read_volatile(handle_ptr)
    }
}
