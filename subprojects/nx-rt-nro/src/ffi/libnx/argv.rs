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
    // SAFETY: the caller guarantees the allocator is up, which is what the
    // parse and the publication below both require.
    unsafe {
        let nxlink_host = argv::setup();
        publish(nxlink_host);
    }
}

/// Points the C-facing globals at the parsed command line.
///
/// The startup sequence calls this too. The parse and the publication are
/// separate steps so that the module holding the parsed arguments does not
/// have to reach up into this one to announce them.
///
/// `nxlink_host` travels with the arguments because that is where it arrived:
/// nxlink appends it to the argument string, and stripping it out is part of
/// the same parse.
///
/// # Safety
///
/// The arguments must already be parsed, which is what leaves something to
/// publish; calling it beforehand leaves the globals as they were.
pub(crate) unsafe fn publish(nxlink_host: Option<u32>) {
    if let Some((argc, argv)) = nx_rt_core::argv::system_argv() {
        // SAFETY: argc/argv describe the leaked argument allocation owned by
        // `nx_rt_core::argv`, which lives for the rest of the program, and the
        // startup thread is the only one running.
        unsafe {
            __nx_rt_nro__libnx_system_argc = argc;
            __nx_rt_nro__libnx_system_argv = argv;
        }
    }

    if let Some(host) = nxlink_host {
        super::env::set_nxlink_host(host);
    }
}
