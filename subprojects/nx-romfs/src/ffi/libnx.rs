//! libnx's `romfs*` surface.
//!
//! Each entry point names a source, resolves whatever the C caller handed over into something
//! [`crate::mount`] takes, and reports a libnx result code. None of them decides *which* source a
//! program's own image lives in; that is `romfsMountSelf`, and it lives in the runtime.
//!
//! ## Objects arrive by value
//!
//! libnx passes an open file or storage as a one-field structure wrapping a `Service`, so the two
//! have the same argument class as a bare `Service` and are declared as one here. The object id
//! inside it names a domain object in the session the runtime installed, which is what makes it
//! usable at all: an id from anywhere else names nothing, and the mount that adopts it would close
//! something it never opened.

use core::ffi::{
    CStr,
    c_char,
};

use nx_service_fs::{
    FsFile,
    FsStorage,
    NcmStorageId,
};
use nx_sf::ffi::Service;
use nx_std_path::{
    OsStr,
    Path,
};

use super::common::{
    BAD_INPUT,
    IO_ERROR,
    NOT_FOUND,
    OUT_OF_MEMORY,
};
use crate::mount::{
    self,
    MountError,
    OpenError,
    PathMountError,
};

/// Mounts the image `offset` bytes into the file `file` names, under `name`.
///
/// Corresponds to `romfsMountFromFile()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string, and `file` must name a file opened inside the session
/// the runtime installed. The mount takes over closing it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_romfs__libnx_romfs_mount_from_file(
    file: Service,
    offset: u64,
    name: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { borrow_name(name) }) else {
        return BAD_INPUT;
    };
    if file.object_id == 0 {
        return BAD_INPUT;
    }

    let Some(service) = nx_fsdev::service::get() else {
        return NOT_FOUND;
    };

    // SAFETY: the caller guarantees the id was issued inside this session's domain, and the mount
    // is what closes it from here on.
    let file = FsFile::from_raw_object_id_unchecked(&service, file.object_id);

    mount_result(mount::from_fs_file(name, file, offset))
}

/// Mounts the image `offset` bytes into the storage `storage` names, under `name`.
///
/// Corresponds to `romfsMountFromStorage()` in libnx.
///
/// # Safety
///
/// The same as [`__nx_romfs__libnx_romfs_mount_from_file`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_romfs__libnx_romfs_mount_from_storage(
    storage: Service,
    offset: u64,
    name: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { borrow_name(name) }) else {
        return BAD_INPUT;
    };
    if storage.object_id == 0 {
        return BAD_INPUT;
    }

    let Some(service) = nx_fsdev::service::get() else {
        return NOT_FOUND;
    };

    // SAFETY: as in `__nx_romfs__libnx_romfs_mount_from_file`.
    let storage = FsStorage::from_raw_object_id_unchecked(&service, storage.object_id);

    mount_result(mount::from_storage(name, storage, offset))
}

/// Mounts the running program's own data partition under `name`.
///
/// Corresponds to `romfsMountFromCurrentProcess()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_romfs__libnx_romfs_mount_from_current_process(
    name: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { borrow_name(name) }) else {
        return BAD_INPUT;
    };

    open_result(mount::from_current_process(name))
}

/// Mounts the data partition of the program `program_id` names, under `name`.
///
/// Corresponds to `romfsMountDataStorageFromProgram()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_romfs__libnx_romfs_mount_data_storage_from_program(
    program_id: u64,
    name: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { borrow_name(name) }) else {
        return BAD_INPUT;
    };

    open_result(mount::from_program(name, program_id))
}

/// Mounts the image `offset` bytes into the file at `path`, under `name`.
///
/// Corresponds to `romfsMountFromFsdev()` in libnx, which resolves the path against the filesystem
/// devices only. Here it resolves against every mounted device, because the descriptor table makes
/// no distinction and neither does the file this ends up reading.
///
/// # Safety
///
/// `path` and `name` must be NUL-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_romfs__libnx_romfs_mount_from_fsdev(
    path: *const c_char,
    offset: u64,
    name: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees NUL-terminated strings.
    let Some(name) = (unsafe { borrow_name(name) }) else {
        return BAD_INPUT;
    };
    // SAFETY: as above.
    let Some(path) = (unsafe { borrow_bytes(path) }) else {
        return BAD_INPUT;
    };

    let path = Path::new(OsStr::from_bytes(path));
    match mount::from_device_path(name, path, offset) {
        Ok(()) => 0,
        Err(PathMountError::NoDevice) => BAD_INPUT,
        Err(PathMountError::Open(_)) => NOT_FOUND,
        Err(PathMountError::Mount(err)) => mount_error_rc(err),
    }
}

