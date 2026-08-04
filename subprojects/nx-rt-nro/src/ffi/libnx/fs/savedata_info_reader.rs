//! `ISaveDataInfoReader` commands.
//!
//! Commands without an implementation are aliased to panicking stubs: one
//! left to libnx hangs rather than failing. See the parent module.
//!
//! Struct parameters are typed as opaque pointers; every one is a pointer, so
//! the ABI is exact without restating a layout this crate cannot check.

use core::ffi::c_void;

use nx_sf::ffi::Service;

/// Stands in for libnx's `fsOpenSaveDataInfoReader`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data_info_reader(
    _out: *mut Service,
    _save_data_space_id: i32,
) -> u32 {
    todo!("fsOpenSaveDataInfoReader")
}

/// Stands in for libnx's `fsOpenSaveDataInfoReaderWithFilter`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `save_data_filter` to a
/// readable `FsSaveDataFilter`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data_info_reader_with_filter(
    _out: *mut Service,
    _save_data_space_id: i32,
    _save_data_filter: *const c_void,
) -> u32 {
    todo!("fsOpenSaveDataInfoReaderWithFilter")
}

/// Stands in for libnx's `fsSaveDataInfoReaderRead`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out, and `buf` to
/// `max_entries` writable `FsSaveDataInfo` entries.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_save_data_info_reader_read(
    _s: *mut Service,
    _buf: *mut c_void,
    _max_entries: usize,
    _total_entries: *mut i64,
) -> u32 {
    todo!("fsSaveDataInfoReaderRead")
}

/// Stands in for libnx's `fsSaveDataInfoReaderClose`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out, and must not be closed
/// twice.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_save_data_info_reader_close(_s: *mut Service) {
    todo!("fsSaveDataInfoReaderClose")
}
