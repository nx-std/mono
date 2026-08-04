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

use core::{
    cell::UnsafeCell,
    ffi::{
        CStr,
        c_char,
        c_int,
    },
};

use nx_sys_sync::Mutex;

mod ctypes;
mod devoptab;
mod dir_state;
mod errno;
mod handle;
mod path;
mod reent;

use self::{
    devoptab::DevOpTab,
    errno::{
        EBADF,
        ENODEV,
        ENOSYS,
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
    registry,
    table::{
        self,
        Fd,
        MAX_FD,
    },
};

/// Orders access to the per-descriptor C state pointers.
static STATE_LOCK: Mutex = Mutex::new();

/// Per-descriptor state belonging to devices registered from C.
///
/// A C device declares how many bytes each of its descriptors needs and reaches them through the
/// `file_struct` pointer in the descriptor header. Rust devices keep nothing here, so their entries
/// stay null and nothing is allocated for them.
static C_STATES: CStates = CStates(UnsafeCell::new([core::ptr::null_mut(); MAX_FD]));

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
    let path = unsafe { CStr::from_ptr(name) };

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
    let size = devoptab::state_size_at(slot);
    let state = match allocate_c_state(size) {
        Ok(state) => state,
        Err(err) => {
            errno::set_thread_errno(err);
            return -1;
        }
    };

    let fd = match table::open(device) {
        Ok(fd) => fd,
        Err(err) => {
            release_c_state(state, size);
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

    let size = table::device_of(fd).map_or(0, |id| devoptab::state_size_at(id.index()));
    table::take(fd);
    release_c_state(set_c_state(fd.number(), core::ptr::null_mut()), size);
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
    set_c_state(fd.number(), core::ptr::null_mut());

    match result {
        Ok(()) => 0,
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Duplicates `fd` onto the lowest free descriptor.
///
/// Corresponds to libsysbase's `dup`.
// TODO: implement descriptor duplication - this reports ENOSYS, so a caller that duplicates a
//  descriptor gets a failure instead of a second reference to the same open device. Doing it
//  properly means reference counting what a descriptor holds, because two descriptors would then
//  share one device state and only the last close may release it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_dup(_oldfd: c_int) -> c_int {
    errno::set_thread_errno(ENOSYS);
    -1
}

/// Duplicates `oldfd` onto `newfd`, closing whatever `newfd` held.
///
/// Corresponds to libsysbase's `dup2`.
// TODO: implement descriptor duplication - this reports ENOSYS. Beyond the reference counting
//  `dup` needs, whatever is displaced from `newfd` must be released after the table lock is
//  dropped, because closing a device can block or reach back into the table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_fd__libsysbase_dup2(_oldfd: c_int, _newfd: c_int) -> c_int {
    errno::set_thread_errno(ENOSYS);
    -1
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
    let table = devoptab::table_at(device.index());
    let size = devoptab::state_size_at(device.index());

    // Free the descriptor before the device runs, so a device that blocks in close does not hold up
    // the table and the number is reusable either way.
    table::take(fd);
    let state = set_c_state(fd.number(), core::ptr::null_mut());

    let mut ret = 0;
    if !table.is_null() {
        // SAFETY: a registered C table is live for as long as its descriptors are.
        if let Some(close) = unsafe { (*table).close_r } {
            // SAFETY: `state` is the state this descriptor was opened with.
            ret = unsafe { close(r, state.cast()) };
        }
    }

    release_c_state(state, size);
    ret
}

/// Allocates `size` zeroed bytes of C device state, or null when the device declares none.
fn allocate_c_state(size: usize) -> Result<*mut u8, c_int> {
    if size == 0 {
        return Ok(core::ptr::null_mut());
    }

    let Ok(layout) = core::alloc::Layout::from_size_align(size, align_of::<*mut u8>()) else {
        return Err(errno::EINVAL);
    };
    // SAFETY: `layout` has a non-zero size.
    let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
    if ptr.is_null() {
        return Err(errno::ENFILE);
    }

    Ok(ptr)
}

/// Releases C device state allocated by [`allocate_c_state`].
fn release_c_state(state: *mut u8, size: usize) {
    if state.is_null() || size == 0 {
        return;
    }

    let Ok(layout) = core::alloc::Layout::from_size_align(size, align_of::<*mut u8>()) else {
        return;
    };
    // SAFETY: the pointer came from `allocate_c_state` with this layout, and the descriptor that
    // owned it is gone, so nothing can reach it again.
    unsafe { alloc::alloc::dealloc(state, layout) };
}

/// Returns the C state pointer recorded for `fd`.
fn c_state_of(fd: usize) -> *mut u8 {
    STATE_LOCK.lock();
    // SAFETY: the lock is held and `fd` is in range.
    let state = unsafe { (*C_STATES.0.get())[fd] };
    STATE_LOCK.unlock();
    state
}

/// Records `state` as the C state for `fd`, returning what it replaced.
fn set_c_state(fd: usize, state: *mut u8) -> *mut u8 {
    STATE_LOCK.lock();
    // SAFETY: the lock is held and `fd` is in range.
    let previous = unsafe { core::mem::replace(&mut (*C_STATES.0.get())[fd], state) };
    STATE_LOCK.unlock();
    previous
}

/// Storage for the per-descriptor C state pointers.
struct CStates(UnsafeCell<[*mut u8; MAX_FD]>);

// SAFETY: entries are only touched while `STATE_LOCK` is held.
unsafe impl Sync for CStates {}
