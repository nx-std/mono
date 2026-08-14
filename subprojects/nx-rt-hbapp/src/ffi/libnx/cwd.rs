//! Startup working directory FFI

use crate::cwd;

/// Changes into the directory the program was loaded from.
///
/// Corresponds to `__libnx_init_cwd()` in `devices/fs_dev.c`.
///
/// # Safety
///
/// Must be called during initialization, after `argvSetup` has parsed the
/// command line and the device the program was loaded from has been mounted.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_init_cwd() {
    cwd::init()
}
