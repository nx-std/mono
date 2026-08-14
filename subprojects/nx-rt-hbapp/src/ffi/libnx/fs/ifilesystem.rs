//! `IFileSystem` commands.

use core::ffi::{
    c_char,
    c_void,
};

use nx_service_fs::{
    DirOpenMode,
    FS_MAX_PATH,
    FileSystemQueryId,
    FsDir,
    FsFile,
    FsFileSystem,
    OpenMode,
    TimeStampRaw,
};
use nx_sf::ffi::Service;

use super::support::{
    PATH_TOO_LONG,
    copy_path,
    fs_space_query,
    object_id_of,
    sub_object_view,
    to_rc,
    with_filesystem,
};
use crate::{
    ffi::common::GENERIC_ERROR,
    services::fs,
};

/// Creates a file.
///
/// Corresponds to `fsFsCreateFile()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_create_file(
    fs_ptr: *const Service,
    path: *const c_char,
    size: i64,
    option: u32,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };
    let option = nx_service_fs::CreateOption::from_bits_truncate(option);

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.create_file(&path, size, option)) })
}

/// Deletes a file.
///
/// Corresponds to `fsFsDeleteFile()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_delete_file(
    fs_ptr: *const Service,
    path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.delete_file(&path)) })
}

/// Creates a directory.
///
/// Corresponds to `fsFsCreateDirectory()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_create_directory(
    fs_ptr: *const Service,
    path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.create_directory(&path)) })
}

/// Deletes a directory.
///
/// Corresponds to `fsFsDeleteDirectory()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_delete_directory(
    fs_ptr: *const Service,
    path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.delete_directory(&path)) })
}

/// Deletes a directory and everything under it.
///
/// Corresponds to `fsFsDeleteDirectoryRecursively()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_delete_directory_recursively(
    fs_ptr: *const Service,
    path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.delete_directory_recursively(&path)) })
}

/// Empties a directory without deleting it.
///
/// Corresponds to `fsFsCleanDirectoryRecursively()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_clean_directory_recursively(
    fs_ptr: *const Service,
    path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.clean_directory_recursively(&path)) })
}

/// Renames a file.
///
/// Corresponds to `fsFsRenameFile()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// both paths must be null or NUL-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_rename_file(
    fs_ptr: *const Service,
    cur_path: *const c_char,
    new_path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees NUL-terminated paths.
    let (Some(cur_path), Some(new_path)) = (unsafe { copy_path(cur_path) }, unsafe {
        copy_path(new_path)
    }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.rename_file(&cur_path, &new_path)) })
}

/// Renames a directory.
///
/// Corresponds to `fsFsRenameDirectory()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// both paths must be null or NUL-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_rename_directory(
    fs_ptr: *const Service,
    cur_path: *const c_char,
    new_path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees NUL-terminated paths.
    let (Some(cur_path), Some(new_path)) = (unsafe { copy_path(cur_path) }, unsafe {
        copy_path(new_path)
    }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.rename_directory(&cur_path, &new_path)) })
}

/// Reports whether a path names a file or a directory.
///
/// Corresponds to `fsFsGetEntryType()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, `path`
/// must be null or a NUL-terminated string, and `out` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_get_entry_type(
    fs_ptr: *const Service,
    path: *const c_char,
    out: *mut u32,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_filesystem(fs_ptr, |fs| fs.get_entry_type(&path)) } {
        Ok(entry_type) => {
            // SAFETY: `out` was null-checked above and the caller guarantees it
            // is writable.
            unsafe { *out = entry_type as u32 };
            0
        }
        Err(rc) => rc,
    }
}

/// Opens a file.
///
/// Corresponds to `fsFsOpenFile()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, `path`
/// must be null or a NUL-terminated string, and `out` must be null or writable.
/// On success the caller owes the returned file a `fsFileClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_open_file(
    fs_ptr: *const Service,
    path: *const c_char,
    mode: u32,
    out: *mut Service,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };
    let mode = OpenMode::from_bits_truncate(mode);

    let Some(service) = fs::get_service() else {
        return GENERIC_ERROR;
    };
    let session = service.session_handle().to_handle();

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe {
        with_filesystem(fs_ptr, |fs| {
            fs.open_file(&path, mode).map(FsFile::into_raw_object_id)
        })
    } {
        Ok(object_id) => {
            // SAFETY: `out` was null-checked above and the caller guarantees it
            // is writable.
            unsafe { *out = sub_object_view(session, object_id) };
            0
        }
        Err(rc) => rc,
    }
}

/// Opens a directory.
///
/// Corresponds to `fsFsOpenDirectory()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, `path`
/// must be null or a NUL-terminated string, and `out` must be null or writable.
/// On success the caller owes the returned directory a `fsDirClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_open_directory(
    fs_ptr: *const Service,
    path: *const c_char,
    mode: u32,
    out: *mut Service,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };
    let mode = DirOpenMode::from_bits_truncate(mode);

    let Some(service) = fs::get_service() else {
        return GENERIC_ERROR;
    };
    let session = service.session_handle().to_handle();

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe {
        with_filesystem(fs_ptr, |fs| {
            fs.open_directory(&path, mode)
                .map(FsDir::into_raw_object_id)
        })
    } {
        Ok(object_id) => {
            // SAFETY: `out` was null-checked above and the caller guarantees it
            // is writable.
            unsafe { *out = sub_object_view(session, object_id) };
            0
        }
        Err(rc) => rc,
    }
}

