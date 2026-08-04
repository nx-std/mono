//! `IDeviceOperator` commands.
//!
//! Commands without an implementation are aliased to panicking stubs: one
//! left to libnx hangs rather than failing. See the parent module.
//!
//! Struct parameters are typed as opaque pointers; every one is a pointer, so
//! the ABI is exact without restating a layout this crate cannot check.

use core::ffi::c_void;

use nx_sf::ffi::Service;

/// Stands in for libnx's `fsOpenDeviceOperator`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_device_operator(_out: *mut Service) -> u32 {
    todo!("fsOpenDeviceOperator")
}

/// Stands in for libnx's `fsDeviceOperatorIsSdCardInserted`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_is_sd_card_inserted(
    _d: *mut Service,
    _out: *mut bool,
) -> u32 {
    todo!("fsDeviceOperatorIsSdCardInserted")
}

/// Stands in for libnx's `fsDeviceOperatorGetSdCardSpeedMode`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_sd_card_speed_mode(
    _d: *mut Service,
    _out: *mut i64,
) -> u32 {
    todo!("fsDeviceOperatorGetSdCardSpeedMode")
}

/// Stands in for libnx's `fsDeviceOperatorGetSdCardCid`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `dst` to
/// `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_sd_card_cid(
    _d: *mut Service,
    _dst: *mut c_void,
    _dst_size: usize,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetSdCardCid")
}

/// Stands in for libnx's `fsDeviceOperatorGetSdCardUserAreaSize`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_sd_card_user_area_size(
    _d: *mut Service,
    _out: *mut i64,
) -> u32 {
    todo!("fsDeviceOperatorGetSdCardUserAreaSize")
}

/// Stands in for libnx's `fsDeviceOperatorGetSdCardProtectedAreaSize`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_sd_card_protected_area_size(
    _d: *mut Service,
    _out: *mut i64,
) -> u32 {
    todo!("fsDeviceOperatorGetSdCardProtectedAreaSize")
}

/// Stands in for libnx's `fsDeviceOperatorGetAndClearSdCardErrorInfo`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `dst` to
/// `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_and_clear_sd_card_error_info(
    _d: *mut Service,
    _out: *mut c_void,
    _out_log_size: *mut i64,
    _dst: *mut c_void,
    _dst_size: usize,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetAndClearSdCardErrorInfo")
}

/// Stands in for libnx's `fsDeviceOperatorGetMmcCid`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `dst` to
/// `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_mmc_cid(
    _d: *mut Service,
    _dst: *mut c_void,
    _dst_size: usize,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetMmcCid")
}

/// Stands in for libnx's `fsDeviceOperatorGetMmcSpeedMode`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_mmc_speed_mode(
    _d: *mut Service,
    _out: *mut i64,
) -> u32 {
    todo!("fsDeviceOperatorGetMmcSpeedMode")
}

/// Stands in for libnx's `fsDeviceOperatorGetMmcPatrolCount`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_mmc_patrol_count(
    _d: *mut Service,
    _out: *mut u32,
) -> u32 {
    todo!("fsDeviceOperatorGetMmcPatrolCount")
}

/// Stands in for libnx's `fsDeviceOperatorGetMmcExtendedCsd`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `dst` to
/// `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_mmc_extended_csd(
    _d: *mut Service,
    _dst: *mut c_void,
    _dst_size: usize,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetMmcExtendedCsd")
}

/// Stands in for libnx's `fsDeviceOperatorGetAndClearMmcErrorInfo`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `dst` to
/// `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_and_clear_mmc_error_info(
    _d: *mut Service,
    _out: *mut c_void,
    _out_log_size: *mut i64,
    _dst: *mut c_void,
    _dst_size: usize,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetAndClearMmcErrorInfo")
}

/// Stands in for libnx's `fsDeviceOperatorIsGameCardInserted`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_is_game_card_inserted(
    _d: *mut Service,
    _out: *mut bool,
) -> u32 {
    todo!("fsDeviceOperatorIsGameCardInserted")
}

/// Stands in for libnx's `fsDeviceOperatorGetGameCardHandle`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_game_card_handle(
    _d: *mut Service,
    _out: *mut c_void,
) -> u32 {
    todo!("fsDeviceOperatorGetGameCardHandle")
}

/// Stands in for libnx's `fsDeviceOperatorGetGameCardAttribute`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `handle` to a
/// readable `FsGameCardHandle`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_game_card_attribute(
    _d: *mut Service,
    _handle: *const c_void,
    _out: *mut u8,
) -> u32 {
    todo!("fsDeviceOperatorGetGameCardAttribute")
}

/// Stands in for libnx's `fsDeviceOperatorGetGameCardDeviceId`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `dst` to
/// `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_game_card_device_id(
    _d: *mut Service,
    _dst: *mut c_void,
    _dst_size: usize,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetGameCardDeviceId")
}

/// Stands in for libnx's `fsDeviceOperatorGetGameCardDeviceCertificate`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, `handle` to a readable
/// `FsGameCardHandle`, and `dst` to `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_game_card_device_certificate(
    _d: *mut Service,
    _handle: *const c_void,
    _dst: *mut c_void,
    _dst_size: usize,
    _out_size: *mut i64,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetGameCardDeviceCertificate")
}

/// Stands in for libnx's `fsDeviceOperatorGetGameCardIdSet`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `dst` to
/// `dst_size` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_game_card_id_set(
    _d: *mut Service,
    _dst: *mut c_void,
    _dst_size: usize,
    _size: i64,
) -> u32 {
    todo!("fsDeviceOperatorGetGameCardIdSet")
}

/// Stands in for libnx's `fsDeviceOperatorGetGameCardErrorReportInfo`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_game_card_error_report_info(
    _d: *mut Service,
    _out: *mut c_void,
) -> u32 {
    todo!("fsDeviceOperatorGetGameCardErrorReportInfo")
}

/// Stands in for libnx's `fsDeviceOperatorGetGameCardUpdatePartitionInfo`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and `handle` to a
/// readable `FsGameCardHandle`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_get_game_card_update_partition_info(
    _d: *mut Service,
    _handle: *const c_void,
    _out: *mut c_void,
) -> u32 {
    todo!("fsDeviceOperatorGetGameCardUpdatePartitionInfo")
}

/// Stands in for libnx's `fsDeviceOperatorChallengeCardExistence`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, `handle` to a readable
/// `FsGameCardHandle`, and each buffer to its stated number of bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_challenge_card_existence(
    _d: *mut Service,
    _handle: *const c_void,
    _dst: *mut c_void,
    _dst_size: usize,
    _seed: *mut c_void,
    _seed_size: usize,
    _value: *mut c_void,
    _value_size: usize,
) -> u32 {
    todo!("fsDeviceOperatorChallengeCardExistence")
}

/// Stands in for libnx's `fsDeviceOperatorClose`.
///
/// # Safety
///
/// `d` must point to a `Service` this module handed out, and must not be closed
/// twice.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_device_operator_close(_d: *mut Service) {
    todo!("fsDeviceOperatorClose")
}
