//! Storage openers and `IStorage` commands.
//!
//! Commands without an implementation are aliased to panicking stubs: one
//! left to libnx hangs rather than failing. See the parent module.
//!
//! Struct parameters are typed as opaque pointers; every one is a pointer, so
//! the ABI is exact without restating a layout this crate cannot check.

use core::ffi::c_void;

use nx_sf::ffi::Service;

/// Stands in for libnx's `fsOpenBisStorage`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_bis_storage(
    _out: *mut Service,
    _partition_id: u32,
) -> u32 {
    todo!("fsOpenBisStorage")
}

/// Stands in for libnx's `fsOpenDataStorageByCurrentProcess`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_data_storage_by_current_process(
    _out: *mut Service,
) -> u32 {
    todo!("fsOpenDataStorageByCurrentProcess")
}

/// Stands in for libnx's `fsOpenDataStorageByDataId`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_data_storage_by_data_id(
    _out: *mut Service,
    _data_id: u64,
    _storage_id: u32,
) -> u32 {
    todo!("fsOpenDataStorageByDataId")
}

/// Stands in for libnx's `fsOpenDataStorageByProgramId`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_data_storage_by_program_id(
    _out: *mut Service,
    _program_id: u64,
) -> u32 {
    todo!("fsOpenDataStorageByProgramId")
}

/// Stands in for libnx's `fsOpenPatchDataStorageByCurrentProcess`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_patch_data_storage_by_current_process(
    _out: *mut Service,
) -> u32 {
    todo!("fsOpenPatchDataStorageByCurrentProcess")
}

/// Stands in for libnx's `fsStorageRead`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out, and `buf` to
/// `read_size` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_storage_read(
    _s: *mut Service,
    _off: i64,
    _buf: *mut c_void,
    _read_size: u64,
) -> u32 {
    todo!("fsStorageRead")
}

/// Stands in for libnx's `fsStorageWrite`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out, and `buf` to
/// `write_size` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_storage_write(
    _s: *mut Service,
    _off: i64,
    _buf: *const c_void,
    _write_size: u64,
) -> u32 {
    todo!("fsStorageWrite")
}

/// Stands in for libnx's `fsStorageFlush`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_storage_flush(_s: *mut Service) -> u32 {
    todo!("fsStorageFlush")
}

/// Stands in for libnx's `fsStorageSetSize`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_storage_set_size(
    _s: *mut Service,
    _sz: i64,
) -> u32 {
    todo!("fsStorageSetSize")
}

/// Stands in for libnx's `fsStorageGetSize`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_storage_get_size(
    _s: *mut Service,
    _out: *mut i64,
) -> u32 {
    todo!("fsStorageGetSize")
}

/// Stands in for libnx's `fsStorageOperateRange`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out, and `out` to a
/// writable `FsRangeInfo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_storage_operate_range(
    _s: *mut Service,
    _op_id: u32,
    _off: i64,
    _len: i64,
    _out: *mut c_void,
) -> u32 {
    todo!("fsStorageOperateRange")
}

/// Stands in for libnx's `fsStorageClose`.
///
/// # Safety
///
/// `s` must point to a `Service` this module handed out, and must not be closed
/// twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_storage_close(_s: *mut Service) {
    todo!("fsStorageClose")
}
