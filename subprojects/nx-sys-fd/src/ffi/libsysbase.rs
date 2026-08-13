//! The `libsysbase` override surface.
//!
//! These thirteen symbols replace `handle_manager.c` and `iosupport.c` wholesale, and every one of
//! them is a thin translation of a C convention into a call on the Rust API: descriptor numbers
//! become [`Fd`], registry slots become [`DeviceId`], and a `Result` becomes a return code plus an
//! error number.
//!
//! Replacement is per translation unit. Every global those two files define appears here, including
//! the ones nothing calls, because a symbol left unclaimed keeps its C definition reachable and
//! then two implementations are live at once over separate state.
//!
//! # References
//!
//! - libgloss/libsysbase/handle_manager.c
//! - libgloss/libsysbase/iosupport.c

use alloc::sync::Arc;
use core::{
    cell::UnsafeCell,
    ffi::{
        CStr,
        c_char,
        c_int,
    },
};

use nx_std_path::{
    OsStr,
    Path,
};
use nx_sys_sync::Mutex;

mod ctypes;
mod devoptab;
mod dir_state;
mod errno;
mod handle;
mod reent;

use self::{
    devoptab::DevOpTab,
    errno::{
        EBADF,
        ENODEV,
        ToErrno as _,
    },
    handle::Handle,
    reent::Reent,
};
use crate::{
    device::{
        DeviceId,
        MAX_DEVICES,
    },
    path,
    registry,
    table::{
        self,
        Fd,
        MAX_FD,
    },
};

/// Orders access to the per-descriptor C state.
static STATE_LOCK: Mutex = Mutex::new();

/// Per-descriptor state belonging to devices registered from C.
///
/// A C device declares how many bytes each of its descriptors needs and reaches them through the
/// `file_struct` pointer in the descriptor header. Rust devices keep nothing here, so their entries
/// stay empty and nothing is allocated for them.
///
/// An entry is shared rather than owned by one descriptor, because duplication gives two
/// descriptors one state: they must be handed the same bytes, and the device must be told to close
/// them once rather than once each.
static C_STATES: CStates = CStates(UnsafeCell::new([const { None }; MAX_FD]));

/// Registers a device and returns its registry slot, or -1 when the registry is full.
///
/// Corresponds to libsysbase's `AddDevice`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_AddDevice(device: *const DevOpTab) -> c_int {
    if device.is_null() {
        return -1;
    }

    // SAFETY: the caller guarantees the table and its name outlive its descriptors.
    match unsafe { devoptab::register_c_device(device) } {
        Some(index) => index as c_int,
        None => -1,
    }
}

/// Resolves the `"name:"` prefix of a path to a registry slot, or -1 when unknown.
///
/// Corresponds to libsysbase's `FindDevice`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_FindDevice(name: *const c_char) -> c_int {
    if name.is_null() {
        return -1;
    }

    // SAFETY: the caller guarantees `name` is a live nul-terminated string.
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes();
    let path = Path::new(OsStr::from_bytes(bytes));

    let Some(id) = path::device_for_path(path) else {
        return -1;
    };
    devoptab::ensure_bound(id.index());

    id.as_raw() as c_int
}

/// Unregisters the device a path resolves to, returning 0 on success and -1 when unknown.
///
/// Corresponds to libsysbase's `RemoveDevice`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_RemoveDevice(name: *const c_char) -> c_int {
    // SAFETY: the caller guarantees `name` is a live nul-terminated string.
    let slot = unsafe { __nx_sys_fd__libsysbase_FindDevice(name) };
    if slot < 0 {
        return -1;
    }

    // SAFETY: `slot` was produced by `FindDevice` above, so it names a registry slot.
    registry::unregister(DeviceId::from_index_unchecked(slot as usize));
    // SAFETY: the slot was just resolved and is in range.
    unsafe { devoptab::clear(slot as usize) };

    0
}

