//! `romfsMountSelf` for a homebrew `NRO`.
//!
//! The one romfs entry point that cannot live in `nx-romfs`, because it is the one that has to know
//! what kind of program is asking. Here the answer is settled: an `NRO` carries its image inside
//! itself, so this reads it out of the file the loader launched.
//!
//! `nx-rt-nso` defines the same C name over its own source, and exactly one of the two entry crates
//! is in any link, so the two definitions never meet.

use core::ffi::{
    CStr,
    c_char,
};

use nx_rt_core::error::{
    LibnxError,
    ToResultCode,
    libnx_error,
};

use crate::romfs;

/// Mounts this program's own image under `name`.
///
/// Corresponds to `romfsMountSelf()` in libnx, and so to `romfsInit()`, which the header defines as
/// a call to it with the default name.
///
/// # Safety
///
/// `name` must be a NUL-terminated string. The device the program's own file lives on must be
/// mounted, which for homebrew means the SD card, and the `fsp-srv` session must be installed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_romfs_mount_self(name: *const c_char) -> u32 {
    if name.is_null() {
        return libnx_error(LibnxError::BadInput);
    }

    // SAFETY: the caller guarantees a NUL-terminated string.
    let Ok(name) = core::str::from_utf8(unsafe { CStr::from_ptr(name) }.to_bytes()) else {
        // A device registers under a name that is text, so a name that is not matches nothing.
        return libnx_error(LibnxError::BadInput);
    };

    match romfs::mount_self(name) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}
