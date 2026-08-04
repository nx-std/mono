//! Savedata openers and management commands.
//!
//! Commands without an implementation are aliased to panicking stubs: one
//! left to libnx hangs rather than failing. See the parent module.
//!
//! Struct parameters are typed as opaque pointers. Every one is a pointer, so
//! the ABI is exact, and restating the layouts would be a claim this crate
//! cannot check. [`AccountUid`] is the exception: it is passed by value, so
//! its size decides how the arguments after it land.

use core::ffi::c_void;

use nx_sf::ffi::Service;

/// Account user id, passed by value.
///
/// Declared because a by-value struct occupies argument registers according to
/// its size; an opaque stand-in would shift every parameter after it.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountUid {
    /// Raw user id words. All-zero means unset.
    pub uid: [u64; 2],
}

/// Stands in for libnx's `fsOpen_SaveData`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data(
    _out: *mut Service,
    _application_id: u64,
    _uid: AccountUid,
) -> u32 {
    todo!("fsOpen_SaveData")
}

/// Stands in for libnx's `fsOpen_SaveDataReadOnly`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data_read_only(
    _out: *mut Service,
    _application_id: u64,
    _uid: AccountUid,
) -> u32 {
    todo!("fsOpen_SaveDataReadOnly")
}

/// Stands in for libnx's `fsOpen_BcatSaveData`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_bcat_save_data(
    _out: *mut Service,
    _application_id: u64,
) -> u32 {
    todo!("fsOpen_BcatSaveData")
}

/// Stands in for libnx's `fsOpen_DeviceSaveData`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_device_save_data(
    _out: *mut Service,
    _application_id: u64,
) -> u32 {
    todo!("fsOpen_DeviceSaveData")
}

/// Stands in for libnx's `fsOpen_TemporaryStorage`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_temporary_storage(_out: *mut Service) -> u32 {
    todo!("fsOpen_TemporaryStorage")
}

/// Stands in for libnx's `fsOpen_CacheStorage`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_cache_storage(
    _out: *mut Service,
    _application_id: u64,
    _save_data_index: u16,
) -> u32 {
    todo!("fsOpen_CacheStorage")
}

/// Stands in for libnx's `fsOpen_SystemSaveData`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_system_save_data(
    _out: *mut Service,
    _save_data_space_id: i32,
    _system_save_data_id: u64,
    _uid: AccountUid,
) -> u32 {
    todo!("fsOpen_SystemSaveData")
}

/// Stands in for libnx's `fsOpen_SystemBcatSaveData`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_system_bcat_save_data(
    _out: *mut Service,
    _system_save_data_id: u64,
) -> u32 {
    todo!("fsOpen_SystemBcatSaveData")
}

/// Stands in for libnx's `fsOpenSaveDataFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `attr` to a readable
/// `FsSaveDataAttribute`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data_file_system(
    _out: *mut Service,
    _save_data_space_id: i32,
    _attr: *const c_void,
) -> u32 {
    todo!("fsOpenSaveDataFileSystem")
}

/// Stands in for libnx's `fsOpenSaveDataFileSystemBySystemSaveDataId`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `attr` to a readable
/// `FsSaveDataAttribute`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_save_data_file_system_by_system_save_data_id(
    _out: *mut Service,
    _save_data_space_id: i32,
    _attr: *const c_void,
) -> u32 {
    todo!("fsOpenSaveDataFileSystemBySystemSaveDataId")
}

/// Stands in for libnx's `fsOpenReadOnlySaveDataFileSystem`.
///
/// # Safety
///
/// `out` must point to a writable `Service`, and `attr` to a readable
/// `FsSaveDataAttribute`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_read_only_save_data_file_system(
    _out: *mut Service,
    _save_data_space_id: i32,
    _attr: *const c_void,
) -> u32 {
    todo!("fsOpenReadOnlySaveDataFileSystem")
}

/// Stands in for libnx's `fsCreateSaveDataFileSystem`.
///
/// # Safety
///
/// `attr`, `creation_info` and `meta` must point to readable structs of the
/// matching libnx types.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_create_save_data_file_system(
    _attr: *const c_void,
    _creation_info: *const c_void,
    _meta: *const c_void,
) -> u32 {
    todo!("fsCreateSaveDataFileSystem")
}