/// Commits pending writes on a filesystem.
///
/// Corresponds to `fsFsCommit()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_commit(fs_ptr: *const Service) -> u32 {
    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_filesystem(fs_ptr, |fs| fs.commit()) })
}

/// Reports the free space at a path.
///
/// Corresponds to `fsFsGetFreeSpace()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, `path`
/// must be null or a NUL-terminated string, and `out` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_get_free_space(
    fs_ptr: *const Service,
    path: *const c_char,
    out: *mut i64,
) -> u32 {
    // SAFETY: the caller guarantees the pointers described below.
    unsafe { fs_space_query(fs_ptr, path, out, |fs, path| fs.get_free_space(path)) }
}

/// Reports the total space at a path.
///
/// Corresponds to `fsFsGetTotalSpace()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, `path`
/// must be null or a NUL-terminated string, and `out` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_get_total_space(
    fs_ptr: *const Service,
    path: *const c_char,
    out: *mut i64,
) -> u32 {
    // SAFETY: the caller guarantees the pointers described below.
    unsafe { fs_space_query(fs_ptr, path, out, |fs, path| fs.get_total_space(path)) }
}

/// Reads a path's raw timestamps.
///
/// Corresponds to `fsFsGetFileTimeStampRaw()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, `path`
/// must be null or a NUL-terminated string, and `out` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_get_file_time_stamp_raw(
    fs_ptr: *const Service,
    path: *const c_char,
    out: *mut TimeStampRaw,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_filesystem(fs_ptr, |fs| fs.get_file_time_stamp_raw(&path)) } {
        Ok(stamp) => {
            // SAFETY: `out` was null-checked above and the caller guarantees it
            // is writable.
            unsafe { *out = stamp };
            0
        }
        Err(rc) => rc,
    }
}

/// Marks a path as a concatenation file.
///
/// Corresponds to `fsFsSetConcatenationFileAttribute()` in libnx, which is a
/// `QueryEntry` with no payload in either direction.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_set_concatenation_file_attribute(
    fs_ptr: *const Service,
    path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated path.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe {
        with_filesystem(fs_ptr, |fs| {
            fs.query_entry(
                &path,
                FileSystemQueryId::SetConcatenationFileAttribute,
                &[],
                &mut [],
            )
        })
    })
}

/// Reports whether the SD card holds a valid signed system partition.
///
/// Corresponds to `fsFsIsValidSignedSystemPartitionOnSdCard()` in libnx, which
/// is a `QueryEntry` on the root path returning one byte.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `out` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_is_valid_signed_system_partition_on_sd_card(
    fs_ptr: *const Service,
    out: *mut bool,
) -> u32 {
    let mut path = [0u8; FS_MAX_PATH];
    path[0] = b'/';

    let mut answer = [0u8; 1];
    // SAFETY: the caller guarantees a readable `Service`.
    let result = unsafe {
        with_filesystem(fs_ptr, |fs| {
            fs.query_entry(
                &path,
                FileSystemQueryId::IsValidSignedSystemPartitionOnSdCard,
                &[],
                &mut answer,
            )
        })
    };

    match result {
        Ok(()) => {
            if !out.is_null() {
                // SAFETY: `out` was null-checked and the caller guarantees it
                // is writable.
                unsafe { *out = answer[0] & 1 != 0 };
            }
            0
        }
        Err(rc) => rc,
    }
}

/// Closes a filesystem.
///
/// Corresponds to `fsFsClose()` in libnx. This is where the close obligation
/// C has been holding is finally discharged.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a writable `Service` this module handed
/// out, and must be closed exactly once: a second call would close an object
/// id the server may have since reissued.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_close(fs_ptr: *mut Service) {
    // SAFETY: the caller guarantees a readable `Service`.
    let Some(object_id) = (unsafe { object_id_of(fs_ptr) }) else {
        return;
    };
    let Some(service) = fs::get_service() else {
        return;
    };

    // SAFETY: `object_id` came from a `Service` this module handed out, and
    // `fsFsClose` is called once per open, so nothing else owes this close.
    // Letting the wrapper drop sends it.
    drop(FsFileSystem::from_raw_object_id_unchecked(
        &service, object_id,
    ));

    // SAFETY: the caller guarantees `fs_ptr` is writable; zeroing matches
    // libnx, which leaves a closed `Service` unusable.
    unsafe { (*fs_ptr).object_id = 0 };
}

/// Stands in for libnx's `fsFsQueryEntry`.
///
/// # Safety
///
/// `fs` must point to a `Service` this module handed out, the buffers to their
/// stated sizes, and `path` to a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_query_entry(
    _fs: *mut Service,
    _out: *mut c_void,
    _out_size: usize,
    _in_buf: *const c_void,
    _in_size: usize,
    _path: *const c_char,
    _query_id: u32,
) -> u32 {
    todo!("fsFsQueryEntry")
}

/// Stands in for libnx's `fsFsGetFileSystemAttribute`.
///
/// # Safety
///
/// `fs` must point to a `Service` this module handed out, and `out` to a
/// writable `FsFileSystemAttribute`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_fs_get_file_system_attribute(
    _fs: *mut Service,
    _out: *mut c_void,
) -> u32 {
    todo!("fsFsGetFileSystemAttribute")
}
