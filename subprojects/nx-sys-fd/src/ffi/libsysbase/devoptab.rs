//! The C device operation table, in both directions.
//!
//! Devices exist on both sides of the boundary, so this module carries each across:
//!
//! - **Rust device, C caller.** The C entry points left in place dispatch through `devoptab_list`
//!   themselves, so a Rust device needs a table for them to find. [`SHIM_TABLES`] is that table:
//!   one set of shims shared by every slot, which recover the descriptor from the state pointer
//!   they are handed and forward into [`crate::table`].
//! - **C device, Rust caller.** A device registered through `AddDevice` arrives as a `devoptab_t`.
//!   [`CDevice`] wraps one so the registry holds a [`Device`] like any other, and `devoptab_list`
//!   keeps pointing at the original table so C-to-C dispatch is untouched.
//!
//! Neither direction is visible to a device author: a Rust device implements [`Device`], and a C
//! device keeps working unchanged.

use core::{
    cell::UnsafeCell,
    ffi::{
        CStr,
        c_char,
        c_int,
        c_long,
        c_void,
    },
};

use super::{
    errno::ToErrno as _,
    handle,
    reent::Reent,
};
use crate::{
    device::{
        Device,
        DeviceId,
        MAX_DEVICES,
    },
    registry,
    table,
};

/// File offset, matching the C library's `off_t`.
pub type OffT = c_long;

/// File mode, matching the C library's `mode_t`.
pub type ModeT = u32;

/// Signed byte count, matching the C library's `ssize_t`.
pub type SsizeT = c_long;

/// Opaque `struct stat`.
#[repr(C)]
pub struct Stat {
    _opaque: [u8; 0],
}

/// Opaque `struct statvfs`.
#[repr(C)]
pub struct StatVfs {
    _opaque: [u8; 0],
}

/// Opaque `struct timeval`.
#[repr(C)]
pub struct TimeVal {
    _opaque: [u8; 0],
}

/// Directory iteration state carried between `dir*` calls.
///
/// Mirrors `DIR_ITER` from `sys/iosupport.h`.
#[repr(C)]
pub struct DirIter {
    /// Registry slot of the device backing this iterator.
    pub device: c_int,
    /// The device's private directory state.
    pub dir_struct: *mut c_void,
}

/// Operations implementing one device, as C declares them.
///
/// Mirrors `devoptab_t` from `sys/iosupport.h`. Field order is fixed by that declaration and must
/// not be rearranged.
#[repr(C)]
pub struct DevOpTab {
    pub name: *const c_char,
    pub struct_size: usize,

    pub open_r:
        Option<unsafe extern "C" fn(*mut Reent, *mut c_void, *const c_char, c_int, c_int) -> c_int>,
    pub close_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void) -> c_int>,
    pub write_r:
        Option<unsafe extern "C" fn(*mut Reent, *mut c_void, *const c_char, usize) -> SsizeT>,
    pub read_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void, *mut c_char, usize) -> SsizeT>,
    pub seek_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void, OffT, c_int) -> OffT>,
    pub fstat_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void, *mut Stat) -> c_int>,
    pub stat_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, *mut Stat) -> c_int>,
    pub link_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, *const c_char) -> c_int>,
    pub unlink_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char) -> c_int>,
    pub chdir_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char) -> c_int>,
    pub rename_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, *const c_char) -> c_int>,
    pub mkdir_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, c_int) -> c_int>,

    pub dir_state_size: usize,

    pub diropen_r:
        Option<unsafe extern "C" fn(*mut Reent, *mut DirIter, *const c_char) -> *mut DirIter>,
    pub dirreset_r: Option<unsafe extern "C" fn(*mut Reent, *mut DirIter) -> c_int>,
    pub dirnext_r:
        Option<unsafe extern "C" fn(*mut Reent, *mut DirIter, *mut c_char, *mut Stat) -> c_int>,
    pub dirclose_r: Option<unsafe extern "C" fn(*mut Reent, *mut DirIter) -> c_int>,
    pub statvfs_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, *mut StatVfs) -> c_int>,
    pub ftruncate_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void, OffT) -> c_int>,
    pub fsync_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void) -> c_int>,

    pub device_data: *mut c_void,

    pub chmod_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, ModeT) -> c_int>,
    pub fchmod_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void, ModeT) -> c_int>,
    pub rmdir_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char) -> c_int>,
    pub lstat_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, *mut Stat) -> c_int>,
    pub utimes_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, *const TimeVal) -> c_int>,

    pub fpathconf_r: Option<unsafe extern "C" fn(*mut Reent, *mut c_void, c_int) -> c_long>,
    pub pathconf_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, c_int) -> c_long>,

    pub symlink_r: Option<unsafe extern "C" fn(*mut Reent, *const c_char, *const c_char) -> c_int>,
    pub readlink_r:
        Option<unsafe extern "C" fn(*mut Reent, *const c_char, *mut c_char, usize) -> SsizeT>,
}

