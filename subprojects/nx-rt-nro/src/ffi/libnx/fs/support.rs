//! Shared helpers for the `fsp-srv` command modules.
//!
//! These sit in a sibling module rather than the parent so the command modules
//! can reach them without referencing the file that declared them.

use core::ffi::c_char;

use nx_service_fs::{
    FS_MAX_PATH,
    FsDir,
    FsFile,
    FsFileSystem,
};
use nx_sf::{
    error::ToResultCode as _,
    ffi::Service,
};

use crate::{
    ffi::common::GENERIC_ERROR,
    services::fs,
};

/// Result code libnx returns for a path that does not fit `FS_MAX_PATH`.
///
/// libnx never checks: it `strncpy`s into a fixed buffer and lets the truncated
/// path reach the server. Rejecting here is the hard shell the C caller does
/// not have; the code is the generic one because no server ever saw the
/// request.
pub(super) const PATH_TOO_LONG: u32 = GENERIC_ERROR;
/// Copies a C path into the fixed-size buffer every `fsp-srv` path command
/// takes, NUL-terminating it.
///
/// Returns `None` when `path` is null or does not fit, which the caller renders
/// as [`PATH_TOO_LONG`].
///
/// # Safety
///
/// `path` must be null or point to a NUL-terminated string.
pub(super) unsafe fn copy_path(path: *const c_char) -> Option<[u8; FS_MAX_PATH]> {
    if path.is_null() {
        return None;
    }

    let mut buf = [0u8; FS_MAX_PATH];
    for (i, slot) in buf.iter_mut().enumerate() {
        // SAFETY: the caller guarantees a NUL-terminated string, so the walk
        // stops at or before the terminator; `i` never passes it.
        let byte = unsafe { *path.add(i) };
        if byte == 0 {
            return Some(buf);
        }
        *slot = byte;
    }

    // The loop filled every byte without meeting a NUL, so the path needs at
    // least FS_MAX_PATH bytes plus a terminator.
    None
}

/// Builds the non-owning `Service` view C stores for a domain sub-object.
///
/// `own_handle = 0` with a non-zero `object_id` is libnx's domain-subservice
/// encoding. The zero says this crate keeps the close obligation, so a stray
/// `serviceClose` on the C side tears nothing down.
pub(super) fn sub_object_view(session: nx_svc::ipc::Handle, object_id: u32) -> Service {
    Service {
        session,
        own_handle: 0,
        object_id,
        pointer_buffer_size: 0,
    }
}

/// Reads the domain object id C stored in a `Service`.
///
/// # Safety
///
/// `service` must be null or point to a readable `Service`.
pub(super) unsafe fn object_id_of(service: *const Service) -> Option<u32> {
    if service.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees a readable `Service`.
    let object_id = unsafe { (*service).object_id };
    (object_id != 0).then_some(object_id)
}

/// Runs one command against the filesystem `fs` names.
///
/// The wrapper is rebuilt for the call and gives the close obligation back
/// before returning, so the object outlives it; only `fsFsClose` closes.
///
/// # Safety
///
/// `fs` must be null or point to a readable `Service`.
pub(super) unsafe fn with_filesystem<R>(
    fs: *const Service,
    f: impl FnOnce(&FsFileSystem<'_>) -> Result<R, nx_sf::service::DispatchError>,
) -> Result<R, u32> {
    // SAFETY: forwarded to this function's own caller.
    let Some(object_id) = (unsafe { object_id_of(fs) }) else {
        return Err(GENERIC_ERROR);
    };
    let Some(service) = fs::get_service() else {
        return Err(GENERIC_ERROR);
    };

    // SAFETY: `object_id` came from a `Service` this module handed out, so the
    // server issued it inside this session's domain and only a `*Close` entry
    // point closes it. The obligation is handed straight back below.
    let wrapper = FsFileSystem::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&wrapper);
    let _ = wrapper.into_raw_object_id();

    result.map_err(|err| err.to_rc())
}

/// Runs one command against the file `f` names. See [`with_filesystem`].
///
/// # Safety
///
/// `file` must be null or point to a readable `Service`.
pub(super) unsafe fn with_file<R>(
    file: *const Service,
    f: impl FnOnce(&FsFile<'_>) -> Result<R, nx_sf::service::DispatchError>,
) -> Result<R, u32> {
    // SAFETY: forwarded to this function's own caller.
    let Some(object_id) = (unsafe { object_id_of(file) }) else {
        return Err(GENERIC_ERROR);
    };
    let Some(service) = fs::get_service() else {
        return Err(GENERIC_ERROR);
    };

    // SAFETY: as in `with_filesystem`.
    let wrapper = FsFile::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&wrapper);
    let _ = wrapper.into_raw_object_id();

    result.map_err(|err| err.to_rc())
}

/// Runs one command against the directory `dir` names. See [`with_filesystem`].
///
/// # Safety
///
/// `dir` must be null or point to a readable `Service`.
pub(super) unsafe fn with_dir<R>(
    dir: *const Service,
    f: impl FnOnce(&FsDir<'_>) -> Result<R, nx_sf::service::DispatchError>,
) -> Result<R, u32> {
    // SAFETY: forwarded to this function's own caller.
    let Some(object_id) = (unsafe { object_id_of(dir) }) else {
        return Err(GENERIC_ERROR);
    };
    let Some(service) = fs::get_service() else {
        return Err(GENERIC_ERROR);
    };

    // SAFETY: as in `with_filesystem`.
    let wrapper = FsDir::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&wrapper);
    let _ = wrapper.into_raw_object_id();

    result.map_err(|err| err.to_rc())
}

/// Renders a `Result<(), u32>` as the bare result code C expects.
pub(super) fn to_rc(result: Result<(), u32>) -> u32 {
    match result {
        Ok(()) => 0,
        Err(rc) => rc,
    }
}

/// Shared body of the two space queries, which differ only in the command they
/// send.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a readable `Service`, `path` must be null
/// or NUL-terminated, and `out` must be null or writable.
pub(super) unsafe fn fs_space_query(
    fs_ptr: *const Service,
    path: *const c_char,
    out: *mut i64,
    query: impl FnOnce(
        &FsFileSystem<'_>,
        &[u8; FS_MAX_PATH],
    ) -> Result<i64, nx_sf::service::DispatchError>,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }
    // SAFETY: forwarded to this function's own caller.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: forwarded to this function's own caller.
    match unsafe { with_filesystem(fs_ptr, |fs| query(fs, &path)) } {
        Ok(space) => {
            // SAFETY: `out` was null-checked above and the caller guarantees it
            // is writable.
            unsafe { *out = space };
            0
        }
        Err(rc) => rc,
    }
}
