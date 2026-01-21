//! C FFI bindings for compatibility with existing C code
//!
//! This module provides `#[no_mangle]` C functions that follow the nx-thread
//! naming convention for internal virtmem operations.

use core::{ffi::c_void, ptr};

use nx_std_sync::mutex::MutexGuard;

use super::sys;

/// FFI-compatible reservation type
pub type VirtmemReservation = sys::VirtmemReservation;

/// Locks the virtual memory manager mutex
///
/// # Safety
///
/// This function intentionally leaks the mutex guard to keep the mutex locked.
/// The caller must ensure `__nx_sys_mem__virtmem_unlock()` is called to release
/// the lock before the thread terminates.
///
/// See: virtmem.h's `virtmemLock()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_mem__virtmem_lock() {
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
pub unsafe extern "C" fn __nx_sys_mem__virtmem_unlock() {
    unsafe { sys::VMM.force_unlock() };
}

/// Sets up the virtual memory manager state
///
/// # Safety
///
/// This must be called during early initialization before any concurrent access
/// to the virtual memory manager. The caller must ensure no other threads are
/// accessing the VMM during initialization.
///
/// This is called by the libnx runtime during early initialization.
/// It initializes internal state but does **not** keep the mutex locked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_mem__virtmem_setup() {
    // Acquire the mutex, initialize state if needed, then immediately release.
    sys::lock().init();
}

/// Finds a random slice of free general purpose address space
///
/// # Safety
///
/// The caller must hold the VMM lock (via `__nx_sys_mem__virtmem_lock()`) before
/// calling this function. Returns null if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_mem__virtmem_find_aslr(
    size: usize,
    guard_size: usize,
) -> *mut c_void {
    if !sys::VMM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    // SAFETY: current thread owns the lock.
    let vmm: &mut sys::VirtmemManager = unsafe { &mut *sys::VMM.data_ptr() };
    vmm.find_aslr(size, guard_size)
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Finds a random slice of free stack address space
///
/// # Safety
///
/// The caller must hold the VMM lock (via `__nx_sys_mem__virtmem_lock()`) before
/// calling this function. Returns null if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_mem__virtmem_find_stack(
    size: usize,
    guard_size: usize,
) -> *mut c_void {
    if !sys::VMM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    let vmm: &mut sys::VirtmemManager = unsafe { &mut *sys::VMM.data_ptr() };
    vmm.find_stack(size, guard_size)
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Finds a random slice of free code memory address space
///
/// # Safety
///
/// The caller must hold the VMM lock (via `__nx_sys_mem__virtmem_lock()`) before
/// calling this function. Returns null if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_mem__virtmem_find_code_memory(
    size: usize,
    guard_size: usize,
) -> *mut c_void {
    if !sys::VMM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    let vmm: &mut sys::VirtmemManager = unsafe { &mut *sys::VMM.data_ptr() };
    vmm.find_code_memory(size, guard_size)
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Reserves a range of memory address space
///
/// # Safety
///
/// The caller must hold the VMM lock (via `__nx_sys_mem__virtmem_lock()`) before
/// calling this function. Returns null if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_mem__virtmem_add_reservation(
    mem: *mut c_void,
    size: usize,
) -> *mut VirtmemReservation {
    if !sys::VMM.is_locked_by_current_thread() {
        return ptr::null_mut();
    }

    let vmm: &mut sys::VirtmemManager = unsafe { &mut *sys::VMM.data_ptr() };
    vmm.add_reservation(mem, size).unwrap_or_default()
}

/// Releases a memory address space reservation
///
/// # Safety
///
/// The caller must hold the VMM lock (via `__nx_sys_mem__virtmem_lock()`) before
/// calling this function. Does nothing if the lock is not held by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_mem__virtmem_remove_reservation(rv: *mut VirtmemReservation) {
    if !sys::VMM.is_locked_by_current_thread() {
        return;
    }

    let vmm: &mut sys::VirtmemManager = unsafe { &mut *sys::VMM.data_ptr() };
    vmm.remove_reservation(rv);
}