// Every field is a pointer or a pointer-sized integer, so the C declaration is 32 words wide.
// A mismatch would silently shift every operation the C entry points call.
static_assertions::assert_eq_size!(DevOpTab, [usize; 32]);

// SAFETY: a table is written once during registration, under the registry lock, and only read
// afterwards.
unsafe impl Sync for DevOpTab {}

/// The tables the C entry points dispatch through, one per registry slot.
///
/// A Rust device's slot carries the shims below; a C device's slot carries the device's own table,
/// so nothing about C-to-C dispatch changes.
///
/// Corresponds to libsysbase's `devoptab_list`. The C entry points load entries from this array
/// directly, so its element type and length are fixed by the C declaration and it cannot be wrapped
/// in anything that would add a field.
#[unsafe(no_mangle)]
pub static mut __nx_sys_fd__libsysbase_devoptab_list: [*const DevOpTab; MAX_DEVICES] =
    [core::ptr::null(); MAX_DEVICES];

/// The null device table, as C declares it.
///
/// Corresponds to libsysbase's `dotab_stdnull`. Nothing here points at it: the standard slots are
/// served by a Rust device that discards writes, so this exists because the C translation unit
/// being replaced defines the symbol, and a global left unclaimed keeps that unit reachable.
#[unsafe(no_mangle)]
pub static __nx_sys_fd__libsysbase_dotab_stdnull: DevOpTab = DevOpTab {
    name: c"stdnull".as_ptr(),
    write_r: Some(discard_write),
    ..empty_table()
};

/// One adapter per registry slot, so registering a C device allocates nothing.
static C_DEVICES: [CDevice; MAX_DEVICES] = [const { CDevice::vacant() }; MAX_DEVICES];

/// The shim table a Rust device's slot points at.
///
/// Every slot shares it: the shims recover the descriptor from the state pointer they are handed,
/// so they need no per-slot identity of their own.
static SHIM_TABLE: DevOpTab = DevOpTab {
    close_r: Some(shim_close),
    write_r: Some(shim_write),
    read_r: Some(shim_read),
    ..empty_table()
};

/// Points a registered Rust device's slot at the shim table, so C has something to dispatch
/// through.
///
/// Registration happens on the Rust side, which knows nothing about `devoptab_list`, so the slot is
/// filled in here instead: on every path where C is about to read the array. A C device is left
/// alone, since its slot already points at its own table.
pub fn ensure_bound(index: usize) {
    if index >= MAX_DEVICES || !table_at(index).is_null() {
        return;
    }
    // SAFETY: `index` was checked against `MAX_DEVICES` at the top of this function.
    if registry::get(DeviceId::from_index_unchecked(index)).is_none() {
        return;
    }

    // SAFETY: the slot is in range and holds a Rust device that has no table yet.
    unsafe { bind_shim(index) };
}

