//! Argument vector FFI

use core::ffi::c_char;

use crate::argv;

/// FFI-exported argc (for C code compatibility)
#[unsafe(no_mangle)]
pub static mut __nx_rt_nro__libnx_system_argc: i32 = 0;

/// FFI-exported argv (for C code compatibility)
///
/// Initialized to the shared empty-argv backing from `nx-rt-core`; [`argv::setup`]
/// repoints it at the parsed command line.
#[unsafe(no_mangle)]
pub static mut __nx_rt_nro__libnx_system_argv: *mut *mut c_char =
    nx_rt_core::argv::EMPTY_ARGV.as_ptr();

/// Setup argv parsing.
///
/// Corresponds to `argvSetup()` in `argv.c`.
///
/// # Safety
///
/// Must be called after the global allocator is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_argv_setup() {
    unsafe { argv::setup() }
}

/// Set the C-style argc/argv globals
///
/// # Safety
///
/// Only called from argv::setup() with valid argc/argv pointers
pub(crate) unsafe fn set_system_argv(argc: i32, argv: *mut *mut c_char) {
    unsafe {
        __nx_rt_nro__libnx_system_argc = argc;
        __nx_rt_nro__libnx_system_argv = argv;
    }
}
