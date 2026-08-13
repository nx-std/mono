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
/// Initialized to the shared empty-argv backing from `nx-sys-args`;
/// [`argv::setup`] repoints it at the parsed command line.
#[unsafe(no_mangle)]
pub static mut __nx_rt_nso__libnx_system_argv: *mut *mut c_char =
    nx_sys_args::ffi::EMPTY_ARGV.as_ptr();

/// Sets up argv parsing.
///
/// Corresponds to `argvSetup()` in `argv.c`.
///
/// # Safety
///
/// Must be called after the global allocator is initialized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_argv_setup() {
    argv::setup();
    // SAFETY: the arguments were just parsed, which is what leaves something
    // to publish.
    unsafe { publish() };
}

/// Points the C-facing globals at the parsed command line.
///
/// The startup sequence calls this too. The parse and the publication are
/// separate steps so that the module holding the parsed arguments does not
/// have to reach up into this one to announce them.
///
/// # Safety
///
/// The arguments must already be parsed, which is what leaves something to
/// publish; calling it beforehand leaves the globals as they were.
pub(crate) unsafe fn publish() {
    let Some((argc, argv)) = nx_sys_args::ffi::system_argv() else {
        return;
    };

    // SAFETY: argc/argv describe the leaked argument allocation owned by
    // `nx_sys_args`, which lives for the rest of the program, and the
    // startup thread is the only one running.
    unsafe {
        __nx_rt_nso__libnx_system_argc = argc;
        __nx_rt_nso__libnx_system_argv = argv;
    }
}
