//! Memory-object (nvmap) FFI.
//!
//! The C surface keeps the object's fields in a caller-owned struct and names
//! the device through a process-wide descriptor, so both live here rather than
//! in `nx-nv`: the descriptor has to sit beside the driver session it borrows,
//! and the runtime is what owns that session's lifetime.
//!
//! Every entry point below therefore does the same two things — take the
//! driver session for the length of one call, and borrow the device over it —
//! before handing the actual work to `nx-nv`.

use core::{
    ffi::c_void,
    ptr::NonNull,
};

use nx_nv::{
    BorrowedMapDevice,
    MapAlign,
    MapBuffer,
    MapHandle,
    MapId,
    MapKind,
    MemoryMap,
    NvMapDevice,
};
use nx_service_nv::fd::Fd;
use nx_std_sync::rwlock::RwLock;

use crate::ffi::common::GENERIC_ERROR;

/// The sentinel the C surface uses for "no handle".
const NO_HANDLE: u32 = u32::MAX;

/// The process-wide memory-object device.
///
/// The descriptor is stored rather than the open device, because the device
/// borrows the driver session and the session is only reachable through a lock
/// guard that cannot outlive a call. Each entry point rebuilds the borrowed
/// view over the descriptor for as long as it holds the guard.
static DEVICE: RwLock<Option<DeviceState>> = RwLock::new(None);

/// The descriptor and the count of callers that asked for it.
struct DeviceState {
    fd: Fd,
    ref_count: u32,
}

/// The caller-owned memory-object record.
///
/// The C header reads these fields directly through inline accessors, so the
/// layout is the contract: a field that moves is not a compile error on either
/// side, it is a caller reading the wrong four bytes. The assertions below
/// pin it.
#[repr(C)]
pub struct NvMap {
    handle: u32,
    id: u32,
    size: u32,
    cpu_addr: *mut c_void,
    kind: u32,
    has_init: bool,
    is_cpu_cacheable: bool,
}

const _: () = {
    assert!(size_of::<NvMap>() == 32);
    assert!(align_of::<NvMap>() == 8);
    assert!(core::mem::offset_of!(NvMap, handle) == 0);
    assert!(core::mem::offset_of!(NvMap, id) == 4);
    assert!(core::mem::offset_of!(NvMap, size) == 8);
    assert!(core::mem::offset_of!(NvMap, cpu_addr) == 16);
    assert!(core::mem::offset_of!(NvMap, kind) == 24);
    assert!(core::mem::offset_of!(NvMap, has_init) == 28);
    assert!(core::mem::offset_of!(NvMap, is_cpu_cacheable) == 29);
};

/// Opens the memory-object device, or counts another caller of an open one.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_nvmap_init() -> u32 {
    // The driver session is taken before the device slot, and every entry
    // point below does the same. Taking them the other way round here would
    // put two threads in a position to hold one lock each and wait for the
    // other's.
    let Some(service) = crate::services::nv::get_service() else {
        return GENERIC_ERROR;
    };

    let mut guard = DEVICE.write();
    if let Some(state) = guard.as_mut() {
        state.ref_count += 1;
        return 0;
    }

    match NvMapDevice::open(&service) {
        Ok(device) => {
            // The descriptor outlives this call, so the close obligation is
            // handed to the slot below: `nvmap_exit` is what discharges it.
            let fd = device.into_fd();
            *guard = Some(DeviceState { fd, ref_count: 1 });
            0
        }
        Err(_) => GENERIC_ERROR,
    }
}

/// Drops one caller's claim, closing the device when the last one goes.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_nvmap_exit() {
    // Taken before the device slot, in the same order as every entry point
    // above and below. A session that is already gone closed every descriptor
    // on it, so the slot is still cleared in that case.
    let service = crate::services::nv::get_service();

    let mut guard = DEVICE.write();
    let Some(state) = guard.as_mut() else {
        return;
    };

    state.ref_count = state.ref_count.saturating_sub(1);
    if state.ref_count > 0 {
        return;
    }

    let fd = state.fd;
    *guard = None;

    let Some(service) = service else {
        return;
    };
    // Rebuilding the owner is how the descriptor gets closed: its destructor
    // is the only thing that sends the close.
    // SAFETY: `fd` was opened by `nvmap_init` and taken out of the slot above,
    // so nothing else will close it.
    let _ = NvMapDevice::from_raw_unchecked(&service, fd);
}

/// Returns the descriptor the device is open on.
#[unsafe(no_mangle)]
pub extern "C" fn __nx_rt_nro__libnx_nvmap_get_fd() -> u32 {
    match DEVICE.read().as_ref() {
        Some(state) => state.fd.to_raw(),
        None => NO_HANDLE,
    }
}