/// Returns the device table a path resolves to, or null when unknown.
///
/// Corresponds to libsysbase's `GetDeviceOpTab`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_GetDeviceOpTab(
    name: *const c_char,
) -> *const DevOpTab {
    // SAFETY: the caller guarantees `name` is a live nul-terminated string.
    let slot = unsafe { __nx_sys_fd__libsysbase_FindDevice(name) };
    if slot < 0 {
        return core::ptr::null();
    }

    devoptab::table_at(slot as usize)
}

/// Sets the device that paths without a `"name:"` prefix resolve to.
///
/// Corresponds to libsysbase's `setDefaultDevice`. Slots 0, 1 and 2 belong to the standard
/// descriptors and are rejected, matching the C implementation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_setDefaultDevice(device: c_int) {
    let slot = device as usize;
    if device <= 2 || slot >= MAX_DEVICES {
        return;
    }

    path::set_default_device(slot);
}

/// Opens a descriptor on `device`, returning it or -1 with the error number set.
///
/// Corresponds to libsysbase's `__alloc_handle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_alloc_handle(device: c_int) -> c_int {
    let Some(device) = parse_device(device) else {
        errno::set_thread_errno(ENODEV);
        return -1;
    };
    let slot = device.index();

    // A C device keeps its own per-descriptor state. It is allocated before the descriptor exists,
    // so no slot is held across the allocation and nothing has to be rolled back.
    let state = match create_c_state(slot) {
        Ok(state) => state,
        Err(err) => {
            errno::set_thread_errno(err);
            return -1;
        }
    };

    let fd = match table::open(device) {
        Ok(fd) => fd,
        Err(err) => {
            // The state is released by dropping it here: no descriptor was taken, so nothing else
            // shares it and no device has been told it exists.
            drop(state);
            errno::set_thread_errno(err.to_errno());
            return -1;
        }
    };

    set_c_state(fd.number(), state);
    fd.number() as c_int
}

/// Returns the descriptor header for `fd`, or null when `fd` is not open.
///
/// Corresponds to libsysbase's `__get_handle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_get_handle(fd: c_int) -> *mut Handle {
    let Some(fd) = parse_fd(fd) else {
        return core::ptr::null_mut();
    };

    let Some(device) = table::device_of(fd) else {
        return core::ptr::null_mut();
    };

    devoptab::ensure_bound(device.index());
    handle::project(fd, device.as_raw(), c_state_of(fd.number()))
}

/// Closes `fd` without telling its device.
///
/// Corresponds to libsysbase's `__release_handle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_release_handle(fd: c_int) {
    let Some(fd) = parse_fd(fd) else {
        return;
    };

    table::take(fd);
    // Released rather than closed, as in `table::take`: the state is freed once the last descriptor
    // sharing it lets go, and no device is told.
    drop(set_c_state(fd.number(), None));
}

/// Closes `fd`, telling its device.
///
/// Corresponds to libsysbase's `_close_r`. A C device is told through its own table, because that
/// is where its per-descriptor state belongs; a Rust device is told through the Rust close.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_close_r(r: *mut Reent, fd: c_int) -> c_int {
    let Some(fd) = parse_fd(fd) else {
        return errno::fail(r, EBADF);
    };

    let Some(device) = table::device_of(fd) else {
        return errno::fail(r, EBADF);
    };

    if devoptab::is_c_device(device.index()) {
        return close_c_descriptor(r, fd, device);
    }

    let result = table::close(fd);
    drop(set_c_state(fd.number(), None));

    match result {
        Ok(()) => 0,
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Duplicates `oldfd` onto the lowest free descriptor, returning it or -1 with the error number
/// set.
///
/// Corresponds to libsysbase's `dup`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_dup(oldfd: c_int) -> c_int {
    let Some(oldfd) = parse_fd(oldfd) else {
        errno::set_thread_errno(EBADF);
        return -1;
    };

    // Shared before the descriptor exists, for the same reason a C device's state is allocated
    // before one: no slot is held while it happens, and a refused descriptor drops the share below.
    let state = share_c_state(oldfd.number());

    let fd = match table::duplicate(oldfd) {
        Ok(fd) => fd,
        Err(err) => {
            drop(state);
            errno::set_thread_errno(err.to_errno());
            return -1;
        }
    };

    set_c_state(fd.number(), state);
    fd.number() as c_int
}

