//! # Global allocator
//!
//! This module provides a global allocator for the Nintendo Switch.
//! It is used to allocate memory for the entire program.

use core::alloc::{GlobalAlloc, Layout};

use crate::{
    llffalloc,
    sync::{Mutex, MutexGuard},
};

/// The `#[global_allocator]` for the Nintendo Switch.
#[cfg(feature = "global-allocator")]
#[global_allocator]
static GLOBAL_ALLOCATOR: NxAllocator = NxAllocator;

/// The allocator instance.
static ALLOC: Mutex<llffalloc::Heap> = Mutex::new(llffalloc::Heap::new_uninit());

/// Initialize the linked-list allocator heap
///
/// This function is used to initialize the linked-list allocator heap.
pub fn init() {
    let mut alloc = ALLOC.lock();
    alloc.init();
}

/// Lock the allocator and return a mutable reference to the heap.
pub fn lock<'a>() -> MutexGuard<'a, llffalloc::Heap> {
    ALLOC.lock()
}

/// A `#[global_allocator]` for the Nintendo Switch.
pub struct NxAllocator;

unsafe impl GlobalAlloc for NxAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut alloc = lock();
        unsafe { alloc.malloc(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut alloc = lock();
        unsafe { alloc.free(ptr, layout.size(), layout.align()) }
    }
}
