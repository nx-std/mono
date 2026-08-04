//! Filesystem openers.
//!
//! Commands without an implementation are aliased to panicking stubs: one
//! left to libnx hangs rather than failing. See the parent module.
//!
//! Struct parameters are typed as opaque pointers; every one is a pointer, so
//! the ABI is exact without restating a layout this crate cannot check.

use core::ffi::{
    c_char,
    c_void,
};

use nx_sf::{
    error::ToResultCode as _,
    ffi::Service,
};

use super::support::sub_object_view;
use crate::{
    ffi::common::GENERIC_ERROR,
    services::fs,
};

/// Stands in for libnx's `fsOpenBisFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `string` to a NUL-terminated
/// string.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_bis_file_system(
    _out: *mut Service,
    _partition_id: u32,
    _string: *const c_char,
) -> u32 {
    todo!("fsOpenBisFileSystem")
}

/// Stands in for libnx's `fsOpenContentStorageFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_content_storage_file_system(
    _out: *mut Service,
    _content_storage_id: u32,
) -> u32 {
    todo!("fsOpenContentStorageFileSystem")
}

/// Stands in for libnx's `fsOpenCustomStorageFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_custom_storage_file_system(
    _out: *mut Service,
    _custom_storage_id: u32,
) -> u32 {
    todo!("fsOpenCustomStorageFileSystem")
}

/// Stands in for libnx's `fsOpenDataFileSystemByCurrentProcess`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_data_file_system_by_current_process(
    _out: *mut Service,
) -> u32 {
    todo!("fsOpenDataFileSystemByCurrentProcess")
}

/// Stands in for libnx's `fsOpenDataFileSystemByProgramId`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_data_file_system_by_program_id(
    _out: *mut Service,
    _program_id: u64,
) -> u32 {
    todo!("fsOpenDataFileSystemByProgramId")
}

/// Stands in for libnx's `fsOpenFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `content_path` to a
/// NUL-terminated string.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_file_system(
    _out: *mut Service,
    _fs_type: u32,
    _content_path: *const c_char,
) -> u32 {
    todo!("fsOpenFileSystem")
}

/// Stands in for libnx's `fsOpenFileSystemWithId`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `content_path` to a
/// NUL-terminated string.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_file_system_with_id(
    _out: *mut Service,
    _id: u64,
    _fs_type: u32,
    _content_path: *const c_char,
    _attr: u8,
) -> u32 {
    todo!("fsOpenFileSystemWithId")
}

/// Stands in for libnx's `fsOpenFileSystemWithPatch`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_file_system_with_patch(
    _out: *mut Service,
    _id: u64,
    _fs_type: u32,
) -> u32 {
    todo!("fsOpenFileSystemWithPatch")
}

/// Stands in for libnx's `fsOpenGameCardFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `handle` to a readable
/// `FsGameCardHandle`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_game_card_file_system(
    _out: *mut Service,
    _handle: *const c_void,
    _partition: u32,
) -> u32 {
    todo!("fsOpenGameCardFileSystem")
}

/// Stands in for libnx's `fsOpenHostFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `path` to a NUL-terminated
/// string.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_host_file_system(
    _out: *mut Service,
    _path: *const c_char,
) -> u32 {
    todo!("fsOpenHostFileSystem")
}

/// Stands in for libnx's `fsOpenHostFileSystemWithOption`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `path` to a NUL-terminated
/// string.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_host_file_system_with_option(
    _out: *mut Service,
    _path: *const c_char,
    _flags: u32,
) -> u32 {
    todo!("fsOpenHostFileSystemWithOption")
}

/// Stands in for libnx's `fsOpenImageDirectoryFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_image_directory_file_system(
    _out: *mut Service,
    _image_directory_id: u32,
) -> u32 {
    todo!("fsOpenImageDirectoryFileSystem")
}

/// Opens the SD card filesystem.
///
/// Corresponds to `fsOpenSdCardFileSystem()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_sd_card_file_system(out: *mut Service) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(service) = fs::get_service() else {
        return GENERIC_ERROR;
    };
    let session = service.session_handle().to_handle();

    match service.open_sd_card_file_system() {
        Ok(filesystem) => {
            let object_id = filesystem.into_raw_object_id();
            // SAFETY: `out` was null-checked above and the caller guarantees it
            // is writable.
            unsafe { *out = sub_object_view(session, object_id) };
            0
        }
        Err(err) => err.to_rc(),
    }
}
