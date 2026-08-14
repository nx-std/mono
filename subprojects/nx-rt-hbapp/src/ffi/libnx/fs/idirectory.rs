//! `IDirectory` commands.

use nx_service_fs::{
    DirectoryEntry,
    FsDir,
};
use nx_sf::ffi::Service;

use super::support::{
    object_id_of,
    with_dir,
};
use crate::{
    ffi::common::GENERIC_ERROR,
    services::fs,
};

/// Reads directory entries.
///
/// Corresponds to `fsDirRead()` in libnx.
///
/// # Safety
///
/// `dir` must be null or point to a `Service` this module handed out, `buf`
/// must be writable for `max_entries` entries, and `total_entries` must be null
/// or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_dir_read(
    dir: *const Service,
    total_entries: *mut i64,
    max_entries: u64,
    buf: *mut DirectoryEntry,
) -> u32 {
    if buf.is_null() {
        return GENERIC_ERROR;
    }
    let Ok(len) = usize::try_from(max_entries) else {
        return GENERIC_ERROR;
    };

    // SAFETY: the caller guarantees `buf` holds `max_entries` entries.
    let entries = unsafe { core::slice::from_raw_parts_mut(buf, len) };

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_dir(dir, |d| d.read(entries)) } {
        Ok(count) => {
            if !total_entries.is_null() {
                // SAFETY: null-checked, and the caller guarantees writability.
                unsafe { *total_entries = count };
            }
            0
        }
        Err(rc) => rc,
    }
}

/// Counts the entries in a directory.
///
/// Corresponds to `fsDirGetEntryCount()` in libnx.
///
/// # Safety
///
/// `dir` must be null or point to a `Service` this module handed out, and
/// `count` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_dir_get_entry_count(
    dir: *const Service,
    count: *mut i64,
) -> u32 {
    if count.is_null() {
        return GENERIC_ERROR;
    }

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_dir(dir, |d| d.get_entry_count()) } {
        Ok(total) => {
            // SAFETY: null-checked, and the caller guarantees writability.
            unsafe { *count = total };
            0
        }
        Err(rc) => rc,
    }
}

/// Closes a directory.
///
/// Corresponds to `fsDirClose()` in libnx.
///
/// # Safety
///
/// `dir` must be null or point to a writable `Service` this module handed out,
/// and must be closed exactly once: a second call would close an object id the
/// server may have since reissued.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_dir_close(dir: *mut Service) {
    // SAFETY: the caller guarantees a readable `Service`.
    let Some(object_id) = (unsafe { object_id_of(dir) }) else {
        return;
    };
    let Some(service) = fs::get_service() else {
        return;
    };

    // SAFETY: as in `fsFsClose` - one close per open, discharged here.
    drop(FsDir::from_raw_object_id_unchecked(&service, object_id));

    // SAFETY: the caller guarantees `dir` is writable.
    unsafe { (*dir).object_id = 0 };
}
