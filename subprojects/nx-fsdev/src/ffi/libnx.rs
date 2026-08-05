//! libnx's `fsdev*` surface.
//!
//! The entry points fall into three groups. Those that mount or unmount go through
//! [`crate::mount`]. Those that act on a path resolve the device out of the path's `"name:"`
//! prefix and hand the remainder to the device, which joins it onto its own working directory.
//! Those that are not implemented panic.
//!
//! A path without a `"name:"` prefix is refused by the path-taking entry points here. The C
//! standard library resolves such a path against the default device before it ever reaches a
//! device, and these entry points are called directly rather than through it, so there is no
//! default to resolve against. libnx falls back to the device holding the process-wide working
//! directory; nothing in this workspace calls these with a bare path.

use core::{
    ffi::{
        CStr,
        c_char,
        c_int,
    },
    mem::MaybeUninit,
};

use nx_service_fs::{
    AccountUid as FsAccountUid,
    CreateOption,
    FsFileSystem,
    FsService,
    SaveDataSpaceId,
};
use nx_sf::{
    error::ToResultCode as _,
    ffi::Service,
    service::DispatchError,
};
use nx_sys_fd::device::{
    DeviceError,
    MAX_DEVICES,
};

use super::common::{
    BAD_INPUT,
    NOT_FOUND,
    OUT_OF_MEMORY,
    SyncUnsafeCell,
};
use crate::{
    device::FsDevice,
    error,
    mount,
    service,
};

/// The user id the save-data mounts are addressed with.
///
/// Declared here rather than borrowed from a service crate because the only thing this boundary
/// needs from it is its ABI: libnx passes it by value, and a 16-byte struct and a pointer are
/// different argument classes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountUid {
    /// The two halves of the id, in the order the C structure holds them.
    pub uid: [u64; 2],
}

impl From<AccountUid> for FsAccountUid {
    fn from(uid: AccountUid) -> Self {
        Self { uid: uid.uid }
    }
}

/// Storage for the filesystem views [`__nx_fsdev__libnx_fsdev_get_device_file_system`] hands back.
///
/// The C prototype returns a pointer, so the value has to outlive the call. One slot per registry
/// slot means a device's view keeps its address for as long as the device holds that slot, which
/// is what a caller holding the pointer expects.
static FILESYSTEM_VIEWS: [SyncUnsafeCell<MaybeUninit<Service>>; MAX_DEVICES] =
    [const { SyncUnsafeCell::new(MaybeUninit::zeroed()) }; MAX_DEVICES];

/// Mounts the SD card as `sdmc:`.
///
/// Corresponds to `fsdevMountSdmc()` in libnx.
///
/// # Safety
///
/// The `fsp-srv` session must have been installed by the runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_sdmc() -> u32 {
    match mount::mount_sdmc() {
        Ok(()) => 0,
        Err(mount::MountSdmcError::NoSession) => NOT_FOUND,
        Err(mount::MountSdmcError::Open(err)) => to_rc(err),
        Err(mount::MountSdmcError::Mount(err)) => mount_error_to_rc(err),
    }
}

/// Mounts `fs` under `name`.
///
/// Corresponds to `fsdevMountDevice()` in libnx, which returns the registry slot the device took
/// or `-1`.
///
/// # Safety
///
/// `name` must be a NUL-terminated string, and `fs` must name a filesystem opened inside the
/// session the runtime installed. The mount takes over closing it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_device(
    name: *const c_char,
    fs: Service,
) -> c_int {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return -1;
    };
    if fs.object_id == 0 {
        return -1;
    }

    let Some(fs_service) = service::get() else {
        return -1;
    };

    // SAFETY: the caller guarantees the id was issued inside this session's domain, and the mount
    // is what closes it from here on.
    let filesystem = FsFileSystem::from_raw_object_id_unchecked(&fs_service, fs.object_id);

    match mount::mount(name, filesystem) {
        Ok(id) => {
            // The registry hands out slots below `MAX_DEVICES`, which is well inside `c_int`.
            mount::set_default_device_if_first(id.index());
            id.index() as c_int
        }
        // The C prototype carries one failure value, so the reason cannot travel with it. A caller
        // that wants the code asks `fsdevGetLastResult`.
        Err(_) => -1,
    }
}

