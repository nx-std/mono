use core::{alloc::Layout, ffi::c_char, ptr};

use linked_list_allocator::Heap;
use nx_svc::{
    debug::{BreakReason, break_event},
    mem::set_heap_size,
    misc::{get_total_memory_size, get_used_memory_size},
};

use crate::sync::Mutex;

/// The allocator instance.
pub static ALLOC: Mutex<LinkedListAllocator> = Mutex::new(LinkedListAllocator::new_uninit());

/// A wrapper around the linked list allocator that provides
/// a lazy initialization mechanism for the heap.
pub struct LinkedListAllocator(Option<Heap>);

impl LinkedListAllocator {
    /// Create a new allocator with an uninitialized heap.
    const fn new_uninit() -> Self {
        Self(None)
    }

    /// Allocate memory from the heap.
    pub unsafe fn malloc(&mut self, size: usize, align: usize) -> *mut u8 {
        // Check if the layout is valid
        let Ok(layout) = Layout::from_size_align(size, align) else {
            return ptr::null_mut();
        };

        let heap = self.0.get_or_insert_with(init_alloc_heap);
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

        let heap = self.0.get_or_insert_with(init_alloc_heap);
        let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
        unsafe { heap.deallocate(ptr, layout) };
    }
}

fn init_alloc_heap() -> Heap {
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
    let heap_addr = match set_heap_size(heap_size) {
        Ok(heap_addr) => heap_addr as *mut c_char,
        Err(_) => {
            break_event(BreakReason::Panic, 0, 0);
        }
    };

    // Safety: The kernel guarantees this region is valid and owned by us
    unsafe { Heap::new(heap_addr as *mut u8, heap_size) }
}
