//! The C device operation table, in both directions.
//!
//! Devices exist on both sides of the boundary, so this module carries each across:
//!
//! - **Rust device, C caller.** The C entry points left in place dispatch through `devoptab_list`
//!   themselves, so a Rust device needs a table for them to find. [`SHIM_TABLE`] is that table:
//!   one set of shims shared by every slot, which recover what they were called for and forward
//!   into the Rust API.
//! - **C device, Rust caller.** A device registered through `AddDevice` arrives as a `devoptab_t`.
//!   [`CDevice`] wraps one so the registry holds a [`Device`] like any other, and `devoptab_list`
//!   keeps pointing at the original table so C-to-C dispatch is untouched.
//!
//! Neither direction is visible to a device author: a Rust device implements [`Device`], and a C
//! device keeps working unchanged.
//!
//! ## How a shim knows what it was called for
//!
//! C dispatches with `devoptab_list[dev]->op(...)`, and relies on the function pointer itself being
//! device-specific. Every Rust device shares one set of shims, so that identity has to come from
//! the arguments instead, and it arrives differently for each of the three kinds of operation:
//!
//! - **Per-descriptor** operations are handed the descriptor's state pointer, which for a Rust
//!   device addresses that descriptor's entry in the tag array, so the descriptor number is
//!   recovered from the address. See [`super::handle`].
//! - **Per-path** operations are handed only the path, which is where the device name came from in
//!   the first place, so the shim resolves it again. See [`super::path`].
//! - **Per-directory** operations are handed the iterator, which carries the open directory in the
//!   private state the C caller allocated behind it. See [`dir_state`].

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
    ctypes::{
        DirIter,
        ModeT,
        OffT,
        SsizeT,
        Stat,
        StatVfs,
        TimeVal,
        decode_open_flags,
    },
    dir_state,
    errno::{
        self,
        EBADF,
        EINVAL,
        ENODEV,
        ENOENT,
        ToErrno as _,
    },
    handle,
    path,
    reent::Reent,
};
use crate::{
    device::{
        Device,
        DeviceError,
        DeviceId,
        MAX_DEVICES,
        SeekFrom,
    },
    registry,
    table,
};

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
/// Every slot shares it: the shims recover what they were called for from their arguments, so they
/// need no per-slot identity of their own.
///
/// Every operation a [`Device`], [`crate::device::File`] or [`crate::device::Dir`] can implement is
/// listed here, because the table is shared: leaving one out would deny it to every Rust device at
/// once, and an operation an individual device does not offer already reports that for itself.
static SHIM_TABLE: DevOpTab = DevOpTab {
    open_r: Some(shim_open),
    close_r: Some(shim_close),
    write_r: Some(shim_write),
    read_r: Some(shim_read),
    seek_r: Some(shim_seek),
    fstat_r: Some(shim_fstat),
    stat_r: Some(shim_stat),
    unlink_r: Some(shim_unlink),
    chdir_r: Some(shim_chdir),
    rename_r: Some(shim_rename),
    mkdir_r: Some(shim_mkdir),
    dir_state_size: dir_state::SIZE,
    diropen_r: Some(shim_diropen),
    dirreset_r: Some(shim_dirreset),
    dirnext_r: Some(shim_dirnext),
    dirclose_r: Some(shim_dirclose),
    statvfs_r: Some(shim_statvfs),
    ftruncate_r: Some(shim_ftruncate),
    fsync_r: Some(shim_fsync),
    rmdir_r: Some(shim_rmdir),
    // Horizon has no symbolic links, so there is nothing for `lstat` to do differently.
    lstat_r: Some(shim_stat),
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
/// Reports the device's name so paths resolve, and leaves every operation unsupported: a C device's
/// work goes through its own table, which its descriptors keep pointing at, and its per-descriptor
/// state is not something the Rust interface can produce.
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

/// Opens a path on behalf of a descriptor the C caller has already allocated.
///
/// The descriptor exists before this runs, so the file produced here is attached to it rather than
/// returned. A failure to attach means the descriptor went away underneath, which the C caller
/// reports as a bad descriptor.
unsafe extern "C" fn shim_open(
    r: *mut Reent,
    state: *mut c_void,
    path: *const c_char,
    flags: c_int,
    _mode: c_int,
) -> c_int {
    let Some(fd) = handle::fd_from_state(state) else {
        return errno::fail(r, EBADF);
    };
    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    let Some(full_path) = (unsafe { borrow_path(path) }) else {
        return errno::fail(r, EINVAL);
    };

    let Some(device) = table::device_of(fd).and_then(registry::get) else {
        return errno::fail(r, ENODEV);
    };

    let file = match device.open(
        path::strip_device_prefix(full_path),
        decode_open_flags(flags),
    ) {
        Ok(file) => file,
        Err(err) => return errno::fail(r, err.to_errno()),
    };

    match table::attach(fd, file) {
        Ok(()) => 0,
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Forwards a C close to the Rust device behind the descriptor.
///
/// The descriptor was already released by `_close_r`, which is what calls this, so there is nothing
/// left to do here beyond reporting.
unsafe extern "C" fn shim_close(_r: *mut Reent, _state: *mut c_void) -> c_int {
    0
}

/// Forwards a C write to the Rust device behind the descriptor.
unsafe extern "C" fn shim_write(
    r: *mut Reent,
    state: *mut c_void,
    buf: *const c_char,
    len: usize,
) -> SsizeT {
    let Some(fd) = handle::fd_from_state(state) else {
        return errno::fail_ssize(r, EBADF);
    };
    if buf.is_null() {
        return errno::fail_ssize(r, EINVAL);
    }

    // SAFETY: C guarantees `buf` addresses `len` readable bytes for the duration of the call.
    let bytes = unsafe { core::slice::from_raw_parts(buf.cast::<u8>(), len) };

    match table::write(fd, bytes) {
        Ok(written) => written as SsizeT,
        Err(err) => errno::fail_ssize(r, err.to_errno()),
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
        return errno::fail_ssize(r, EBADF);
    };
    if buf.is_null() {
        return errno::fail_ssize(r, EINVAL);
    }

    // SAFETY: C guarantees `buf` addresses `len` writable bytes for the duration of the call.
    let bytes = unsafe { core::slice::from_raw_parts_mut(buf.cast::<u8>(), len) };

    match table::read(fd, bytes) {
        Ok(read) => read as SsizeT,
        Err(err) => errno::fail_ssize(r, err.to_errno()),
    }
}

/// Moves the position of the file behind the descriptor.
unsafe extern "C" fn shim_seek(
    r: *mut Reent,
    state: *mut c_void,
    offset: OffT,
    whence: c_int,
) -> OffT {
    /// Measure from the start of the file.
    const SEEK_SET: c_int = 0;
    /// Measure from the current position.
    const SEEK_CUR: c_int = 1;
    /// Measure from the end of the file.
    const SEEK_END: c_int = 2;

    // `fail` reports C's integer failure value, which is -1 and so fits every width it is widened
    // to here. The seek entry point returns `off_t` rather than `int`, which is the only reason a
    // cast is involved at all.
    let failed = |errno| OffT::from(errno::fail(r, errno));

    let Some(fd) = handle::fd_from_state(state) else {
        return failed(EBADF);
    };

    let pos = match whence {
        SEEK_SET if offset < 0 => return failed(EINVAL),
        // Guarded by the arm above, so the offset is non-negative and the cast is exact.
        SEEK_SET => SeekFrom::Start(offset as u64),
        SEEK_CUR => SeekFrom::Current(offset),
        SEEK_END => SeekFrom::End(offset),
        _ => return failed(EINVAL),
    };

    match table::seek(fd, pos) {
        // A lossy cast the C signature forces: `off_t` is signed and 64 bits wide, so a position
        // beyond 2^63 has nowhere to go. No Horizon filesystem produces one.
        Ok(position) => position as OffT,
        Err(err) => failed(err.to_errno()),
    }
}

/// Reports on the file behind the descriptor.
unsafe extern "C" fn shim_fstat(r: *mut Reent, state: *mut c_void, out: *mut Stat) -> c_int {
    let Some(fd) = handle::fd_from_state(state) else {
        return errno::fail(r, EBADF);
    };
    if out.is_null() {
        return errno::fail(r, EINVAL);
    }

    match table::metadata(fd) {
        // SAFETY: `out` is non-null and C guarantees it addresses a writable `struct stat`.
        Ok(metadata) => unsafe {
            out.write(metadata.into());
            0
        },
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Resizes the file behind the descriptor.
unsafe extern "C" fn shim_ftruncate(r: *mut Reent, state: *mut c_void, len: OffT) -> c_int {
    let Some(fd) = handle::fd_from_state(state) else {
        return errno::fail(r, EBADF);
    };
    let Ok(len) = u64::try_from(len) else {
        return errno::fail(r, EINVAL);
    };

    match table::set_len(fd, len) {
        Ok(()) => 0,
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Commits what has been written to the file behind the descriptor.
unsafe extern "C" fn shim_fsync(r: *mut Reent, state: *mut c_void) -> c_int {
    let Some(fd) = handle::fd_from_state(state) else {
        return errno::fail(r, EBADF);
    };

    match table::sync(fd) {
        Ok(()) => 0,
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Reports on the entry a path names.
///
/// Also serves `lstat`: Horizon has no symbolic links, so there is no distinction to draw.
unsafe extern "C" fn shim_stat(r: *mut Reent, path: *const c_char, out: *mut Stat) -> c_int {
    if out.is_null() {
        return errno::fail(r, EINVAL);
    }

    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    let result = unsafe { with_device(path, |device, path| device.metadata(path)) };

    match result {
        Ok(metadata) => {
            // SAFETY: `out` is non-null and C guarantees it addresses a writable `struct stat`.
            unsafe { out.write(metadata.into()) };
            0
        }
        Err(errno) => errno::fail(r, errno),
    }
}

/// Removes the file a path names.
unsafe extern "C" fn shim_unlink(r: *mut Reent, path: *const c_char) -> c_int {
    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    report(r, unsafe {
        with_device(path, |device, path| device.remove_file(path))
    })
}

/// Makes a path the working directory of the device that serves it.
unsafe extern "C" fn shim_chdir(r: *mut Reent, path: *const c_char) -> c_int {
    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    report(r, unsafe {
        with_device(path, |device, path| device.set_current_dir(path))
    })
}

/// Creates a directory at a path.
unsafe extern "C" fn shim_mkdir(r: *mut Reent, path: *const c_char, _mode: c_int) -> c_int {
    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    report(r, unsafe {
        with_device(path, |device, path| device.create_dir(path))
    })
}

/// Removes the directory a path names.
unsafe extern "C" fn shim_rmdir(r: *mut Reent, path: *const c_char) -> c_int {
    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    report(r, unsafe {
        with_device(path, |device, path| device.remove_dir(path))
    })
}

/// Moves an entry from one path to another.
///
/// Both paths must name the same device: the C standard library refuses a rename that crosses
/// devices, and this refuses it again rather than trusting that.
unsafe extern "C" fn shim_rename(r: *mut Reent, from: *const c_char, to: *const c_char) -> c_int {
    // SAFETY: C guarantees both are live nul-terminated strings for the duration of the call.
    let (Some(from), Some(to)) = (unsafe { borrow_path(from) }, unsafe { borrow_path(to) }) else {
        return errno::fail(r, EINVAL);
    };

    let (Some(from_device), Some(to_device)) =
        (path::device_for_path(from), path::device_for_path(to))
    else {
        return errno::fail(r, ENODEV);
    };
    if from_device != to_device {
        return errno::fail(r, EINVAL);
    }

    let Some(device) = registry::get(from_device) else {
        return errno::fail(r, ENODEV);
    };

    match device.rename(
        path::strip_device_prefix(from),
        path::strip_device_prefix(to),
    ) {
        Ok(()) => 0,
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Reports how much space the filesystem holding a path has.
unsafe extern "C" fn shim_statvfs(r: *mut Reent, path: *const c_char, out: *mut StatVfs) -> c_int {
    if out.is_null() {
        return errno::fail(r, EINVAL);
    }

    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    let result = unsafe { with_device(path, |device, path| device.space_info(path)) };

    match result {
        Ok(info) => {
            // SAFETY: `out` is non-null and C guarantees it addresses a writable `struct statvfs`.
            unsafe { out.write(info.into()) };
            0
        }
        Err(errno) => errno::fail(r, errno),
    }
}

/// Opens a directory, storing the walk in the state the C caller allocated behind the iterator.
///
/// Returns the iterator it was given on success and null on failure, which is the contract
/// `__diropen` expects: on null it frees the iterator without calling anything else.
unsafe extern "C" fn shim_diropen(
    r: *mut Reent,
    iter: *mut DirIter,
    path: *const c_char,
) -> *mut DirIter {
    if iter.is_null() {
        errno::fail(r, EINVAL);
        return core::ptr::null_mut();
    }

    // SAFETY: C guarantees `path` is a live nul-terminated string for the duration of the call.
    let result = unsafe { with_device(path, |device, path| device.open_dir(path)) };

    let dir = match result {
        Ok(dir) => dir,
        Err(errno) => {
            errno::fail(r, errno);
            return core::ptr::null_mut();
        }
    };

    // SAFETY: `iter` is non-null and C allocated `SIZE` bytes of state behind it, which is what
    // `dir_state_size` in the shim table asked for.
    unsafe { dir_state::store(iter, dir) };
    iter
}

/// Restarts a directory walk from its first entry.
unsafe extern "C" fn shim_dirreset(r: *mut Reent, iter: *mut DirIter) -> c_int {
    // SAFETY: `iter` was produced by `shim_diropen`, so its state holds a live walk.
    let Some(dir) = (unsafe { dir_state::borrow(iter) }) else {
        return errno::fail(r, EBADF);
    };

    match dir.reset() {
        Ok(()) => 0,
        Err(err) => errno::fail(r, err.to_errno()),
    }
}

/// Produces the next entry of a directory walk.
///
/// The end of the directory is reported the way `readdir` expects to hear it: a failure whose error
/// number is `ENOENT`, which it translates back into a clean end rather than an error.
unsafe extern "C" fn shim_dirnext(
    r: *mut Reent,
    iter: *mut DirIter,
    name_out: *mut c_char,
    stat_out: *mut Stat,
) -> c_int {
    // SAFETY: `iter` was produced by `shim_diropen`, so its state holds a live walk.
    let Some(dir) = (unsafe { dir_state::borrow(iter) }) else {
        return errno::fail(r, EBADF);
    };
    if name_out.is_null() {
        return errno::fail(r, EINVAL);
    }

    let entry = match dir.next() {
        Ok(Some(entry)) => entry,
        Ok(None) => return errno::fail(r, ENOENT),
        Err(err) => return errno::fail(r, err.to_errno()),
    };

    let name = entry.name.as_bytes();
    // SAFETY: C provides a buffer of `NAME_MAX + 1` bytes, and an `EntryName` is at most `NAME_MAX`
    // long, so the name and its terminator fit.
    unsafe {
        core::ptr::copy_nonoverlapping(name.as_ptr().cast::<c_char>(), name_out, name.len());
        name_out.add(name.len()).write(0);
    }

    // `seekdir` walks a directory for position alone and passes no place to put the metadata.
    if !stat_out.is_null() {
        // SAFETY: `stat_out` is non-null and C guarantees it addresses a writable `struct stat`.
        unsafe { stat_out.write(entry.metadata.into()) };
    }

    0
}

/// Ends a directory walk, releasing what it held.
unsafe extern "C" fn shim_dirclose(r: *mut Reent, iter: *mut DirIter) -> c_int {
    // SAFETY: `iter` was produced by `shim_diropen` and is closed exactly once, so the walk stored
    // behind it is live and unreachable afterwards.
    let Some(dir) = (unsafe { dir_state::take(iter) }) else {
        return errno::fail(r, EBADF);
    };

    drop(dir);
    0
}

/// Runs `operation` against the device a path resolves to.
///
/// Every per-path shim starts the same way, because the path is the only thing it is handed: parse
/// it, resolve the device, strip the device name, and hand the remainder over. The failures before
/// `operation` runs are the boundary's own, so they arrive as error numbers rather than as a
/// [`crate::device::DeviceError`] that no device produced.
///
/// # Safety
///
/// `path` must be null or point to a live nul-terminated string.
unsafe fn with_device<T>(
    path: *const c_char,
    operation: impl FnOnce(&'static dyn Device, &CStr) -> Result<T, DeviceError>,
) -> Result<T, c_int> {
    // SAFETY: the caller guarantees `path` is null or a live nul-terminated string.
    let Some(full_path) = (unsafe { borrow_path(path) }) else {
        return Err(EINVAL);
    };

    let device = path::device_for_path(full_path)
        .and_then(registry::get)
        .ok_or(ENODEV)?;

    operation(device, path::strip_device_prefix(full_path)).map_err(|err| err.to_errno())
}

/// Reports the outcome of a per-path shim the way C expects it.
fn report(r: *mut Reent, result: Result<(), c_int>) -> c_int {
    match result {
        Ok(()) => 0,
        Err(errno) => errno::fail(r, errno),
    }
}

/// Borrows a path handed in by C, refusing a null pointer.
///
/// # Safety
///
/// `path` must be null or point to a live nul-terminated string that outlives the returned borrow.
unsafe fn borrow_path<'a>(path: *const c_char) -> Option<&'a CStr> {
    if path.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees `path` is a live nul-terminated string.
    Some(unsafe { CStr::from_ptr(path) })
}
