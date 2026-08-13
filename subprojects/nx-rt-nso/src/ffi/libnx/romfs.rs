//! `romfsMountSelf` for an `NSO`.
//!
//! The one romfs entry point that cannot live in `nx-romfs`, because it is the one that has to know
//! what kind of program is asking. Here the answer is settled: a packaged program's data is a
//! partition the filesystem service hands out, so this mounts that and nothing else.
//!
//! `nx-rt-nro` defines the same C name over its own source, and exactly one of the two entry crates
//! is in any link, so the two definitions never meet.

use core::ffi::{
    CStr,
    c_char,
};

use nx_rt_core::error::{
    LibnxError,
    libnx_error,
};

/// Mounts this program's own data partition under `name`.
///
/// Corresponds to `romfsMountSelf()` in libnx, and so to `romfsInit()`, which the header defines as
/// a call to it with the default name.
///
/// # Safety
///
/// `name` must be a NUL-terminated string, and the `fsp-srv` session must be installed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nso__libnx_romfs_mount_self(name: *const c_char) -> u32 {
    if name.is_null() {
        return libnx_error(LibnxError::BadInput);
    }

    // SAFETY: the caller guarantees a NUL-terminated string.
    let Ok(name) = core::str::from_utf8(unsafe { CStr::from_ptr(name) }.to_bytes()) else {
        // A device registers under a name that is text, so a name that is not matches nothing.
        return libnx_error(LibnxError::BadInput);
    };

    match nx_romfs::mount::from_current_process(name) {
        Ok(()) => 0,
        Err(nx_romfs::mount::OpenError::NoSession) => libnx_error(LibnxError::NotFound),
        // The server's own code, which is what libnx passes through here and what a program with no
        // data partition inspects to tell that apart from a failure.
        Err(nx_romfs::mount::OpenError::Open(err)) => {
            use nx_sf::error::ToResultCode as _;
            err.to_rc()
        }
        Err(nx_romfs::mount::OpenError::Mount(err)) => match err {
            nx_romfs::mount::MountError::AlreadyMounted
            | nx_romfs::mount::MountError::Registry(_) => libnx_error(LibnxError::OutOfMemory),
            nx_romfs::mount::MountError::Image(_) => libnx_error(LibnxError::IoError),
        },
    }
}