/// Duplicates `oldfd` onto `newfd`, closing whatever `newfd` held, and returns `newfd` or -1 with
/// the error number set.
///
/// Corresponds to libsysbase's `dup2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_dup2(oldfd: c_int, newfd: c_int) -> c_int {
    let (Some(oldfd), Some(newfd)) = (parse_fd(oldfd), parse_fd(newfd)) else {
        errno::set_thread_errno(EBADF);
        return -1;
    };

    let state = share_c_state(oldfd.number());

    let displaced = match table::duplicate_to(oldfd, newfd) {
        Ok(displaced) => displaced,
        Err(err) => {
            drop(state);
            errno::set_thread_errno(err.to_errno());
            return -1;
        }
    };

    let displaced_state = set_c_state(newfd.number(), state);

    // What was displaced is released only now, with both locks gone, because a device's close may
    // block or reach back into the table it was displaced from.
    if let Some(device) = displaced.device()
        && devoptab::is_c_device(device.index())
    {
        // A C device is told through its own table, as in the close above.
        close_c_state(errno::thread_reent(), device, displaced_state);
    }

    // A descriptor on a C device holds no file of its own, so this closes a Rust device's file and
    // does nothing otherwise. What the file reported is discarded: this call reports whether the
    // descriptor was rebound, which it was, and not what closing the old one ran into.
    let _ = displaced.close();

    newfd.number() as c_int
}

/// Parses a descriptor number handed in by a C caller.
///
/// Values arriving from C are parsed rather than wrapped: nothing has established that the number
/// names a descriptor slot, and this is the boundary that exists to establish it.
fn parse_fd(fd: c_int) -> Option<Fd> {
    let number = usize::try_from(fd).ok()?;
    Fd::try_from(number).ok()
}

/// Parses a registry slot handed in by a C caller.
///
/// Parsed rather than wrapped, for the same reason as [`parse_fd`].
fn parse_device(device: c_int) -> Option<DeviceId> {
    let index = usize::try_from(device).ok()?;
    DeviceId::try_from(index).ok()
}

/// Closes a descriptor on a C device by calling that device's own close.
fn close_c_descriptor(r: *mut Reent, fd: Fd, device: DeviceId) -> c_int {
    // Free the descriptor before the device runs, so a device that blocks in close does not hold up
    // the table and the number is reusable either way.
    table::take(fd);
    let state = set_c_state(fd.number(), None);

    close_c_state(r, device, state)
}

/// Tells `device` to close `state`, if it was the last descriptor sharing it.
///
/// Returns what the device's close reported, or 0 when there was nothing to tell it: another
/// descriptor still shares the state, the device registered no close, or it is gone.
fn close_c_state(r: *mut Reent, device: DeviceId, state: Option<Arc<CState>>) -> c_int {
    let Some(state) = state.and_then(Arc::into_inner) else {
        return 0;
    };

    let table = devoptab::table_at(device.index());
    if table.is_null() {
        return 0;
    }

    // SAFETY: a registered C table is live for as long as its descriptors are.
    let Some(close) = (unsafe { (*table).close_r }) else {
        return 0;
    };

    // SAFETY: `state` is the state the descriptors sharing it were opened with, and this is the
    // last of them.
    unsafe { close(r, state.as_ptr().cast()) }
}

/// Allocates the state a descriptor on the device in registry slot `index` needs.
///
/// Returns `None` for a device implemented in Rust, which keeps nothing here. A C device gets an
/// entry even when it declares no bytes, because the entry is also what counts the descriptors
/// sharing it, and a device with no state to free still expects exactly one close.
fn create_c_state(index: usize) -> Result<Option<Arc<CState>>, c_int> {
    if !devoptab::is_c_device(index) {
        return Ok(None);
    }

    let state = CState::create(devoptab::state_size_at(index))?;
    Ok(Some(Arc::new(state)))
}