/// Stands in for libnx's `fsCreateSaveDataFileSystemBySystemSaveDataId`.
///
/// # Safety
///
/// `attr` and `creation_info` must point to readable structs of the matching
/// libnx types.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_create_save_data_file_system_by_system_save_data_id(
    _attr: *const c_void,
    _creation_info: *const c_void,
) -> u32 {
    todo!("fsCreateSaveDataFileSystemBySystemSaveDataId")
}

/// Stands in for libnx's `fsCreate_SystemSaveData`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_create_system_save_data(
    _save_data_space_id: i32,
    _system_save_data_id: u64,
    _size: i64,
    _journal_size: i64,
    _flags: u32,
) -> u32 {
    todo!("fsCreate_SystemSaveData")
}

/// Stands in for libnx's `fsCreate_SystemSaveDataWithOwner`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_create_system_save_data_with_owner(
    _save_data_space_id: i32,
    _system_save_data_id: u64,
    _uid: AccountUid,
    _owner_id: u64,
    _size: i64,
    _journal_size: i64,
    _flags: u32,
) -> u32 {
    todo!("fsCreate_SystemSaveDataWithOwner")
}

/// Stands in for libnx's `fsCreate_TemporaryStorage`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_create_temporary_storage(
    _application_id: u64,
    _owner_id: u64,
    _size: i64,
    _flags: u32,
) -> u32 {
    todo!("fsCreate_TemporaryStorage")
}

/// Stands in for libnx's `fsDeleteSaveDataFileSystem`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_delete_save_data_file_system(_application_id: u64) -> u32 {
    todo!("fsDeleteSaveDataFileSystem")
}

/// Stands in for libnx's `fsDeleteSaveDataFileSystemBySaveDataAttribute`.
///
/// # Safety
///
/// `attr` must point to a readable `FsSaveDataAttribute`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_delete_save_data_file_system_by_save_data_attribute(
    _save_data_space_id: i32,
    _attr: *const c_void,
) -> u32 {
    todo!("fsDeleteSaveDataFileSystemBySaveDataAttribute")
}

/// Stands in for libnx's `fsDeleteSaveDataFileSystemBySaveDataSpaceId`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_delete_save_data_file_system_by_save_data_space_id(
    _save_data_space_id: i32,
    _save_id: u64,
) -> u32 {
    todo!("fsDeleteSaveDataFileSystemBySaveDataSpaceId")
}

/// Stands in for libnx's `fsExtendSaveDataFileSystem`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_extend_save_data_file_system(
    _save_data_space_id: i32,
    _save_id: u64,
    _data_size: i64,
    _journal_size: i64,
) -> u32 {
    todo!("fsExtendSaveDataFileSystem")
}

/// Stands in for libnx's `fsReadSaveDataFileSystemExtraData`.
///
/// # Safety
///
/// `buf` must point to `len` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_read_save_data_file_system_extra_data(
    _buf: *mut c_void,
    _len: usize,
    _save_id: u64,
) -> u32 {
    todo!("fsReadSaveDataFileSystemExtraData")
}

/// Stands in for libnx's `fsReadSaveDataFileSystemExtraDataBySaveDataSpaceId`.
///
/// # Safety
///
/// `buf` must point to `len` writable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_read_save_data_file_system_extra_data_by_save_data_space_id(
    _buf: *mut c_void,
    _len: usize,
    _save_data_space_id: i32,
    _save_id: u64,
) -> u32 {
    todo!("fsReadSaveDataFileSystemExtraDataBySaveDataSpaceId")
}

/// Stands in for libnx's `fsWriteSaveDataFileSystemExtraData`.
///
/// # Safety
///
/// `buf` must point to `len` readable bytes.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_write_save_data_file_system_extra_data(
    _buf: *const c_void,
    _len: usize,
    _save_data_space_id: i32,
    _save_id: u64,
) -> u32 {
    todo!("fsWriteSaveDataFileSystemExtraData")
}

/// Stands in for libnx's `fsDisableAutoSaveDataCreation`.
///
/// # Panics
///
/// Always: the command is not implemented yet.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_fs_disable_auto_save_data_creation() -> u32 {
    todo!("fsDisableAutoSaveDataCreation")
}
