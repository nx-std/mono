//! `IFile` commands.

use core::ffi::c_void;

use nx_service_fs::{
    FsFile,
    ReadOption,
    WriteOption,
};
use nx_sf::ffi::Service;

use super::support::{
    object_id_of,
    to_rc,
    with_file,
};
use crate::{
    ffi::common::GENERIC_ERROR,
    services::fs,
};

/// Reads from a file.
///
/// Corresponds to `fsFileRead()` in libnx.
///
/// # Safety
///
/// `file` must be null or point to a `Service` this module handed out, `buf`
/// must be writable for `read_size` bytes, and `bytes_read` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_file_read(
    file: *const Service,
    offset: i64,
    buf: *mut u8,
    read_size: u64,
    option: u32,
    bytes_read: *mut u64,
) -> u32 {
    if buf.is_null() {
        return GENERIC_ERROR;
    }
    let Ok(len) = usize::try_from(read_size) else {
        return GENERIC_ERROR;
    };
    let option = ReadOption::from_bits_truncate(option);

    // SAFETY: the caller guarantees `buf` is writable for `read_size` bytes.
    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_file(file, |f| f.read(offset, slice, read_size, option)) } {
        Ok(count) => {
            if !bytes_read.is_null() {
                // SAFETY: null-checked, and the caller guarantees writability.
                unsafe { *bytes_read = count };
            }
            0
        }
        Err(rc) => rc,
    }
}

/// Writes to a file.
///
/// Corresponds to `fsFileWrite()` in libnx.
///
/// # Safety
///
/// `file` must be null or point to a `Service` this module handed out, and
/// `buf` must be readable for `write_size` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_file_write(
    file: *const Service,
    offset: i64,
    buf: *const u8,
    write_size: u64,
    option: u32,
) -> u32 {
    if buf.is_null() {
        return GENERIC_ERROR;
    }
    let Ok(len) = usize::try_from(write_size) else {
        return GENERIC_ERROR;
    };
    let option = WriteOption::from_bits_truncate(option);

    // SAFETY: the caller guarantees `buf` is readable for `write_size` bytes.
    let slice = unsafe { core::slice::from_raw_parts(buf, len) };

    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_file(file, |f| f.write(offset, slice, write_size, option)) })
}

/// Flushes a file.
///
/// Corresponds to `fsFileFlush()` in libnx.
///
/// # Safety
///
/// `file` must be null or point to a `Service` this module handed out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_file_flush(file: *const Service) -> u32 {
    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_file(file, |f| f.flush()) })
}

/// Resizes a file.
///
/// Corresponds to `fsFileSetSize()` in libnx.
///
/// # Safety
///
/// `file` must be null or point to a `Service` this module handed out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_file_set_size(
    file: *const Service,
    size: i64,
) -> u32 {
    // SAFETY: the caller guarantees a readable `Service`.
    to_rc(unsafe { with_file(file, |f| f.set_size(size)) })
}

/// Reads a file's size.
///
/// Corresponds to `fsFileGetSize()` in libnx.
///
/// # Safety
///
/// `file` must be null or point to a `Service` this module handed out, and
/// `out` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_file_get_size(
    file: *const Service,
    out: *mut i64,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_file(file, |f| f.get_size()) } {
        Ok(size) => {
            // SAFETY: null-checked, and the caller guarantees writability.
            unsafe { *out = size };
            0
        }
        Err(rc) => rc,
    }
}

/// Closes a file.
///
/// Corresponds to `fsFileClose()` in libnx.
///
/// # Safety
///
/// `file` must be null or point to a writable `Service` this module handed out,
/// and must be closed exactly once: a second call would close an object id the
/// server may have since reissued.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_file_close(file: *mut Service) {
    // SAFETY: the caller guarantees a readable `Service`.
    let Some(object_id) = (unsafe { object_id_of(file) }) else {
        return;
    };
    let Some(service) = fs::get_service() else {
        return;
    };

    // SAFETY: as in `fsFsClose` - one close per open, discharged here.
    drop(FsFile::from_raw_object_id_unchecked(&service, object_id));

    // SAFETY: the caller guarantees `file` is writable.
    unsafe { (*file).object_id = 0 };
}

/// Stands in for libnx's `fsFileOperateRange`.
///
/// # Safety
///
/// `f` must point to a `Service` this module handed out, and `out` to a
/// writable `FsRangeInfo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_file_operate_range(
    _f: *mut Service,
    _op_id: u32,
    _off: i64,
    _len: i64,
    _out: *mut c_void,
) -> u32 {
    todo!("fsFileOperateRange")
}