/// Unmounts whatever is mounted under `name`.
///
/// Corresponds to `fsdevUnmountDevice()` in libnx, which returns `0` or `-1`.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_unmount_device(name: *const c_char) -> c_int {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return -1;
    };

    match mount::unmount(name) {
        Ok(()) => 0,
        // Nothing was mounted under the name, which is the only way this fails and the only thing
        // the C prototype's `-1` can say.
        Err(_) => -1,
    }
}

/// Commits every write made through the device mounted under `name`.
///
/// Corresponds to `fsdevCommitDevice()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_commit_device(name: *const c_char) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return NOT_FOUND;
    };
    let Some(device) = mount::find(name) else {
        return NOT_FOUND;
    };

    match device.commit() {
        Ok(()) => 0,
        Err(err) => device_error_rc(err),
    }
}

/// Returns the filesystem the device mounted under `name` is mounted on.
///
/// Corresponds to `fsdevGetDeviceFileSystem()` in libnx, which hands back a pointer into its own
/// device table. The view describes the same object this crate holds: a domain sub-object, with
/// `own_handle` left at zero so a stray `serviceClose` on the C side tears nothing down.
///
/// # Safety
///
/// `name` must be a NUL-terminated string. The returned pointer is valid until the device is
/// unmounted, and the `Service` it addresses must not be closed by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_get_device_file_system(
    name: *const c_char,
) -> *mut Service {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return core::ptr::null_mut();
    };

    filesystem_view(name.to_bytes())
}

/// Splits `path` into the filesystem serving it and the path that filesystem takes.
///
/// Corresponds to `fsdevTranslatePath()` in libnx, which returns `0` or `-1`.
///
/// # Safety
///
/// `path` must be a NUL-terminated string, `device` must be null or writable, and `outpath` must
/// point to a buffer of at least `FS_MAX_PATH` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_translate_path(
    path: *const c_char,
    device: *mut *mut Service,
    outpath: *mut c_char,
) -> c_int {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some((mounted, relative, name)) = (unsafe { split_path(path) }) else {
        return -1;
    };

    // The path was rejected before any command was built; `-1` is the whole contract here, so the
    // reason has nowhere to go.
    let Ok((_, resolved)) = mounted.locate(relative) else {
        return -1;
    };

    if !outpath.is_null() {
        let buf = resolved.as_buf();
        // SAFETY: the caller guarantees `outpath` holds at least `FS_MAX_PATH` bytes, which is
        // exactly what the resolved buffer is.
        unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr().cast::<c_char>(), outpath, buf.len())
        };
    }

    if !device.is_null() {
        // SAFETY: the caller guarantees `device` is writable; the view it receives is the same one
        // `fsdevGetDeviceFileSystem` hands out for this device.
        unsafe { *device = filesystem_view(name) };
    }

    0
}

/// Creates a file of `size` bytes at `path`.
///
/// Corresponds to `fsdevCreateFile()` in libnx.
///
/// # Safety
///
/// `path` must be a NUL-terminated string carrying a `"name:"` prefix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_create_file(
    path: *const c_char,
    size: usize,
    flags: u32,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some((device, relative, _)) = (unsafe { split_path(path) }) else {
        return NOT_FOUND;
    };

    // Both of these are the hard shell: an option bit this crate does not know and a size the
    // command cannot express are the caller's mistakes, and truncating either would dispatch
    // something other than what was asked for.
    let Some(option) = CreateOption::from_bits(flags) else {
        return BAD_INPUT;
    };
    let Ok(size) = i64::try_from(size) else {
        return BAD_INPUT;
    };

    match device.create_file(relative, size, option) {
        Ok(()) => 0,
        Err(err) => device_error_rc(err),
    }
}

/// Removes the directory at `path` and everything under it.
///
/// Corresponds to `fsdevDeleteDirectoryRecursively()` in libnx.
///
/// # Safety
///
/// `path` must be a NUL-terminated string carrying a `"name:"` prefix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_delete_directory_recursively(
    path: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some((device, relative, _)) = (unsafe { split_path(path) }) else {
        return NOT_FOUND;
    };

    match device.remove_dir_all(relative) {
        Ok(()) => 0,
        Err(err) => device_error_rc(err),
    }
}

