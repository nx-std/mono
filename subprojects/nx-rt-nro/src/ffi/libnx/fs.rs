//! Filesystem (`fsp-srv`) service FFI.
//!
//! libnx keeps its `fsp-srv` session in a file-local `g_fsSrv`, so a linker
//! script cannot redirect it: `static` gives it internal linkage, and every
//! reference inside `fs.c` was already bound to that definition at compile
//! time. What makes the override work anyway is that *every* reader of
//! `g_fsSrv` is itself an `fs*` function. Redirect the whole set fsdev calls
//! and the C global is simply never read.
//!
//! That is why this module is all-or-nothing over the fsdev-facing surface: a
//! single `fsFs*` left to libnx would dispatch a non-domain request against an
//! object id this crate issued inside a domain, and the server would reject it.
//!
//! # Object ownership across the boundary
//!
//! C holds a `Service` per filesystem, file and directory, and decides when
//! each dies (`fsFsClose` and friends). The Rust wrappers are RAII with a
//! lifetime tied to the session, so each entry point here rebuilds the wrapper
//! from the stored object id, runs one command, and hands the close obligation
//! straight back. Only the `*Close` entry points let the wrapper drop, which is
//! what sends the close.

use core::{
    ffi::c_char,
    mem::MaybeUninit,
};

use nx_rt_core::error::ToResultCode as _;
use nx_service_fs::{
    DirOpenMode,
    DirectoryEntry,
    FS_MAX_PATH,
    FileSystemQueryId,
    FsDir,
    FsFile,
    FsFileSystem,
    OpenMode,
    Priority,
    ReadOption,
    TimeStampRaw,
    WriteOption,
};
use nx_sf::{
    error::ToResultCode as _,
    ffi::Service,
};

use crate::{
    ffi::common::{
        GENERIC_ERROR,
        SyncUnsafeCell,
    },
    services::fs,
};

/// Result code libnx returns for a path that does not fit `FS_MAX_PATH`.
///
/// libnx never checks: it `strncpy`s into a fixed buffer and lets the truncated
/// path reach the server. Rejecting here is the hard shell the C caller does
/// not have; the code is the generic one because no server ever saw the
/// request.
const PATH_TOO_LONG: u32 = GENERIC_ERROR;

/// Backing storage for [`__nx_rt_nro__libnx_fs_get_service_session`], which
/// hands C a pointer rather than a value. Written on `fsInitialize` and zeroed
/// on `fsExit`.
static FS_FFI_SESSION: SyncUnsafeCell<MaybeUninit<Service>> =
    SyncUnsafeCell::new(MaybeUninit::zeroed());

/// Initializes the `fsp-srv` service.
///
/// Corresponds to `fsInitialize()` in libnx.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_initialize() -> u32 {
    if let Err(err) = fs::init() {
        return err.to_rc();
    }

    if let Some(service) = fs::get_service() {
        // Domain root, in libnx's `Service` encoding: a non-zero `object_id`
        // with `own_handle` left at zero, so the view describes the domain
        // without claiming the close.
        let view = Service {
            session: service.session_handle().to_handle(),
            own_handle: 0,
            object_id: service.root_object_id().to_raw(),
            pointer_buffer_size: 0,
        };
        // SAFETY: Called only during initialization; no concurrent readers.
        unsafe { FS_FFI_SESSION.get().cast::<Service>().write(view) };
    }

    0
}

/// Closes the `fsp-srv` service.
///
/// Corresponds to `fsExit()` in libnx.
///
/// # Safety
///
/// No filesystem, file or directory this module handed out may still be in use:
/// closing the session invalidates every object id issued within it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_exit() {
    fs::exit();
    // SAFETY: Called only during exit, after the session has been closed.
    unsafe { FS_FFI_SESSION.get().write(MaybeUninit::zeroed()) };
}

/// Returns the `fsp-srv` service session.
///
/// Corresponds to `fsGetServiceSession()` in libnx, which hands back its
/// `g_fsSrv`. The view describes the same thing this crate owns: a domain whose
/// root is `fsp-srv` itself, carrying the object id the conversion assigned it.
///
/// `own_handle` is zero because the Rust `FsService` keeps the close: a C caller
/// that ran `serviceClose` on an owning snapshot would tear down the session out
/// from under the pool.
///
/// # Safety
///
/// The returned pointer is valid until `fsExit`, and the `Service` it addresses
/// must not be closed by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_get_service_session() -> *mut Service {
    FS_FFI_SESSION.get().cast::<Service>()
}

