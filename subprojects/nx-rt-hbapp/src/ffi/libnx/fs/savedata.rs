//! Savedata openers and management commands.
//!
//! Commands without an implementation are aliased to panicking stubs: one
//! left to libnx hangs rather than failing. See the parent module.
//!
//! Struct parameters are typed as opaque pointers, except where
//! [`nx_service_fs`] declares the layout and pins its size: `FsSaveDataAttribute`
//! is that case, so it is named rather than stood in for. Every other one is a
//! pointer, so the ABI is exact either way. [`AccountUid`] is the exception in
//! the other direction: it is passed by value, so its size decides how the
//! arguments after it land.
//!
//! # Firmware gates
//!
//! libnx refuses some of these outright on firmware that predates the command
//! or the save-data kind they name, rather than letting the server answer. The
//! gates are reproduced here so a caller branching on
//! `LibnxError_IncompatSysVer` sees what it saw before. A shaped opener carries
//! the gate of the command it wraps, since the wrapper is what a C caller
//! reaches.

use core::ffi::c_void;

use nx_service_fs::{
    AccountUid as FsAccountUid,
    SaveDataAttribute,
    SaveDataSpaceId,
};
use nx_sf::ffi::Service;

use super::support::open_filesystem;
use crate::{
    env::hos_version::{
        self,
        HosVersion,
    },
    ffi::common::{
        GENERIC_ERROR,
        LibnxError,
        libnx_error,
    },
};

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

impl From<AccountUid> for FsAccountUid {
    fn from(uid: AccountUid) -> Self {
        Self { uid: uid.uid }
    }
}

/// Opens an application's account savedata.
///
/// Corresponds to `fsOpen_SaveData()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_save_data(
    out: *mut Service,
    application_id: u64,
    uid: AccountUid,
) -> u32 {
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_account_save_data(application_id, uid.into())
    })
}

/// Opens an application's account savedata for reading only.
///
/// Corresponds to `fsOpen_SaveDataReadOnly()` in libnx, which reaches the caller
/// through `fsOpenReadOnlySaveDataFileSystem` and so inherits its HOS 2.0.0
/// floor.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_save_data_read_only(
    out: *mut Service,
    application_id: u64,
    uid: AccountUid,
) -> u32 {
    if hos_version::get() < HosVersion::new(2, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_account_save_data_read_only(application_id, uid.into())
    })
}

/// Opens an application's BCAT savedata.
///
/// Corresponds to `fsOpen_BcatSaveData()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_bcat_save_data(
    out: *mut Service,
    application_id: u64,
) -> u32 {
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| service.open_bcat_save_data(application_id))
}

/// Opens an application's device savedata.
///
/// Corresponds to `fsOpen_DeviceSaveData()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_device_save_data(
    out: *mut Service,
    application_id: u64,
) -> u32 {
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| service.open_device_save_data(application_id))
}

/// Opens the temporary storage.
///
/// Corresponds to `fsOpen_TemporaryStorage()` in libnx, which refuses the call
/// before HOS 3.0.0, where the storage does not exist.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_temporary_storage(out: *mut Service) -> u32 {
    if hos_version::get() < HosVersion::new(3, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| service.open_temporary_storage())
}

/// Opens one of an application's cache storages.
///
/// Corresponds to `fsOpen_CacheStorage()` in libnx, which refuses the call
/// before HOS 3.0.0, where the storage does not exist.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_cache_storage(
    out: *mut Service,
    application_id: u64,
    save_data_index: u16,
) -> u32 {
    if hos_version::get() < HosVersion::new(3, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_cache_storage(application_id, save_data_index)
    })
}

/// Opens a system savedata.
///
/// Corresponds to `fsOpen_SystemSaveData()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_system_save_data(
    out: *mut Service,
    save_data_space_id: i32,
    system_save_data_id: u64,
    uid: AccountUid,
) -> u32 {
    let Ok(space_id) = SaveDataSpaceId::try_from(save_data_space_id) else {
        return GENERIC_ERROR;
    };

    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_system_save_data(space_id, system_save_data_id, uid.into())
    })
}

/// Opens a system BCAT savedata.
///
/// Corresponds to `fsOpen_SystemBcatSaveData()` in libnx, which refuses the call
/// before HOS 4.0.0, where the savedata kind does not exist.
///
/// # Safety
///
/// `out` must be null or writable. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_system_bcat_save_data(
    out: *mut Service,
    system_save_data_id: u64,
) -> u32 {
    if hos_version::get() < HosVersion::new(4, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_system_bcat_save_data(system_save_data_id)
    })
}

/// Opens the savedata an attribute names.
///
/// Corresponds to `fsOpenSaveDataFileSystem()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable, and `attr` must be null or point to a
/// readable `FsSaveDataAttribute`. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_save_data_file_system(
    out: *mut Service,
    save_data_space_id: i32,
    attr: *const SaveDataAttribute,
) -> u32 {
    let Ok(space_id) = SaveDataSpaceId::try_from(save_data_space_id) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `attr` is null or points to a readable
    // attribute.
    let Some(attr) = (unsafe { attr.as_ref() }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_save_data_file_system(space_id, attr)
    })
}

