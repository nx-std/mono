//! # Global allocator
//!
//! This module provides a global allocator that uses the linked list allocator.
//! It is used to allocate memory for the entire program.
use core::alloc::{GlobalAlloc, Layout};

use crate::llalloc::ALLOC;

/// A global allocator that uses the linked list allocator.
pub struct GlobalLinkedListAllocator;

unsafe impl GlobalAlloc for GlobalLinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut alloc = ALLOC.lock();
        unsafe { alloc.malloc(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut alloc = ALLOC.lock();
        unsafe { alloc.free(ptr, layout.size(), layout.align()) }
    }
}