/// Points slot `index` at a C device's own table.
///
/// # Safety
///
/// `table` must outlive every descriptor opened against it, and the caller must hold the registry
/// lock.
pub unsafe fn bind_c_table(index: usize, table: *const DevOpTab) {
    // SAFETY: the caller holds the registry lock and `index` is in range.
    unsafe { (*core::ptr::addr_of_mut!(__nx_sys_fd__libsysbase_devoptab_list))[index] = table };
}

/// Clears slot `index`.
///
/// # Safety
///
/// The caller must hold the registry lock.
pub unsafe fn clear(index: usize) {
    // SAFETY: the caller holds the registry lock and `index` is in range.
    unsafe {
        (*core::ptr::addr_of_mut!(__nx_sys_fd__libsysbase_devoptab_list))[index] = core::ptr::null()
    };
}

/// Returns the table at slot `index`, or null when nothing is registered there.
pub fn table_at(index: usize) -> *const DevOpTab {
    if index >= MAX_DEVICES {
        return core::ptr::null();
    }
    // SAFETY: entries are pointer-sized and only written under the registry lock.
    unsafe { (*core::ptr::addr_of_mut!(__nx_sys_fd__libsysbase_devoptab_list))[index] }
}

/// A device registered from C, seen as a [`Device`].
///
/// Reports the device's name so paths resolve, and leaves the transfer operations unsupported: a C
/// device's bytes move through its own table, which its descriptors keep pointing at, and its
/// per-descriptor state is not something the Rust interface can produce.
pub struct CDevice {
    table: UnsafeCell<*const DevOpTab>,
}

// SAFETY: the pointer is written under the registry lock during registration and only read after.
unsafe impl Sync for CDevice {}

impl CDevice {
    /// Returns an adapter with no table yet.
    const fn vacant() -> Self {
        Self {
            table: UnsafeCell::new(core::ptr::null()),
        }
    }

    /// Points this adapter at `table`.
    ///
    /// # Safety
    ///
    /// `table` must outlive every descriptor opened against it, and the caller must hold the
    /// registry lock.
    unsafe fn bind(&self, table: *const DevOpTab) {
        // SAFETY: the caller holds the registry lock.
        unsafe { *self.table.get() = table };
    }

    /// Returns the table this adapter describes.
    fn table(&self) -> *const DevOpTab {
        // SAFETY: the pointer is only written during registration, under the registry lock.
        unsafe { *self.table.get() }
    }
}

impl Device for CDevice {
    fn name(&self) -> &'static CStr {
        let table = self.table();
        if table.is_null() {
            return c"";
        }
        // SAFETY: a bound table is live and its name outlives it.
        unsafe { CStr::from_ptr((*table).name) }
    }
}

/// Registers a C device table, returning the slot it took.
///
/// # Safety
///
/// `table` must outlive every descriptor opened against it, and its `name` must be a live
/// nul-terminated string.
pub unsafe fn register_c_device(table: *const DevOpTab) -> Option<usize> {
    // SAFETY: the caller guarantees the table and its name are live.
    let name = unsafe { CStr::from_ptr((*table).name) };
    let index = registry::free_slot_for(name)?;

    // SAFETY: the caller guarantees the table outlives its descriptors.
    unsafe { C_DEVICES[index].bind(table) };
    registry::bind_at(index, &C_DEVICES[index]);
    // SAFETY: registration owns the slot it just claimed.
    unsafe { bind_c_table(index, table) };

    Some(index)
}

/// Returns the per-descriptor state size a C device declares, or 0 for a Rust device.
pub fn state_size_at(index: usize) -> usize {
    let table = table_at(index);
    if table.is_null() || core::ptr::eq(table, &raw const SHIM_TABLE) {
        return 0;
    }
    // SAFETY: a non-null, non-shim entry is a live C table.
    unsafe { (*table).struct_size }
}