/// Allocates a memory object over a caller-provided buffer.
///
/// # Safety
///
/// `m` must point to a writable [`NvMap`], and `cpu_addr` to `size` bytes this
/// process owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_nvmap_create(
    m: *mut NvMap,
    cpu_addr: *mut c_void,
    size: u32,
    align: u32,
    kind: u32,
    is_cpu_cacheable: bool,
) -> u32 {
    let Some(record) = (unsafe { m.as_mut() }) else {
        return GENERIC_ERROR;
    };
    let Some(ptr) = NonNull::new(cpu_addr.cast::<u8>()) else {
        return GENERIC_ERROR;
    };
    let Ok(buffer) = MapBuffer::create(ptr, size as usize) else {
        return GENERIC_ERROR;
    };
    // A caller that asks for less than a page gets the page the driver would
    // have rounded up to anyway, which is what the C surface has always done.
    let align = MapAlign::try_from(align.max(MapAlign::PAGE.to_raw())).unwrap_or(MapAlign::PAGE);
    // The kind is a byte on the wire and arrives here in a C `u32`. Narrowing
    // it silently would fold 0x100 onto 0x00, which is a different, valid kind.
    let Ok(kind) = u8::try_from(kind) else {
        return GENERIC_ERROR;
    };
    let kind = MapKind::from_raw(kind);

    let Some(service) = crate::services::nv::get_service() else {
        return GENERIC_ERROR;
    };
    let Some(device) = borrow_device(&service) else {
        return GENERIC_ERROR;
    };

    match device.create_map(buffer, align, kind, is_cpu_cacheable) {
        Ok(map) => {
            let id = map.id();
            *record = NvMap {
                handle: map.into_handle().to_raw(),
                id: id.to_raw(),
                size,
                cpu_addr,
                kind: kind.to_raw() as u32,
                has_init: true,
                is_cpu_cacheable,
            };
            0
        }
        Err(_) => GENERIC_ERROR,
    }
}

/// Adopts a memory object another process allocated.
///
/// # Safety
///
/// `m` must point to a writable [`NvMap`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_nvmap_load_remote(m: *mut NvMap, id: u32) -> u32 {
    let Some(record) = (unsafe { m.as_mut() }) else {
        return GENERIC_ERROR;
    };

    let Some(service) = crate::services::nv::get_service() else {
        return GENERIC_ERROR;
    };
    let Some(device) = borrow_device(&service) else {
        return GENERIC_ERROR;
    };

    let map = match device.adopt_map(MapId::from_raw(id)) {
        Ok(map) => map,
        Err(_) => return GENERIC_ERROR,
    };

    let (Ok(size), Ok(kind)) = (map.size(), map.kind()) else {
        return GENERIC_ERROR;
    };

    *record = NvMap {
        handle: map.into_handle().to_raw(),
        id,
        size,
        cpu_addr: core::ptr::null_mut(),
        kind: kind.to_raw() as u32,
        has_init: true,
        is_cpu_cacheable: false,
    };
    0
}

/// Releases this process's reference to a memory object.
///
/// # Safety
///
/// `m` must point to a writable [`NvMap`] that a create or adopt call filled
/// in and that has not already been closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_nvmap_close(m: *mut NvMap) {
    let Some(record) = (unsafe { m.as_mut() }) else {
        return;
    };
    if !record.has_init {
        return;
    }

    if record.handle != NO_HANDLE
        && let Some(service) = crate::services::nv::get_service()
        && let Some(device) = borrow_device(&service)
    {
        let buffer = NonNull::new(record.cpu_addr.cast::<u8>())
            .and_then(|ptr| MapBuffer::create(ptr, record.size as usize).ok());

        // Rebuilding the owner is what sends the release and restores the
        // buffer's mapping: its destructor owns both.
        // SAFETY: the record says this reference is still held, and it is
        // cleared below so nothing rebuilds a second owner over it.
        let _ = MemoryMap::from_raw_unchecked(
            device,
            MapHandle::from_raw_unchecked(record.handle),
            MapId::from_raw(record.id),
            buffer,
            record.is_cpu_cacheable,
        );
    }

    record.handle = NO_HANDLE;
    record.cpu_addr = core::ptr::null_mut();
    record.has_init = false;
}

/// Borrows the open device over a driver session held for this call.
fn borrow_device<'s>(service: &'s nx_service_nv::NvService) -> Option<BorrowedMapDevice<'s>> {
    let fd = DEVICE.read().as_ref().map(|state| state.fd)?;
    // SAFETY: the descriptor came out of the slot `nvmap_init` filled and
    // `nvmap_exit` clears, so it is open for the length of this call.
    Some(BorrowedMapDevice::from_raw_unchecked(service, fd))
}
