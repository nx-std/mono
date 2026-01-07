//! # Global allocator
//!
//! This module provides a global allocator for the Nintendo Switch.
//! It is used to allocate memory for the entire program.

use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
    ptr::NonNull,
};

use crate::{
    llffalloc,
    sync::{Mutex, MutexGuard},
};

/// The global allocator instance.
#[cfg_attr(feature = "global-allocator", global_allocator)]
pub static ALLOC: NxAllocator = NxAllocator::new_uninit();

/// Initialize the linked-list allocator heap via SVC.
///
/// This function is idempotent - subsequent calls after initialization are no-ops.
pub fn init() {
    let mut alloc = ALLOC.0.lock();
    if alloc.is_initialized() {
        return;
    }
    alloc.init();
}

/// Initialize the linked-list allocator heap with a pre-allocated memory region.
///
/// This function is idempotent - subsequent calls after initialization are no-ops.
///
/// # Safety
///
/// The caller must ensure that:
/// - `addr` points to a valid, owned memory region of at least `size` bytes
/// - The memory region will remain valid for the lifetime of the allocator
pub unsafe fn init_with_heap_override(addr: NonNull<c_void>, size: usize) {
    let mut alloc = ALLOC.0.lock();
    if alloc.is_initialized() {
        return;
    }
    // SAFETY: Caller guarantees the memory region is valid and owned.
    unsafe { alloc.init_with_heap_override(addr, size) };
}

/// Lock the allocator and return a mutable reference to the heap.
pub fn lock() -> MutexGuard<'static, llffalloc::Heap> {
    ALLOC.0.lock()
}

/// A `#[global_allocator]` for the Nintendo Switch.
pub struct NxAllocator(Mutex<llffalloc::Heap>);

impl NxAllocator {
    /// Create a new uninitialized allocator.
    pub const fn new_uninit() -> Self {
        Self(Mutex::new(llffalloc::Heap::new_uninit()))
    }
}

unsafe impl GlobalAlloc for NxAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut alloc = self.0.lock();
        unsafe { alloc.malloc(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut alloc = self.0.lock();
        unsafe { alloc.free(ptr, layout.size(), layout.align()) }
    }
}