/// Returns whether slot `index` dispatches through a C device's own table.
pub fn is_c_device(index: usize) -> bool {
    let table = table_at(index);
    !table.is_null() && !core::ptr::eq(table, &raw const SHIM_TABLE)
}

/// Returns a table with no operations, for another to fill in the few it implements.
const fn empty_table() -> DevOpTab {
    DevOpTab {
        name: c"".as_ptr(),
        struct_size: 0,
        open_r: None,
        close_r: None,
        write_r: None,
        read_r: None,
        seek_r: None,
        fstat_r: None,
        stat_r: None,
        link_r: None,
        unlink_r: None,
        chdir_r: None,
        rename_r: None,
        mkdir_r: None,
        dir_state_size: 0,
        diropen_r: None,
        dirreset_r: None,
        dirnext_r: None,
        dirclose_r: None,
        statvfs_r: None,
        ftruncate_r: None,
        fsync_r: None,
        device_data: core::ptr::null_mut(),
        chmod_r: None,
        fchmod_r: None,
        rmdir_r: None,
        lstat_r: None,
        utimes_r: None,
        fpathconf_r: None,
        pathconf_r: None,
        symlink_r: None,
        readlink_r: None,
    }
}

/// Reports every byte as written without looking at them.
unsafe extern "C" fn discard_write(
    _r: *mut Reent,
    _state: *mut c_void,
    _buf: *const c_char,
    len: usize,
) -> SsizeT {
    len as SsizeT
}

/// Points slot `index` at the shim table, so C can dispatch to a Rust device.
///
/// # Safety
///
/// `index` must be in range.
unsafe fn bind_shim(index: usize) {
    // SAFETY: the caller holds the registry lock and `index` is in range.
    unsafe {
        (*core::ptr::addr_of_mut!(__nx_sys_fd__libsysbase_devoptab_list))[index] =
            &raw const SHIM_TABLE
    };
}

/// Forwards a C write to the Rust device behind the descriptor.
unsafe extern "C" fn shim_write(
    r: *mut Reent,
    state: *mut c_void,
    buf: *const c_char,
    len: usize,
) -> SsizeT {
    let Some(fd) = handle::fd_from_state(state) else {
        return super::errno::fail_ssize(r, super::errno::EBADF);
    };
    if buf.is_null() {
        return super::errno::fail_ssize(r, super::errno::EINVAL);
    }

    // SAFETY: C guarantees `buf` addresses `len` readable bytes for the duration of the call.
    let bytes = unsafe { core::slice::from_raw_parts(buf.cast::<u8>(), len) };

    match table::write(fd, bytes) {
        Ok(written) => written as SsizeT,
        Err(err) => super::errno::fail_ssize(r, err.to_errno()),
    }
}

/// Forwards a C read to the Rust device behind the descriptor.
unsafe extern "C" fn shim_read(
    r: *mut Reent,
    state: *mut c_void,
    buf: *mut c_char,
    len: usize,
) -> SsizeT {
    let Some(fd) = handle::fd_from_state(state) else {
        return super::errno::fail_ssize(r, super::errno::EBADF);
    };
    if buf.is_null() {
        return super::errno::fail_ssize(r, super::errno::EINVAL);
    }

    // SAFETY: C guarantees `buf` addresses `len` writable bytes for the duration of the call.
    let bytes = unsafe { core::slice::from_raw_parts_mut(buf.cast::<u8>(), len) };

    match table::read(fd, bytes) {
        Ok(read) => read as SsizeT,
        Err(err) => super::errno::fail_ssize(r, err.to_errno()),
    }
}

/// Forwards a C close to the Rust device behind the descriptor.
///
/// The descriptor was already released by `_close_r`, which is what calls this, so there is nothing
/// left to do here beyond reporting.
unsafe extern "C" fn shim_close(_r: *mut Reent, _state: *mut c_void) -> c_int {
    0
}
