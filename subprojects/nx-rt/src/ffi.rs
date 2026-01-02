//! FFI exports for libnx runtime functions

use core::ffi::c_char;

use crate::argv;

/// Wrapper for static argv array to implement Sync
struct StaticArgv([*mut c_char; 1]);

// SAFETY: This is a null-terminated pointer array. While the pointers themselves
// aren't Sync, the array is static and immutable (contains only null pointers).
unsafe impl Sync for StaticArgv {}

/// Empty argv - null-terminated array with zero arguments
static EMPTY_ARGV: StaticArgv = StaticArgv([core::ptr::null_mut()]);

/// FFI-exported argc (for C code compatibility)
#[unsafe(no_mangle)]
pub static mut __nx_rt__system_argc: i32 = 0;

/// FFI-exported argv (for C code compatibility)
#[unsafe(no_mangle)]
pub static mut __nx_rt__system_argv: *mut *mut c_char = EMPTY_ARGV.0.as_ptr() as *mut _;

/// Setup argv parsing
///
/// # Safety
///
/// Must be called after the global allocator is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__argv_setup() {
    unsafe { argv::setup() }
}

/// Set the C-style argc/argv globals
///
/// # Safety
///
/// Only called from argv::setup() with valid argc/argv pointers
pub(crate) unsafe fn set_system_argv(argc: i32, argv: *mut *mut c_char) {
    unsafe {
        __nx_rt__system_argc = argc;
        __nx_rt__system_argv = argv;
    }
}

/// nxlink host address (C-compatible, network byte order)
///
/// This corresponds to `struct in_addr __nxlink_host` in libnx.
#[unsafe(no_mangle)]
pub static mut __nx_rt__nxlink_host: u32 = 0;

/// Set the nxlink host address
///
/// Called from nxlink::strip_nxlink_suffix() when the _NXLINK_ suffix is detected.
pub(crate) fn set_nxlink_host(addr: u32) {
    unsafe {
        __nx_rt__nxlink_host = addr;
    }
}

/// Get the nxlink host address
///
/// Returns None if no nxlink host was detected.
pub(crate) fn get_nxlink_host() -> Option<u32> {
    let addr = unsafe { __nx_rt__nxlink_host };
    if addr != 0 { Some(addr) } else { None }
}
