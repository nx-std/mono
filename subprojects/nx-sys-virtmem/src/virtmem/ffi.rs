//! C FFI bindings for compatibility with existing C code
//!
//! This module provides `#[no_mangle]` C functions that export the
//! `__nx_sys_virtmem__virtmem_*` symbols backing the libnx `virtmem*` ABI.

use core::{cell::UnsafeCell, ffi::c_void, mem::size_of, ptr};

use nx_sys_sync::data::MutexGuard;

use super::{reservation::Reservation, sys};

/// Number of concurrently-live FFI reservation handles the descriptor pool can
/// hand out.
///
/// The C `virtmem*` ABI hands callers an opaque `VirtmemReservation*`; each
/// live handle occupies one pool slot. This bounds *concurrently outstanding
/// FFI handles* — not reservations overall, which the address-space-sized
/// bitmap tracks without a cap. Live reservations are only ever a handful (the
/// gap between a `find_*` call and the matching map operation), so 128 slots
/// are effectively uncapped; exhaustion falls back to the ABI's `NULL` return.
const RESERVATION_POOL_LEN: usize = 128;

/// Fixed descriptor pool backing the opaque `VirtmemReservation*` handles.
///
/// Heap-free `static` storage: the bitmap migration removed the heap-allocated
/// reservation node, and this pool replaces it as the source of the
/// pointer-shaped token the C ABI requires. It is touched only by the FFI
/// `add`/`remove` entry points, which run with the `VIRTMEM` mutex held, so the
/// `UnsafeCell` is never accessed concurrently.
static RESERVATION_POOL: ReservationPool = ReservationPool::new();

/// Opaque C handle for a virtual-address reservation.
///
/// `nx_virtmem.h` declares `VirtmemReservation` as an incomplete type: C code
/// only ever holds and round-trips a `VirtmemReservation*`, never dereferences
/// it. Each handle is a pointer to one [`RESERVATION_POOL`] slot; the slot
/// stores the page-aligned extent so `remove` can release the exact range.
pub struct VirtmemReservation {
    /// The reserved range, or `None` when the slot is free.
    extent: Option<Reservation>,
}

impl VirtmemReservation {
    /// A free slot — the initial state of every pool entry.
    const FREE: Self = Self { extent: None };
}

/// Locks the virtual memory manager mutex
///
/// # Safety
///
/// This function intentionally leaks the mutex guard to keep the mutex locked.
/// The caller must ensure `__nx_sys_virtmem__virtmem_unlock()` is called to release
/// the lock before the thread terminates.
///
/// See: virtmem.h's `virtmemLock()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_lock() {
    // Acquire the lock and intentionally leak the guard so the mutex remains
    // held for subsequent FFI calls.
    let guard = sys::lock();
    let _ = MutexGuard::leak(guard);
}

/// Unlocks the virtual memory manager mutex
///
/// # Safety
///
/// The caller must ensure that the mutex is currently locked by the current
/// thread.
///
/// See: virtmem.h's `virtmemUnlock()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_unlock() {
    unsafe { sys::VIRTMEM.force_unlock() };
}

/// Sets up the virtual memory manager state
///
/// # Safety
///
/// This must be called during early initialization before any concurrent access
/// to the virtual memory manager. The caller must ensure no other threads are
/// accessing the virtmem manager during initialization.
///
/// This is called by the libnx runtime during early initialization.
/// It initializes internal state but does **not** keep the mutex locked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_setup() {
    // Acquire the mutex, initialize state if needed, then immediately release.
    sys::lock().init();
}

/// Finds a random slice of free general purpose address space
///
/// # Safety
///
/// The caller must hold the virtmem lock (via `__nx_sys_virtmem__virtmem_lock()`) before
/// calling this function. Returns null if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_find_aslr(
    size: usize,
    guard_size: usize,
) -> *mut c_void {
    if !sys::VIRTMEM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    // SAFETY: current thread owns the lock.
    let virtmem: &mut sys::VirtmemManager = unsafe { &mut *sys::VIRTMEM.data_ptr() };
    virtmem
        .find_aslr(size, guard_size)
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Finds a random slice of free stack address space
///
/// # Safety
///
/// The caller must hold the virtmem lock (via `__nx_sys_virtmem__virtmem_lock()`) before
/// calling this function. Returns null if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_find_stack(
    size: usize,
    guard_size: usize,
) -> *mut c_void {
    if !sys::VIRTMEM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    let virtmem: &mut sys::VirtmemManager = unsafe { &mut *sys::VIRTMEM.data_ptr() };
    virtmem
        .find_stack(size, guard_size)
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Finds a random slice of free code memory address space
///
/// # Safety
///
/// The caller must hold the virtmem lock (via `__nx_sys_virtmem__virtmem_lock()`) before
/// calling this function. Returns null if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_find_code_memory(
    size: usize,
    guard_size: usize,
) -> *mut c_void {
    if !sys::VIRTMEM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    let virtmem: &mut sys::VirtmemManager = unsafe { &mut *sys::VIRTMEM.data_ptr() };
    virtmem
        .find_code_memory(size, guard_size)
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Reserves a range of memory address space
///
/// Records the reservation in the heap-free bitmap and hands back an opaque
/// `VirtmemReservation*` from the descriptor pool. Returns null if the lock is
/// not held, if the range is invalid or already reserved, or if the descriptor
/// pool is exhausted.
///
/// # Safety
///
/// The caller must hold the virtmem lock (via `__nx_sys_virtmem__virtmem_lock()`) before
/// calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_add_reservation(
    mem: *mut c_void,
    size: usize,
) -> *mut VirtmemReservation {
    if !sys::VIRTMEM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    // SAFETY: current thread owns the lock.
    let virtmem: &mut sys::VirtmemManager = unsafe { &mut *sys::VIRTMEM.data_ptr() };
    let Some(range) = virtmem.add_reservation(mem, size) else {
        return ptr::null_mut();
    };

    // SAFETY: the virtmem lock is held (checked above), so the descriptor pool is
    // not accessed concurrently.
    let handle = unsafe { RESERVATION_POOL.claim(range) };
    if handle.is_null() {
        // Pool exhausted: undo the bitmap reservation so the address space is
        // not leaked, and report failure via the ABI's `NULL` return.
        virtmem.remove_reservation(range);
    }
    handle
}

