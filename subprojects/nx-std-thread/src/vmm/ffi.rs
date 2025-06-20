//! C FFI bindings for compatibility with existing C code
//!
//! This module provides `#[no_mangle]` C functions that follow the nx-thread
//! naming convention for internal virtmem operations.

use alloc::boxed::Box;
use core::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use nx_std_sync::mutex::MutexGuard;

use super::sys;

/// FFI-compatible reservation type
pub type VirtmemReservation = sys::VirtmemReservation;

type VmmGuard = MutexGuard<'static, sys::VirtmemManager>;

// Non-null pointer ⇒ mutex is currently held by this thread; null ⇒ free.
static HELD_GUARD: AtomicPtr<VmmGuard> = AtomicPtr::new(ptr::null_mut());

/// Locks the virtual memory manager mutex
///
/// See: virtmem.h's `virtmemLock()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_lock() {
    if !HELD_GUARD.load(Ordering::Acquire).is_null() {
        return;
    }

    let guard = sys::lock();

    // Leak the guard so the mutex stays locked across FFI calls.
    HELD_GUARD.store(Box::into_raw(Box::new(guard)), Ordering::Release);
}

/// Unlocks the virtual memory manager mutex  
///
/// See: virtmem.h's `virtmemUnlock()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_unlock() {
    // Atomically take ownership of the leaked guard, if any.
    let ptr = HELD_GUARD.swap(core::ptr::null_mut(), Ordering::AcqRel);
    if ptr.is_null() {
        return;
    }

    // Reconstruct the Box so it gets dropped, unlocking the mutex.
    let _boxed: Box<VmmGuard> = unsafe { Box::from_raw(ptr) };
}

/// Sets up the virtual memory manager state
///
/// This is called by the libnx runtime during early initialization.
/// It initializes internal state but does **not** keep the mutex locked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_setup() {
    // Acquire the mutex, initialize state if needed, then immediately release.
    sys::lock().init();
}

/// Finds a random slice of free general purpose address space
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_find_aslr(size: usize, guard_size: usize) -> *mut c_void {
    unsafe { with_lock(|vmm| vmm.find_aslr(size, guard_size)) }
        .flatten()
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Finds a random slice of free stack address space
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_find_stack(size: usize, guard_size: usize) -> *mut c_void {
    unsafe { with_lock(|vmm| vmm.find_stack(size, guard_size)) }
        .flatten()
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Finds a random slice of free code memory address space
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_find_code_memory(
    size: usize,
    guard_size: usize,
) -> *mut c_void {
    unsafe { with_lock(|vmm| vmm.find_code_memory(size, guard_size)) }
        .flatten()
        .map_or(ptr::null_mut(), |nn| nn.as_ptr())
}

/// Reserves a range of memory address space
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_add_reservation(
    mem: *mut c_void,
    size: usize,
) -> *mut VirtmemReservation {
    unsafe { with_lock(|vmm| vmm.add_reservation(mem, size)) }
        .flatten()
        .unwrap_or(ptr::null_mut())
}

/// Releases a memory address space reservation
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_virtmem_remove_reservation(rv: *mut VirtmemReservation) {
    let _ = unsafe {
        with_lock(|vmm| {
            vmm.remove_reservation(rv);
        })
    };
}

/// Execute a closure with a mutable reference to the global `VirtmemManager`
/// **only if** the caller currently holds the mutex through
/// `__nx_virtmem_lock()`.  When the lock is not held, the function
/// returns `None` and **does not** acquire the mutex by itself — matching the
/// original libnx contract that the caller must have locked beforehand.
#[inline]
unsafe fn with_lock<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut sys::VirtmemManager) -> R,
{
    let ptr = ptr::NonNull::new(HELD_GUARD.load(Ordering::Acquire))?;

    // SAFETY: We know the pointer is non-null and we're holding the lock.
    let guard_ref: &mut VmmGuard = unsafe { &mut *ptr.as_ptr() };
    Some(f(&mut *guard_ref))
}
