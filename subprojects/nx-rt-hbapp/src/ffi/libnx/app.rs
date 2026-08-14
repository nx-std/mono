//! Application service bring-up and teardown FFI

use crate::app;

/// Opens the services a homebrew program expects `main` to find connected.
///
/// Corresponds to `__appInit()` in `runtime/init.c`.
///
/// Upstream declares this weak so a program can replace the whole sequence.
/// Aliasing it here takes that away, because a linker-script assignment beats
/// any definition an object file carries. What a program is left with is the
/// `userAppInit` hook this calls at the end, which is what the homebrew that
/// extends startup actually reaches for; replacing the sequence outright is
/// the system-module pattern, and a system module is not an NRO.
///
/// # Panics
///
/// Panics when a service libnx treats as mandatory fails to open, which ends
/// the process.
///
/// An `__nx_*` entry point normally may not host a panic, because one crossing
/// the C boundary is undefined behaviour. Nothing crosses it here. This is the
/// port of `diagAbortWithResult`, the workspace builds with `panic = "abort"`,
/// and `nx-panic-handler` ends in `svcBreak` without ever returning: the same
/// instruction libnx's abort issues. The process therefore dies exactly where
/// it died before, carrying a message that names the failing step rather than
/// a bare result code, and no frame is left for a C caller to unwind through.
///
/// # Safety
///
/// Must be called once during initialization, on the startup thread, after the
/// heap and the main thread's TLS are set up and the command line is parsed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_app_init() {
    app::init()
}

/// Closes what [`__nx_rt_hbapp__libnx_app_init`] opened.
///
/// Corresponds to `__appExit()` in `runtime/init.c`. Weak upstream for the
/// same reason, and claimed here on the same terms, with `userAppExit` left as
/// the hook a program extends teardown through.
///
/// # Safety
///
/// Must be called once on the way out, on the thread that is exiting, with
/// every service the init sequence opened still open.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_app_exit() {
    app::exit()
}
