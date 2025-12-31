//! # Linked list First Fit Allocator
//!
//! This module provides a linked list first fit allocator.
//! It is used to allocate memory for the entire program.
//!
//! It is based on the [linked_list_allocator](https://github.com/rust-osdev/linked_list_allocator) crate.
use core::{alloc::Layout, ffi::c_char, ptr};

use nx_rt_env as env;
use nx_svc::{
    mem::set_heap_size,
    misc::{get_total_memory_size, get_used_memory_size},
};

/// A wrapper around the linked list allocator that provides
/// a lazy initialization mechanism for the heap.
pub struct Heap(Option<linked_list_allocator::Heap>);

impl Heap {
    /// Create a new allocator with an uninitialized heap.
    pub const fn new_uninit() -> Self {
        Self(None)
    }

    /// Initialize the heap.
    pub fn init(&mut self) {
        self.0 = Some(init_inner_heap());
    }

    /// Allocate memory from the heap.
    pub unsafe fn malloc(&mut self, size: usize, align: usize) -> *mut u8 {
        // Check if the layout is valid
        let Ok(layout) = Layout::from_size_align(size, align) else {
            return ptr::null_mut();
        };

        let heap = self.0.get_or_insert_with(init_inner_heap);
        match heap.allocate_first_fit(layout) {
            Ok(nn) => nn.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }

    /// Free memory to the heap.
    pub unsafe fn free(&mut self, ptr: *mut u8, size: usize, align: usize) {
        let Some(ptr) = ptr::NonNull::new(ptr) else {
            return;
        };

        let heap = self.0.get_or_insert_with(init_inner_heap);
        let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
        unsafe { heap.deallocate(ptr, layout) };
    }
}

/// Initialize the linked-list allocator heap
///
/// This function is used to initialize the linked-list allocator heap.
/// It is either called by the `init` function or when the heap is first used.
fn init_inner_heap() -> linked_list_allocator::Heap {
    // Check if homebrew loader provided a heap override
    if let Some((heap_addr, heap_size)) = env::heap_override() {
        // SAFETY: The homebrew loader guarantees this region is valid and owned by us.
        return unsafe {
            linked_list_allocator::Heap::new(heap_addr.as_ptr() as *mut u8, heap_size)
        };
    }

    // No heap override - allocate via SVC
    // Default heap size if not specified (0x2000000 * 16)
    const DEFAULT_HEAP_SIZE: usize = 0x2_000_000 * 16;
    const HEAP_SIZE_ALIGN: usize = 0x200_000;

    // Try to get total and used memory to determine heap size
    let mem_available = get_total_memory_size().unwrap_or(0);
    let mem_used = get_used_memory_size().unwrap_or(0);

    // Calculate heap size
    let mut heap_size = 0;
    if mem_available > mem_used + HEAP_SIZE_ALIGN {
        heap_size = (mem_available - mem_used - HEAP_SIZE_ALIGN) & !(HEAP_SIZE_ALIGN - 1);
    }
    if heap_size == 0 {
        heap_size = DEFAULT_HEAP_SIZE;
    }

    // Actually allocate the heap
    let heap_bottom = match set_heap_size(heap_size) {
        Ok(heap_addr) => heap_addr as *mut c_char,
        Err(_) => {
            panic!("Failed to allocate heap memory: HEAP_ALLOCATION_FAILED");
        }
    };

    // SAFETY: The kernel guarantees this region is valid and owned by us.
    unsafe { linked_list_allocator::Heap::new(heap_bottom as *mut u8, heap_size) }
}
