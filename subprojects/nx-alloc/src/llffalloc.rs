//! # Linked list First Fit Allocator
//!
//! This module provides a linked list first fit allocator.
//! It is used to allocate memory for the entire program.
//!
//! It is based on the [linked_list_allocator](https://github.com/rust-osdev/linked_list_allocator) crate.
use core::{
    alloc::Layout,
    ffi::{
        c_char,
        c_void,
    },
    ptr::{
        self,
        NonNull,
    },
};

use nx_svc::{
    mem::set_heap_size,
    misc::{
        get_total_memory_size,
        get_used_memory_size,
    },
};

/// A wrapper around the linked list allocator that provides
/// a lazy initialization mechanism for the heap.
pub struct Heap(Option<linked_list_allocator::Heap>);

impl Heap {
    /// Create a new allocator with an uninitialized heap.
    pub const fn new_uninit() -> Self {
        Self(None)
    }

    /// Returns `true` if the heap has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.0.is_some()
    }

    /// Initialize the heap using SVC memory allocation.
    pub fn init(&mut self) {
        self.0 = Some(init_inner_heap());
    }

    /// Initialize the heap with a pre-allocated memory region.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `addr` points to a valid, owned memory region of at least `size` bytes
    /// - The memory region will remain valid for the lifetime of the allocator
    pub unsafe fn init_with_heap_override(&mut self, addr: NonNull<c_void>, size: usize) {
        self.0 = Some(unsafe { linked_list_allocator::Heap::new(addr.as_ptr() as *mut u8, size) });
    }

    /// Allocate memory from the heap.
    ///
    /// Returns a null pointer if the heap has no block that fits `layout`.
    ///
    /// Taking a [`Layout`] rather than a loose size and alignment is what makes this safe: the
    /// validity the previous signature asked callers to guarantee is the invariant `Layout`
    /// already carries, so there is nothing left to re-check and nothing left to get wrong.
    ///
    /// # Panics
    ///
    /// Panics if the heap is not yet initialized and the kernel refuses to give this process
    /// one, which leaves no memory for the caller to have proceeded with.
    pub fn malloc(&mut self, layout: Layout) -> *mut u8 {
        let heap = self.0.get_or_insert_with(init_inner_heap);
        match heap.allocate_first_fit(layout) {
            Ok(nn) => nn.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }

    /// Free memory to the heap.
    ///
    /// A null `ptr` is a no-op.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `ptr` was previously allocated by this allocator with this exact `layout`
    /// - `ptr` has not been freed already
    /// - The memory region is no longer in use
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`malloc`](Self::malloc), which can only arise if a
    /// pointer is freed before anything was ever allocated.
    pub unsafe fn free(&mut self, ptr: *mut u8, layout: Layout) {
        let Some(ptr) = ptr::NonNull::new(ptr) else {
            return;
        };

        let heap = self.0.get_or_insert_with(init_inner_heap);
        unsafe { heap.deallocate(ptr, layout) };
    }
}

/// Initialize the heap via SVC memory allocation.
///
/// This function allocates heap memory using the kernel's SetHeapSize SVC.
/// It is either called by the `init` function or when the heap is first used.
fn init_inner_heap() -> linked_list_allocator::Heap {
    // Default heap size if not specified (0x2000000 * 16)
    const DEFAULT_HEAP_SIZE: usize = 0x2_000_000 * 16;
    const HEAP_SIZE_ALIGN: usize = 0x200_000;

    // Try to get total and used memory to determine heap size.
    // A failed query is treated as "nothing known", which drives the calculation below to
    // `DEFAULT_HEAP_SIZE`; there is no smaller heap worth attempting on the strength of a
    // reading the kernel would not give us.
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
    unsafe { linked_list_allocator::Heap::new(heap_bottom, heap_size) }
}
