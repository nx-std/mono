//! Heap configuration set by the runtime before allocator initialization.

use core::{
    ffi::c_void,
    ptr::NonNull,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

static HEAP_ADDR: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());
static HEAP_SIZE: AtomicUsize = AtomicUsize::new(0);

/// Set the heap override configuration.
///
/// Called by nx-rt::env during initialization before the allocator is used.
/// Uses `#[no_mangle]` to ensure single symbol when multiple staticlibs link nx-alloc.
///
/// # Safety
///
/// Must be called before any heap allocations occur.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_heap_override(addr: NonNull<c_void>, size: usize) {
    HEAP_ADDR.store(addr.as_ptr(), Ordering::Release);
    HEAP_SIZE.store(size, Ordering::Release);
}

/// Get the heap override configuration if set.
///
/// Returns `Some((addr, size))` if heap override was configured.
pub fn heap_override() -> Option<(NonNull<c_void>, usize)> {
    let addr = HEAP_ADDR.load(Ordering::Acquire);
    let size = HEAP_SIZE.load(Ordering::Acquire);

    NonNull::new(addr).map(|a| (a, size))
}
