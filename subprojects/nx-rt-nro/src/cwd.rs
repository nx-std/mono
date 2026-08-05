//! Startup working directory (NRO)
//!
//! Ports `libnx`'s `__libnx_init_cwd` for the homebrew NRO output kind. The
//! homebrew loader launches an NRO by its full path, so `argv[0]` names the
//! file the program was loaded from, and changing into its directory is what
//! lets a program reach the files shipped beside it by relative path.
//!
//! libnx keeps this next to the `fsdev` devoptab layer and guards it with
//! `envIsNso()`, because that translation unit serves every output kind. Here
//! the guard is gone: this crate is the NRO entry crate, so the branch could
//! only ever go one way.
//!
//! The directory change itself goes through the C standard library rather than
//! a device directly. `chdir` resolves the `"name:"` prefix to a registered
//! device, asks that device to move its working directory, and only then
//! records the path and makes the device the default one. Reaching past it
//! would set the device's directory without any of that bookkeeping.

use alloc::ffi::CString;
use core::ffi::{
    c_char,
    c_int,
};

use crate::argv;

/// Changes into the directory the program was loaded from.
///
/// Does nothing when the loader passed no command line, or when `argv[0]`
/// names no directory to change into.
pub fn init() {
    unsafe extern "C" {
        // Provided by newlib's `libsysbase`, which dispatches to the device the
        // path resolves to.
        fn chdir(path: *const c_char) -> c_int;
    }

    let Some(program) = argv::args().next() else {
        return;
    };

    // Everything before the last separator is the directory the program sits
    // in: `sdmc:/switch/app.nro` changes into `sdmc:/switch`. A path with no
    // separator names no directory, so there is nothing to change into.
    let Some(separator) = program.rfind('/') else {
        return;
    };
    let Ok(directory) = CString::new(&program[..separator]) else {
        return;
    };

    // The result is discarded because there is nothing here to report it to:
    // this runs before `main`, and a failure leaves the working directory
    // where it was. A program that then opens a relative path fails that open
    // and reports it itself. libnx ignores the result at this point too.
    // SAFETY: `directory` owns a live nul-terminated string for the call.
    let _ = unsafe { chdir(directory.as_ptr()) };
}