/// Sets the request priority applied to subsequent `fsp-srv` commands.
///
/// Corresponds to `fsSetPriority()` in libnx, which ignores the request before
/// HOS 5.0.0. The priority rides in the CMIF context word, which older servers
/// do not read, so applying it unconditionally is the same no-op without the
/// version query.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety; an unrecognized priority
/// is ignored.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_set_priority(priority: u32) {
    let priority = match priority {
        0 => Priority::Normal,
        1 => Priority::Realtime,
        2 => Priority::Low,
        3 => Priority::Background,
        _ => return,
    };

    if let Some(service) = fs::get_service() {
        service.set_priority(priority);
    }
}

/// Creates a file.
///
/// Corresponds to `fsFsCreateFile()` in libnx.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a `Service` this module handed out, and
/// `path` must be null or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_create_file(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_delete_file(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_create_directory(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_delete_directory(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_delete_directory_recursively(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_clean_directory_recursively(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_rename_file(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_rename_directory(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_get_entry_type(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_open_file(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_open_directory(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_commit(fs_ptr: *const Service) -> u32 {
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_get_free_space(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_get_total_space(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_get_file_time_stamp_raw(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_set_concatenation_file_attribute(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_is_valid_signed_system_partition_on_sd_card(
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
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_fs_close(fs_ptr: *mut Service) {
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

/// Reads directory entries.
///
/// Corresponds to `fsDirRead()` in libnx.
///
/// # Safety
///
/// `dir` must be null or point to a `Service` this module handed out, `buf`
/// must be writable for `max_entries` entries, and `total_entries` must be null
/// or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_dir_read(
    dir: *const Service,
    total_entries: *mut i64,
    max_entries: u64,
    buf: *mut DirectoryEntry,
) -> u32 {
    if buf.is_null() {
        return GENERIC_ERROR;
    }
    let Ok(len) = usize::try_from(max_entries) else {
        return GENERIC_ERROR;
    };

    // SAFETY: the caller guarantees `buf` holds `max_entries` entries.
    let entries = unsafe { core::slice::from_raw_parts_mut(buf, len) };

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_dir(dir, |d| d.read(entries)) } {
        Ok(count) => {
            if !total_entries.is_null() {
                // SAFETY: null-checked, and the caller guarantees writability.
                unsafe { *total_entries = count };
            }
            0
        }
        Err(rc) => rc,
    }
}

/// Counts the entries in a directory.
///
/// Corresponds to `fsDirGetEntryCount()` in libnx.
///
/// # Safety
///
/// `dir` must be null or point to a `Service` this module handed out, and
/// `count` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_dir_get_entry_count(
    dir: *const Service,
    count: *mut i64,
) -> u32 {
    if count.is_null() {
        return GENERIC_ERROR;
    }

    // SAFETY: the caller guarantees a readable `Service`.
    match unsafe { with_dir(dir, |d| d.get_entry_count()) } {
        Ok(total) => {
            // SAFETY: null-checked, and the caller guarantees writability.
            unsafe { *count = total };
            0
        }
        Err(rc) => rc,
    }
}

/// Closes a directory.
///
/// Corresponds to `fsDirClose()` in libnx.
///
/// # Safety
///
/// `dir` must be null or point to a writable `Service` this module handed out,
/// and must be closed exactly once: a second call would close an object id the
/// server may have since reissued.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_dir_close(dir: *mut Service) {
    // SAFETY: the caller guarantees a readable `Service`.
    let Some(object_id) = (unsafe { object_id_of(dir) }) else {
        return;
    };
    let Some(service) = fs::get_service() else {
        return;
    };

    // SAFETY: as in `fsFsClose` - one close per open, discharged here.
    drop(FsDir::from_raw_object_id_unchecked(&service, object_id));

    // SAFETY: the caller guarantees `dir` is writable.
    unsafe { (*dir).object_id = 0 };
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

/// Copies a C path into the fixed-size buffer every `fsp-srv` path command
/// takes, NUL-terminating it.
///
/// Returns `None` when `path` is null or does not fit, which the caller renders
/// as [`PATH_TOO_LONG`].
///
/// # Safety
///
/// `path` must be null or point to a NUL-terminated string.
unsafe fn copy_path(path: *const c_char) -> Option<[u8; FS_MAX_PATH]> {
    if path.is_null() {
        return None;
    }

    let mut buf = [0u8; FS_MAX_PATH];
    for (i, slot) in buf.iter_mut().enumerate() {
        // SAFETY: the caller guarantees a NUL-terminated string, so the walk
        // stops at or before the terminator; `i` never passes it.
        let byte = unsafe { *path.add(i) };
        if byte == 0 {
            return Some(buf);
        }
        *slot = byte;
    }

    // The loop filled every byte without meeting a NUL, so the path needs at
    // least FS_MAX_PATH bytes plus a terminator.
    None
}

/// Builds the non-owning `Service` view C stores for a domain sub-object.
///
/// `own_handle = 0` with a non-zero `object_id` is libnx's domain-subservice
/// encoding. The zero says this crate keeps the close obligation, so a stray
/// `serviceClose` on the C side tears nothing down.
fn sub_object_view(session: nx_svc::ipc::Handle, object_id: u32) -> Service {
    Service {
        session,
        own_handle: 0,
        object_id,
        pointer_buffer_size: 0,
    }
}

/// Reads the domain object id C stored in a `Service`.
///
/// # Safety
///
/// `service` must be null or point to a readable `Service`.
unsafe fn object_id_of(service: *const Service) -> Option<u32> {
    if service.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees a readable `Service`.
    let object_id = unsafe { (*service).object_id };
    (object_id != 0).then_some(object_id)
}

/// Runs one command against the filesystem `fs` names.
///
/// The wrapper is rebuilt for the call and gives the close obligation back
/// before returning, so the object outlives it; only `fsFsClose` closes.
///
/// # Safety
///
/// `fs` must be null or point to a readable `Service`.
unsafe fn with_filesystem<R>(
    fs: *const Service,
    f: impl FnOnce(&FsFileSystem<'_>) -> Result<R, nx_sf::service::DispatchError>,
) -> Result<R, u32> {
    // SAFETY: forwarded to this function's own caller.
    let Some(object_id) = (unsafe { object_id_of(fs) }) else {
        return Err(GENERIC_ERROR);
    };
    let Some(service) = fs::get_service() else {
        return Err(GENERIC_ERROR);
    };

    // SAFETY: `object_id` came from a `Service` this module handed out, so the
    // server issued it inside this session's domain and only a `*Close` entry
    // point closes it. The obligation is handed straight back below.
    let wrapper = FsFileSystem::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&wrapper);
    let _ = wrapper.into_raw_object_id();

    result.map_err(|err| err.to_rc())
}

/// Runs one command against the file `f` names. See [`with_filesystem`].
///
/// # Safety
///
/// `file` must be null or point to a readable `Service`.
unsafe fn with_file<R>(
    file: *const Service,
    f: impl FnOnce(&FsFile<'_>) -> Result<R, nx_sf::service::DispatchError>,
) -> Result<R, u32> {
    // SAFETY: forwarded to this function's own caller.
    let Some(object_id) = (unsafe { object_id_of(file) }) else {
        return Err(GENERIC_ERROR);
    };
    let Some(service) = fs::get_service() else {
        return Err(GENERIC_ERROR);
    };

    // SAFETY: as in `with_filesystem`.
    let wrapper = FsFile::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&wrapper);
    let _ = wrapper.into_raw_object_id();

    result.map_err(|err| err.to_rc())
}

/// Runs one command against the directory `dir` names. See [`with_filesystem`].
///
/// # Safety
///
/// `dir` must be null or point to a readable `Service`.
unsafe fn with_dir<R>(
    dir: *const Service,
    f: impl FnOnce(&FsDir<'_>) -> Result<R, nx_sf::service::DispatchError>,
) -> Result<R, u32> {
    // SAFETY: forwarded to this function's own caller.
    let Some(object_id) = (unsafe { object_id_of(dir) }) else {
        return Err(GENERIC_ERROR);
    };
    let Some(service) = fs::get_service() else {
        return Err(GENERIC_ERROR);
    };

    // SAFETY: as in `with_filesystem`.
    let wrapper = FsDir::from_raw_object_id_unchecked(&service, object_id);
    let result = f(&wrapper);
    let _ = wrapper.into_raw_object_id();

    result.map_err(|err| err.to_rc())
}

/// Renders a `Result<(), u32>` as the bare result code C expects.
fn to_rc(result: Result<(), u32>) -> u32 {
    match result {
        Ok(()) => 0,
        Err(rc) => rc,
    }
}

/// Shared body of the two space queries, which differ only in the command they
/// send.
///
/// # Safety
///
/// `fs_ptr` must be null or point to a readable `Service`, `path` must be null
/// or NUL-terminated, and `out` must be null or writable.
unsafe fn fs_space_query(
    fs_ptr: *const Service,
    path: *const c_char,
    out: *mut i64,
    query: impl FnOnce(
        &FsFileSystem<'_>,
        &[u8; FS_MAX_PATH],
    ) -> Result<i64, nx_sf::service::DispatchError>,
) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }
    // SAFETY: forwarded to this function's own caller.
    let Some(path) = (unsafe { copy_path(path) }) else {
        return PATH_TOO_LONG;
    };

    // SAFETY: forwarded to this function's own caller.
    match unsafe { with_filesystem(fs_ptr, |fs| query(fs, &path)) } {
        Ok(space) => {
            // SAFETY: `out` was null-checked above and the caller guarantees it
            // is writable.
            unsafe { *out = space };
            0
        }
        Err(rc) => rc,
    }
}