/// Opens the system savedata an attribute names.
///
/// Corresponds to `fsOpenSaveDataFileSystemBySystemSaveDataId()` in libnx.
///
/// # Safety
///
/// `out` must be null or writable, and `attr` must be null or point to a
/// readable `FsSaveDataAttribute`. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_save_data_file_system_by_system_save_data_id(
    out: *mut Service,
    save_data_space_id: i32,
    attr: *const SaveDataAttribute,
) -> u32 {
    let Ok(space_id) = SaveDataSpaceId::try_from(save_data_space_id) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `attr` is null or points to a readable
    // attribute.
    let Some(attr) = (unsafe { attr.as_ref() }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_save_data_file_system_by_system_save_data_id(space_id, attr)
    })
}

/// Opens the savedata an attribute names, for reading only.
///
/// Corresponds to `fsOpenReadOnlySaveDataFileSystem()` in libnx, which refuses
/// the call before HOS 2.0.0, where the command does not exist.
///
/// # Safety
///
/// `out` must be null or writable, and `attr` must be null or point to a
/// readable `FsSaveDataAttribute`. On success the caller owes the returned
/// filesystem a `fsFsClose`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_open_read_only_save_data_file_system(
    out: *mut Service,
    save_data_space_id: i32,
    attr: *const SaveDataAttribute,
) -> u32 {
    if hos_version::get() < HosVersion::new(2, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    let Ok(space_id) = SaveDataSpaceId::try_from(save_data_space_id) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `attr` is null or points to a readable
    // attribute.
    let Some(attr) = (unsafe { attr.as_ref() }) else {
        return GENERIC_ERROR;
    };
    // SAFETY: the caller guarantees `out` is null or writable.
    let Some(out) = (unsafe { out.as_mut() }) else {
        return GENERIC_ERROR;
    };

    open_filesystem(out, |service| {
        service.open_read_only_save_data_file_system(space_id, attr)
    })
}

/// Stands in for libnx's `fsCreateSaveDataFileSystem`.
///
/// # Safety
///
/// `attr`, `creation_info` and `meta` must point to readable structs of the
/// matching libnx types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_create_save_data_file_system(
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_create_save_data_file_system_by_system_save_data_id(
    _attr: *const c_void,
    _creation_info: *const c_void,
) -> u32 {
    todo!("fsCreateSaveDataFileSystemBySystemSaveDataId")
}

/// Stands in for libnx's `fsCreate_SystemSaveData`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_fs_create_system_save_data(
    _save_data_space_id: i32,
    _system_save_data_id: u64,
    _size: i64,
    _journal_size: i64,
    _flags: u32,
) -> u32 {
    todo!("fsCreate_SystemSaveData")
}

/// Stands in for libnx's `fsCreate_SystemSaveDataWithOwner`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_fs_create_system_save_data_with_owner(
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
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_fs_create_temporary_storage(
    _application_id: u64,
    _owner_id: u64,
    _size: i64,
    _flags: u32,
) -> u32 {
    todo!("fsCreate_TemporaryStorage")
}

/// Stands in for libnx's `fsDeleteSaveDataFileSystem`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_fs_delete_save_data_file_system(
    _application_id: u64,
) -> u32 {
    todo!("fsDeleteSaveDataFileSystem")
}

/// Stands in for libnx's `fsDeleteSaveDataFileSystemBySaveDataAttribute`.
///
/// # Safety
///
/// `attr` must point to a readable `FsSaveDataAttribute`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_delete_save_data_file_system_by_save_data_attribute(
    _save_data_space_id: i32,
    _attr: *const c_void,
) -> u32 {
    todo!("fsDeleteSaveDataFileSystemBySaveDataAttribute")
}

/// Stands in for libnx's `fsDeleteSaveDataFileSystemBySaveDataSpaceId`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_fs_delete_save_data_file_system_by_save_data_space_id(
    _save_data_space_id: i32,
    _save_id: u64,
) -> u32 {
    todo!("fsDeleteSaveDataFileSystemBySaveDataSpaceId")
}

/// Stands in for libnx's `fsExtendSaveDataFileSystem`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_fs_extend_save_data_file_system(
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_read_save_data_file_system_extra_data(
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_read_save_data_file_system_extra_data_by_save_data_space_id(
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_hbapp__libnx_fs_write_save_data_file_system_extra_data(
    _buf: *const c_void,
    _len: usize,
    _save_data_space_id: i32,
    _save_id: u64,
) -> u32 {
    todo!("fsWriteSaveDataFileSystemExtraData")
}

/// Stands in for libnx's `fsDisableAutoSaveDataCreation`.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_hbapp__libnx_fs_disable_auto_save_data_creation() -> u32 {
    todo!("fsDisableAutoSaveDataCreation")
}