/// Mounts the system data archive `data_id` names on `storage_id`, under `name`.
///
/// Corresponds to `romfsMountFromDataArchive()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_romfs__libnx_romfs_mount_from_data_archive(
    data_id: u64,
    storage_id: u32,
    name: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { borrow_name(name) }) else {
        return BAD_INPUT;
    };
    // A storage the command does not name is the caller's mistake, and passing it through would
    // address an archive on whichever storage the server made of the stray value.
    let Some(storage_id) = storage_id_for(storage_id) else {
        return BAD_INPUT;
    };

    open_result(mount::from_data_archive(name, data_id, storage_id))
}

/// Unmounts whatever is mounted under `name`.
///
/// Corresponds to `romfsUnmount()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_romfs__libnx_romfs_unmount(name: *const c_char) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { borrow_name(name) }) else {
        return BAD_INPUT;
    };

    match mount::unmount(name) {
        Ok(()) => 0,
        Err(_) => NOT_FOUND,
    }
}

/// Returns the result code for a mount that took a source it was handed.
fn mount_result(result: Result<(), MountError>) -> u32 {
    match result {
        Ok(()) => 0,
        Err(err) => mount_error_rc(err),
    }
}

/// Returns the result code for a mount that opened its own source first.
fn open_result(result: Result<(), OpenError>) -> u32 {
    match result {
        Ok(()) => 0,
        Err(OpenError::NoSession) => NOT_FOUND,
        // The server's own code, which is what libnx passes through here and what a caller
        // inspecting the result expects to see.
        Err(OpenError::Open(err)) => {
            use nx_sf::error::ToResultCode as _;
            err.to_rc()
        }
        Err(OpenError::Mount(err)) => mount_error_rc(err),
    }
}

/// Returns the result code a failed mount reported.
fn mount_error_rc(err: MountError) -> u32 {
    match err {
        // libnx has no code for "that name is taken" and answers a full mount table with
        // out-of-memory, which is the nearest thing a caller can act on.
        MountError::AlreadyMounted | MountError::Registry(_) => OUT_OF_MEMORY,
        MountError::Image(_) => IO_ERROR,
    }
}

/// Returns the storage `id` names, or nothing when it names none.
fn storage_id_for(id: u32) -> Option<NcmStorageId> {
    match id {
        0 => Some(NcmStorageId::None),
        1 => Some(NcmStorageId::Host),
        2 => Some(NcmStorageId::GameCard),
        3 => Some(NcmStorageId::BuiltinSystem),
        4 => Some(NcmStorageId::BuiltinUser),
        5 => Some(NcmStorageId::SdCard),
        6 => Some(NcmStorageId::Any),
        _ => None,
    }
}

/// Borrows `ptr` as the bytes between it and its terminator, or reports that it is null.
///
/// This is where the terminator stops travelling: what comes out is the string itself, and nothing
/// below is handed a pointer again.
///
/// # Safety
///
/// `ptr` must be null or point to a NUL-terminated string.
unsafe fn borrow_bytes<'a>(ptr: *const c_char) -> Option<&'a [u8]> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees a NUL-terminated string.
    Some(unsafe { CStr::from_ptr(ptr) }.to_bytes())
}

/// Borrows `ptr` as a device name, or reports that it is null or not text.
///
/// A device registers under a name that is text, so a name that is not UTF-8 matches nothing. It is
/// read as text here so the lookups below compare names rather than raw bytes.
///
/// # Safety
///
/// `ptr` must be null or point to a NUL-terminated string.
unsafe fn borrow_name<'a>(ptr: *const c_char) -> Option<&'a str> {
    // SAFETY: forwarded to this function's own caller.
    let bytes = unsafe { borrow_bytes(ptr) }?;
    core::str::from_utf8(bytes).ok()
}
