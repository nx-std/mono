//! Global `fsp-srv` queries and settings.
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

/// Stands in for libnx's `fsGetAndClearErrorInfo`.
///
/// # Safety
///
/// `out` must point to a writable `FsFileSystemProxyErrorInfo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_and_clear_error_info(_out: *mut c_void) -> u32 {
    todo!("fsGetAndClearErrorInfo")
}

/// Stands in for libnx's `fsGetAndClearMemoryReportInfo`.
///
/// # Safety
///
/// `out` must point to a writable `FsMemoryReportInfo`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_and_clear_memory_report_info(
    _out: *mut c_void,
) -> u32 {
    todo!("fsGetAndClearMemoryReportInfo")
}

/// Stands in for libnx's `fsGetContentStorageInfoIndex`.
///
/// # Safety
///
/// `out` must point to a writable `s32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_content_storage_info_index(
    _out: *mut i32,
) -> u32 {
    todo!("fsGetContentStorageInfoIndex")
}

/// Stands in for libnx's `fsGetGlobalAccessLogMode`.
///
/// # Safety
///
/// `out_mode` must point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_global_access_log_mode(
    _out_mode: *mut u32,
) -> u32 {
    todo!("fsGetGlobalAccessLogMode")
}

/// Stands in for libnx's `fsSetGlobalAccessLogMode`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_set_global_access_log_mode(_mode: u32) -> u32 {
    todo!("fsSetGlobalAccessLogMode")
}

/// Stands in for libnx's `fsGetProgramId`.
///
/// # Safety
///
/// `out` must point to a writable `u64`, and `path` to a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_program_id(
    _out: *mut u64,
    _path: *const c_char,
    _attr: u8,
) -> u32 {
    todo!("fsGetProgramId")
}

/// Stands in for libnx's `fsGetProgramIndexForAccessLog`.
///
/// # Safety
///
/// Both out-parameters must point to writable `u32`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_program_index_for_access_log(
    _out_program_index: *mut u32,
    _out_program_count: *mut u32,
) -> u32 {
    todo!("fsGetProgramIndexForAccessLog")
}

/// Stands in for libnx's `fsGetRightsIdByPath`.
///
/// # Safety
///
/// `path` must be a NUL-terminated string, and `out_rights_id` must point to a
/// writable `FsRightsId`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_rights_id_by_path(
    _path: *const c_char,
    _out_rights_id: *mut c_void,
) -> u32 {
    todo!("fsGetRightsIdByPath")
}

/// Stands in for libnx's `fsGetRightsIdAndKeyGenerationByPath`.
///
/// # Safety
///
/// `path` must be a NUL-terminated string, and both out-parameters must point
/// to writable storage of the matching libnx types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_rights_id_and_key_generation_by_path(
    _path: *const c_char,
    _attr: u8,
    _out_key_generation: *mut u8,
    _out_rights_id: *mut c_void,
) -> u32 {
    todo!("fsGetRightsIdAndKeyGenerationByPath")
}

/// Stands in for libnx's `fsIsExFatSupported`.
///
/// # Safety
///
/// `out` must point to a writable `bool`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_is_ex_fat_supported(_out: *mut bool) -> u32 {
    todo!("fsIsExFatSupported")
}

/// Stands in for libnx's `fsIsSignedSystemPartitionOnSdCardValid`.
///
/// # Safety
///
/// `out` must point to a writable `bool`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_is_signed_system_partition_on_sd_card_valid(
    _out: *mut bool,
) -> u32 {
    todo!("fsIsSignedSystemPartitionOnSdCardValid")
}

/// Stands in for libnx's `fsOutputAccessLogToSdCard`.
///
/// # Safety
///
/// `log` must point to `size` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_output_access_log_to_sd_card(
    _log: *const c_char,
    _size: usize,
) -> u32 {
    todo!("fsOutputAccessLogToSdCard")
}