/// Reports whether the SD card holds a valid signed system partition.
///
/// Corresponds to `fsdevIsValidSignedSystemPartitionOnSdCard()` in libnx, which asks about the
/// device named rather than the card itself. The question only has an answer for the SD card, so a
/// name that is not mounted is refused.
///
/// # Safety
///
/// `name` must be a NUL-terminated string and `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_is_valid_signed_system_partition_on_sd_card(
    name: *const c_char,
    out: *mut bool,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return NOT_FOUND;
    };
    if mount::find(name).is_none() {
        return NOT_FOUND;
    }

    let Some(fs_service) = service::get() else {
        return NOT_FOUND;
    };

    match fs_service.is_signed_system_partition_on_sd_card_valid() {
        Ok(valid) => {
            if !out.is_null() {
                // SAFETY: the caller guarantees `out` is writable.
                unsafe { *out = valid };
            }
            0
        }
        Err(err) => to_rc(err),
    }
}

/// Unmounts every mounted device.
///
/// Corresponds to `fsdevUnmountAll()` in libnx.
///
/// # Safety
///
/// No descriptor opened against any mounted device may still be open.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_unmount_all() -> u32 {
    mount::unmount_all();
    0
}

/// Returns the result code of the most recent failed command.
///
/// Corresponds to `fsdevGetLastResult()` in libnx.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_get_last_result() -> u32 {
    error::last_result()
}

/// Stands in for libnx's `fsdevSetConcatenationFileAttribute`.
///
/// # Safety
///
/// `path` must be a NUL-terminated string.
///
/// # Panics
///
/// Always: this crate does not implement the command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_set_concatenation_file_attribute(
    _path: *const c_char,
) -> u32 {
    todo!("fsdevSetConcatenationFileAttribute")
}

/// Mounts an application's account savedata under `name`.
///
/// Corresponds to `fsdevMountSaveData()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_save_data(
    name: *const c_char,
    application_id: u64,
    uid: AccountUid,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| {
        fs_service.open_account_save_data(application_id, uid.into())
    })
}

/// Mounts an application's account savedata under `name`, for reading only.
///
/// Corresponds to `fsdevMountSaveDataReadOnly()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_save_data_read_only(
    name: *const c_char,
    application_id: u64,
    uid: AccountUid,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| {
        fs_service.open_account_save_data_read_only(application_id, uid.into())
    })
}

/// Mounts an application's BCAT savedata under `name`.
///
/// Corresponds to `fsdevMountBcatSaveData()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_bcat_save_data(
    name: *const c_char,
    application_id: u64,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| {
        fs_service.open_bcat_save_data(application_id)
    })
}

/// Mounts an application's device savedata under `name`.
///
/// Corresponds to `fsdevMountDeviceSaveData()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_device_save_data(
    name: *const c_char,
    application_id: u64,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| {
        fs_service.open_device_save_data(application_id)
    })
}

/// Mounts the temporary storage under `name`.
///
/// Corresponds to `fsdevMountTemporaryStorage()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_temporary_storage(
    name: *const c_char,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| fs_service.open_temporary_storage())
}

/// Mounts one of an application's cache storages under `name`.
///
/// Corresponds to `fsdevMountCacheStorage()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_cache_storage(
    name: *const c_char,
    application_id: u64,
    save_data_index: u16,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| {
        fs_service.open_cache_storage(application_id, save_data_index)
    })
}

/// Mounts a system savedata under `name`.
///
/// Corresponds to `fsdevMountSystemSaveData()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_system_save_data(
    name: *const c_char,
    save_data_space_id: i32,
    system_save_data_id: u64,
    uid: AccountUid,
) -> u32 {
    let Ok(space_id) = SaveDataSpaceId::try_from(save_data_space_id) else {
        return BAD_INPUT;
    };

    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| {
        fs_service.open_system_save_data(space_id, system_save_data_id, uid.into())
    })
}

/// Mounts a system BCAT savedata under `name`.
///
/// Corresponds to `fsdevMountSystemBcatSaveData()` in libnx.
///
/// # Safety
///
/// `name` must be a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_fsdev__libnx_fsdev_mount_system_bcat_save_data(
    name: *const c_char,
    system_save_data_id: u64,
) -> u32 {
    // SAFETY: the caller guarantees a NUL-terminated string.
    let Some(name) = (unsafe { as_cstr(name) }) else {
        return BAD_INPUT;
    };

    mount_save_data(name, |fs_service| {
        fs_service.open_system_bcat_save_data(system_save_data_id)
    })
}