/// Releases a memory address space reservation
///
/// Recovers the reserved extent from the opaque handle, clears it from the
/// bitmap, and frees the descriptor-pool slot. A null handle, a stale handle,
/// or a double-remove is a safe no-op.
///
/// # Safety
///
/// The caller must hold the virtmem lock (via `__nx_sys_virtmem__virtmem_lock()`) before
/// calling this function. `rv` must be null or a handle previously returned by
/// `__nx_sys_virtmem__virtmem_add_reservation()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_virtmem__virtmem_remove_reservation(rv: *mut VirtmemReservation) {
    if !sys::VIRTMEM.is_locked_by_current_thread() {
        return;
    }

    // SAFETY: the virtmem lock is held (checked above), so the descriptor pool is
    // not accessed concurrently; `rv` is null or a handle from `add`.
    let Some(range) = (unsafe { RESERVATION_POOL.release(rv) }) else {
        return;
    };

    // SAFETY: current thread owns the lock.
    let virtmem: &mut sys::VirtmemManager = unsafe { &mut *sys::VIRTMEM.data_ptr() };
    virtmem.remove_reservation(range);
}

/// Fixed-size descriptor pool for the opaque `VirtmemReservation*` handles.
struct ReservationPool(UnsafeCell<[VirtmemReservation; RESERVATION_POOL_LEN]>);

// SAFETY: every access goes through `claim`/`release`, which the FFI entry
// points call only after confirming the current thread holds the `VIRTMEM` mutex,
// so no two threads ever touch the cell concurrently.
unsafe impl Sync for ReservationPool {}

impl ReservationPool {
    /// Creates an all-free descriptor pool.
    const fn new() -> Self {
        Self(UnsafeCell::new(
            [VirtmemReservation::FREE; RESERVATION_POOL_LEN],
        ))
    }

    /// Stores `range` in a free slot and returns a pointer to it, or a null
    /// pointer when every slot is occupied.
    ///
    /// # Safety
    ///
    /// The caller must hold the `VIRTMEM` mutex so this is the only live access to
    /// the pool.
    unsafe fn claim(&self, range: Reservation) -> *mut VirtmemReservation {
        // SAFETY: the VIRTMEM-mutex precondition makes this the only live borrow
        // of the pool.
        let slots = unsafe { &mut *self.0.get() };
        for slot in slots.iter_mut() {
            if slot.extent.is_none() {
                slot.extent = Some(range);
                return slot as *mut VirtmemReservation;
            }
        }
        ptr::null_mut()
    }

    /// Releases the slot `handle` points at and returns the reservation it
    /// held, or `None` when `handle` is not a live pool slot (null, stale, or
    /// already freed).
    ///
    /// # Safety
    ///
    /// The caller must hold the `VIRTMEM` mutex so this is the only live access to
    /// the pool. `handle` must be null or a pointer previously returned by
    /// [`claim`](Self::claim).
    unsafe fn release(&self, handle: *mut VirtmemReservation) -> Option<Reservation> {
        if handle.is_null() {
            return None;
        }

        // SAFETY: the VIRTMEM-mutex precondition makes this the only live borrow
        // of the pool.
        let slots = unsafe { &mut *self.0.get() };

        // Map the handle back to a slot index, rejecting any pointer that is
        // not a properly-aligned slot inside the pool — the opaque handle is
        // trusted but cheaply bounds-checked.
        let base = slots.as_mut_ptr() as usize;
        let offset = (handle as usize).checked_sub(base)?;
        let stride = size_of::<VirtmemReservation>();
        if offset % stride != 0 {
            return None;
        }
        let index = offset / stride;
        if index >= RESERVATION_POOL_LEN {
            return None;
        }

        // A free slot here means a double-remove; `take` makes it a no-op.
        slots[index].extent.take()
    }
}