/// Returns the state pointer recorded for `fd`, or null when it has none.
fn c_state_of(fd: usize) -> *mut u8 {
    STATE_LOCK.lock();
    // SAFETY: the lock is held and `fd` is in range.
    let state = unsafe {
        (*C_STATES.0.get())[fd]
            .as_ref()
            .map_or(core::ptr::null_mut(), |state| state.as_ptr())
    };
    STATE_LOCK.unlock();
    state
}

/// Returns another share of the state recorded for `fd`, for a descriptor duplicated from it.
fn share_c_state(fd: usize) -> Option<Arc<CState>> {
    STATE_LOCK.lock();
    // SAFETY: the lock is held and `fd` is in range.
    let state = unsafe { (*C_STATES.0.get())[fd].clone() };
    STATE_LOCK.unlock();
    state
}

/// Records `state` as the state for `fd`, returning what it replaced.
///
/// What comes back travels out of the lock so the caller drops it with the lock gone, which is what
/// keeps a free off it.
fn set_c_state(fd: usize, state: Option<Arc<CState>>) -> Option<Arc<CState>> {
    STATE_LOCK.lock();
    // SAFETY: the lock is held and `fd` is in range.
    let previous = unsafe { core::mem::replace(&mut (*C_STATES.0.get())[fd], state) };
    STATE_LOCK.unlock();
    previous
}

/// Per-descriptor state a C device asked for, owned by the descriptors sharing it.
///
/// Frees the allocation when the last of them lets go. Telling the device happens first and
/// elsewhere ([`close_c_state`]): this owns the memory and nothing else.
struct CState {
    ptr: *mut u8,
    size: usize,
}

// SAFETY: the pointer addresses an allocation this crate never reads or writes. It hands it to the
// device that asked for it and frees it once the last descriptor sharing it is gone, which the
// reference count orders after every other descriptor let go. What the device does with those bytes
// across threads was its own business when the C table held this pointer, and remains so.
unsafe impl Send for CState {}

// SAFETY: a shared reference reaches the pointer value and nothing behind it, so two threads
// holding one share the address of an allocation this crate never reads or writes. What the device
// that asked for those bytes does with them across threads was its own business when the C table
// held this pointer, and remains so.
unsafe impl Sync for CState {}

impl CState {
    /// Allocates `size` zeroed bytes, allocating nothing when the device declares no state.
    ///
    /// # Errors
    ///
    /// Returns the error number to report when the size is one no layout can describe, or when the
    /// allocator has no room for it.
    fn create(size: usize) -> Result<Self, c_int> {
        if size == 0 {
            return Ok(Self {
                ptr: core::ptr::null_mut(),
                size,
            });
        }

        let Ok(layout) = core::alloc::Layout::from_size_align(size, align_of::<*mut u8>()) else {
            return Err(errno::EINVAL);
        };
        // SAFETY: `layout` has a non-zero size.
        let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err(errno::ENFILE);
        }

        Ok(Self { ptr, size })
    }

    /// Returns what the device is handed as its per-descriptor state.
    fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }
}

impl Drop for CState {
    fn drop(&mut self) {
        if self.ptr.is_null() || self.size == 0 {
            return;
        }

        let Ok(layout) = core::alloc::Layout::from_size_align(self.size, align_of::<*mut u8>())
        else {
            return;
        };
        // SAFETY: the pointer came from `create` with this layout, and the last descriptor sharing
        // it is gone, so nothing can reach it again.
        unsafe { alloc::alloc::dealloc(self.ptr, layout) };
    }
}

/// Storage for the per-descriptor C state.
struct CStates(UnsafeCell<[Option<Arc<CState>>; MAX_FD]>);

// SAFETY: entries are only touched while `STATE_LOCK` is held.
unsafe impl Sync for CStates {}