/// Shared body of the savedata mounts, which differ only in which savedata they
/// open.
///
/// Unlike libnx, no firmware gate is applied: the gates live on libnx's
/// `fsOpen_*` wrappers, which this crate does not go through, and a crate below
/// the runtime may not ask which firmware it is on. A command the firmware does
/// not have is answered by the server instead, so the difference is which result
/// code comes back rather than whether the call is refused.
fn mount_save_data(
    name: &CStr,
    open: impl FnOnce(&FsService) -> Result<FsFileSystem<'_>, DispatchError>,
) -> u32 {
    let Some(fs_service) = service::get() else {
        return NOT_FOUND;
    };

    let filesystem = match open(&fs_service) {
        Ok(filesystem) => filesystem,
        Err(err) => return to_rc(err),
    };

    match mount::mount(name, filesystem) {
        Ok(id) => {
            mount::set_default_device_if_first(id.index());
            0
        }
        Err(err) => mount_error_to_rc(err),
    }
}

/// Turns a failed mount into the result code the C caller expects.
fn mount_error_to_rc(err: mount::MountError) -> u32 {
    match err {
        mount::MountError::AlreadyMounted => NOT_FOUND,
        mount::MountError::RegistryFull(_) => OUT_OF_MEMORY,
    }
}

/// Borrows `ptr` as a string, or reports that it is null.
///
/// # Safety
///
/// `ptr` must be null or point to a NUL-terminated string.
unsafe fn as_cstr<'a>(ptr: *const c_char) -> Option<&'a CStr> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees a NUL-terminated string.
    Some(unsafe { CStr::from_ptr(ptr) })
}

/// Splits a prefixed path into the device serving it, the path that device takes, and the name it
/// was reached by.
///
/// # Safety
///
/// `path` must be null or point to a NUL-terminated string.
unsafe fn split_path<'a>(path: *const c_char) -> Option<(&'static FsDevice, &'a CStr, &'a [u8])> {
    // SAFETY: forwarded to this function's own caller.
    let path = unsafe { as_cstr(path) }?;
    let bytes = path.to_bytes();

    let colon = bytes.iter().position(|byte| *byte == b':')?;
    let name = &bytes[..colon];
    let device = mount::find_by_bytes(name)?;

    // SAFETY: `colon` indexes the colon inside `path`, so advancing past it stays within the same
    // allocation and lands on or before the original nul terminator, which still terminates the
    // remainder.
    let relative = unsafe { CStr::from_ptr(path.as_ptr().add(colon + 1)) };

    Some((device, relative, name))
}

/// Returns the view of the filesystem the device named `name` is mounted on, or null.
///
/// The view lives in this module's own storage, indexed by the registry slot the device holds, so
/// the pointer stays valid for as long as the device holds that slot.
fn filesystem_view(name: &[u8]) -> *mut Service {
    let Some(device) = mount::find_by_bytes(name) else {
        return core::ptr::null_mut();
    };
    let Some(object_id) = device.filesystem() else {
        return core::ptr::null_mut();
    };
    let Some(id) = nx_sys_fd::registry::find_by_name_bytes(name) else {
        return core::ptr::null_mut();
    };
    let Some(session) = service::session_handle() else {
        return core::ptr::null_mut();
    };

    let view = Service {
        session,
        own_handle: 0,
        object_id,
        pointer_buffer_size: 0,
    };

    let slot = FILESYSTEM_VIEWS[id.index()].get().cast::<Service>();
    // SAFETY: the slot is storage this module owns, sized and aligned for a `Service`, and the
    // index came from the registry so it is inside the array.
    unsafe { slot.write(view) };

    slot
}

/// Returns the result code a failed device operation reported.
///
/// A failure that reached the server carries its own code, which is what a caller inspecting
/// `fsdevGetLastResult` expects to see. One rejected before that has no such code, so it reports
/// what libnx reports for a bad path.
fn device_error_rc(err: DeviceError) -> u32 {
    match err {
        DeviceError::InvalidPath | DeviceError::Unsupported => BAD_INPUT,
        DeviceError::NotFound | DeviceError::AlreadyExists | DeviceError::Io => {
            error::last_result()
        }
    }
}

/// Returns the result code a failed command reported.
fn to_rc(err: DispatchError) -> u32 {
    err.to_rc()
}
