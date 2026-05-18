//! Argument vector FFI (NSO).
//!
//! Redirects `libnx`'s `argvSetup` and exposes the C-style `__system_argc` /
//! `__system_argv` globals for C consumers. The arguments themselves are read
//! from the `__argdata__` region by [`crate::argv`].

use core::ffi::c_char;

use crate::argv;

/// FFI-exported argc (for C consumers).
#[unsafe(no_mangle)]
pub static mut __nx_rt_nso__libnx_system_argc: i32 = 0;

/// FFI-exported argv (for C consumers).
///
/// Initialized to the shared empty-argv backing from `nx-rt-core`; [`argv::setup`]
/// repoints it at the parsed command line.
#[unsafe(no_mangle)]
pub static mut __nx_rt_nso__libnx_system_argv: *mut *mut c_char =
    nx_rt_core::argv::EMPTY_ARGV.as_ptr();

/// Sets up argv parsing.
///
/// Corresponds to `argvSetup()` in `argv.c`.
///
/// # Safety
///
/// Must be called after the global allocator is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_argv_setup() {
    unsafe { argv::setup() }
}

/// Publishes the C-style argc/argv globals.
///
/// # Safety
///
/// Only called from `argv::setup()` with `argc`/`argv` describing the leaked
/// argument allocation owned by `nx_rt_core::argv`.
pub(crate) unsafe fn set_system_argv(argc: i32, argv: *mut *mut c_char) {
    unsafe {
        __nx_rt_nso__libnx_system_argc = argc;
        __nx_rt_nso__libnx_system_argv = argv;
    }
}
