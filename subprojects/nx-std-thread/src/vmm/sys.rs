//! Virtual memory management for Nintendo Switch
//!
//! This module provides C-compatible virtual memory management functions
//! that match the original libnx virtmem API.

extern crate alloc;

use alloc::boxed::Box;
use core::{ffi::c_void, ptr::NonNull};

use intrusive_collections::{LinkedList, LinkedListLink, intrusive_adapter};
use nx_rand::sys::next_u64;
use nx_std_sync::mutex::{Mutex, MutexGuard};
use nx_svc::{
    debug::{BreakReason, break_event},
    mem::{self, MemoryType, UnmapMemoryError},
};

/// Global virtual memory manager
static VMM: Mutex<VirtmemManager> = Mutex::new(VirtmemManager::new_uninit());

/// Lock the virtual memory manager
///
/// This function is equivalent to the C `virtmemLock()` function.
pub fn lock() -> MutexGuard<'static, VirtmemManager> {
    VMM.lock()
}

/// Virtual memory manager state
pub struct VirtmemManager(Option<VirtmemState>);

impl VirtmemManager {
    /// Create a new uninitialized virtual memory manager.
    ///
    /// If the virtual memory manager is not initialized, the initialization
    /// must be done by calling `init()` or it will be lazily initialized.
    const fn new_uninit() -> Self {
        Self(None)
    }

    /// Initialize the virtual memory manager
    ///
    /// This function is called when the virtual memory manager is first initialized.
    /// It queries the system for the memory regions and initializes the virtual memory
    /// manager state.
    ///
    /// If the virtual memory manager is already initialized, this function is a no-op.
    pub fn init(&mut self) {
        if self.0.is_some() {
            return;
        }
        let _ = self.0.insert(init_state());
    }

    /// Finds a random slice of free general purpose address space.
    ///
    /// This function searches the ASLR region for a suitable address range
    /// that can accommodate the requested size plus guard areas.
    ///
    /// # Arguments
    ///
    /// * `size` - Desired size of the slice (rounded up to page alignment)
    /// * `guard_size` - Desired size of unmapped guard areas (rounded up to page alignment)
    ///
    /// Returns a pointer to the slice of address space, or null on failure.
    ///
    /// This function is equivalent to the C `virtmemFindAslr()` function.
    pub fn find_aslr(&mut self, size: usize, guard_size: usize) -> Option<NonNull<c_void>> {
        let state = self.0.get_or_insert_with(init_state);
        state.find_random(RegionType::Aslr, size, guard_size)
    }

    /// Finds a random slice of free stack address space.
    ///
    /// This function searches the stack region for a suitable address range
    /// that can accommodate the requested size plus guard areas.
    ///
    /// # Arguments
    ///
    /// * `size` - Desired size of the slice (rounded up to page alignment)
    /// * `guard_size` - Desired size of unmapped guard areas (rounded up to page alignment)
    ///
    /// Returns a pointer to the slice of address space, or null on failure.
    ///
    /// This function is equivalent to the C `virtmemFindStack()` function.
    pub fn find_stack(&mut self, size: usize, guard_size: usize) -> Option<NonNull<c_void>> {
        let state = self.0.get_or_insert_with(init_state);
        state.find_random(RegionType::Stack, size, guard_size)
    }

    /// Finds a random slice of free code memory address space.
    ///
    /// This function searches the appropriate region for code memory allocation.
    /// On legacy kernels (1.0.0), code memory must be allocated in the stack region.
    /// On newer kernels, code memory can be allocated in the ASLR region.
    ///
    /// # Arguments
    ///
    /// * `size` - Desired size of the slice (rounded up to page alignment)
    /// * `guard_size` - Desired size of unmapped guard areas (rounded up to page alignment)
    ///
    /// Returns a pointer to the slice of address space, or null on failure.
    ///
    /// This function is equivalent to the C `virtmemFindCodeMemory()` function.
    pub fn find_code_memory(&mut self, size: usize, guard_size: usize) -> Option<NonNull<c_void>> {
        let state = self.0.get_or_insert_with(init_state);
        state.find_random(RegionType::CodeMemory, size, guard_size)
    }

    /// Reserves a range of memory address space.
    pub fn add_reservation(
        &mut self,
        mem: *mut c_void,
        size: usize,
    ) -> Option<*mut VirtmemReservation> {
        if mem.is_null() || size == 0 {
            return None;
        }

        let state = self.0.get_or_insert_with(init_state);

        // SAFETY: We allocate the node on the heap; its address remains stable
        // while it is linked in `state.reservations`.
        let mut node = Box::new(VirtmemReservation::new(mem as usize, size));
        let ptr: *mut VirtmemReservation = &mut *node;

        // Insert at the front of the intrusive list.
        state.reservations.push_front(node);

        Some(ptr)
    }

    /// Releases a memory address space reservation.
    pub fn remove_reservation(&mut self, rv: *mut VirtmemReservation) {
        if rv.is_null() {
            return;
        }

        let state = self.0.get_or_insert_with(init_state);

        unsafe {
            // Obtain a cursor pointing at the requested node and unlink it.
            let mut cursor = state.reservations.cursor_mut_from_ptr(rv as *const _);
            if let Some(_boxed) = cursor.remove() {
                // `_boxed` is dropped here, freeing the reservation.
            }
        }
    }
}

impl core::ops::Deref for VirtmemManager {
    type Target = VirtmemState;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("VirtmemManager is not initialized")
    }
}

/// Virtual memory manager state
pub struct VirtmemState {
    alias_region: MemRegion,
    heap_region: MemRegion,
    aslr_region: MemRegion,
    stack_region: MemRegion,
    reservations: LinkedList<ReservationAdapter>,
    is_legacy_kernel: bool,
}

/// Maximum number of attempts to find a random memory region
const RANDOM_MAX_ATTEMPTS: usize = 0x200;

const PAGE_SIZE: usize = 0x1000;
const PAGE_MASK: usize = PAGE_SIZE - 1;

impl VirtmemState {
    /// Finds a random memory region of the given type and size.
    ///
    /// # Arguments
    ///
    /// * `region_type` - The type of memory region to find
    /// * `size` - The size of the memory region to find
    /// * `guard` - The size of the guard area to leave around the memory region
    ///
    /// Returns a pointer to the memory region, or null if no suitable region
    /// is found.
    fn find_random(
        &mut self,
        region_type: RegionType,
        size: usize,
        guard: usize,
    ) -> Option<NonNull<c_void>> {
        // Get the region based on the type
        let region = match region_type {
            RegionType::Aslr => &self.aslr_region,
            RegionType::Stack => &self.stack_region,
            RegionType::CodeMemory => {
                if self.is_legacy_kernel {
                    &self.stack_region
                } else {
                    &self.aslr_region
                }
            }
        };

        // Page align the sizes
        let size = (size + PAGE_MASK) & !PAGE_MASK;
        let guard = (guard + PAGE_MASK) & !PAGE_MASK;

        // Ensure the requested size isn't greater than the memory region itself
        let region_size = region.end - region.start;
        if size > region_size {
            return None;
        }

        // Main allocation loop
        let aslr_max_page_offset = (region_size - size) >> 12;
        for _ in 0..RANDOM_MAX_ATTEMPTS {
            // Calculate a random memory range outside reserved areas
            let region = loop {
                let page_offset = (next_u64() as usize) % (aslr_max_page_offset + 1);
                let addr = region.start + (page_offset << 12);

                let region = MemRegion::new(addr, addr + size);

                // Avoid mapping within the alias region
                if self.alias_region.overlaps_with(&region) {
                    continue;
                }

                // Avoid mapping within the heap region
                if self.heap_region.overlaps_with(&region) {
                    continue;
                }

                break region;
            };

            // Check that there isn't anything mapped at the desired memory range
            if self.is_mapped(&region, guard) {
                continue;
            }

            // Check that the desired memory range doesn't overlap any reservations
            if self.is_reserved(&region, guard) {
                continue;
            }

            // We found a suitable address!
            // SAFETY: We know the address is valid because we checked it above.
            return Some(unsafe { NonNull::new_unchecked(region.base()) });
        }

        None
    }

    /// Check if the memory region is mapped
    ///
    /// Query the memory properties of the region and return true if it's mapped
    #[inline]
    pub fn is_mapped(&self, region: &MemRegion, guard: usize) -> bool {
        // Adjust start/end by the desired guard size
        let query_start = region.start.saturating_sub(guard);
        let query_end = region.end.saturating_add(guard);

        // Query memory properties
        let Ok((info, _)) = mem::query_memory(query_start) else {
            // TODO: diagAbortWithResult(MAKERESULT(Module_Libnx, LibnxError_BadQueryMemory));
            break_event(BreakReason::Panic, 0, 0);
        };

        // Return true if there's anything mapped
        let mem_end = info.addr + info.size;
        if info.typ != MemoryType::Unmapped || query_end > mem_end {
            return true;
        }

        false
    }

    /// Check if the memory region is reserved
    ///
    /// If the queried region overlaps with any reservation, return true.
    /// Otherwise, return false.
    #[inline]
    pub fn is_reserved(&self, region: &MemRegion, guard: usize) -> bool {
        // Adjust start/end by the desired guard size
        let query_start = region.start.saturating_sub(guard);
        let query_end = region.end.saturating_add(guard);

        let query_region = MemRegion::new(query_start, query_end);

        self.reservations
            .iter()
            .any(|rsv| rsv.region.overlaps_with(&query_region))
    }
}

/// Initialize virtual memory manager state
///
/// This function is called when the virtual memory manager is first initialized.
/// It initializes the virtual memory manager state and returns it.
fn init_state() -> VirtmemState {
    // The alias region
    let alias_region = {
        let (alias_region_start, mut alias_region_size) = nx_svc::misc::get_alias_region_info()
            .unwrap_or_else(|_| {
                // TODO: diagAbortWithResult(MAKERESULT(Module_Libnx, LibnxError_WeirdKernel));
                break_event(BreakReason::Panic, 0, 0);
            });

        // Account for the alias region extra size.
        if let Ok(extra) = nx_svc::misc::get_alias_region_extra_size() {
            alias_region_size -= extra;
        }

        MemRegion::new(alias_region_start, alias_region_start + alias_region_size)
    };

    // Reserve the heap region
    let heap_region = {
        let (heap_region_start, heap_region_size) = nx_svc::misc::get_heap_region_info()
            .unwrap_or_else(|_| {
                // TODO: diagAbortWithResult(MAKERESULT(Module_Libnx, LibnxError_BadGetInfo_Heap));
                break_event(BreakReason::Panic, 0, 0);
            });
        MemRegion::new(heap_region_start, heap_region_start + heap_region_size)
    };

    // Retrieve memory region information for the aslr/stack regions
    let (aslr_region, stack_region, is_legacy_kernel) = match nx_svc::misc::get_aslr_region_info() {
        // Modern kernels (2.0.0+) expose ASLR/stack info directly.
        Ok((aslr_region_start, aslr_region_size)) => {
            let (stack_region_start, stack_region_size) = nx_svc::misc::get_stack_region_info()
                .unwrap_or_else(|_| {
                    // TODO: diagAbortWithResult(MAKERESULT(Module_Libnx, LibnxError_BadGetInfo_Stack));
                    break_event(BreakReason::Panic, 0, 0);
                });

            (
                MemRegion::new(aslr_region_start, aslr_region_start + aslr_region_size),
                MemRegion::new(stack_region_start, stack_region_start + stack_region_size),
                false,
            )
        }

        // Legacy kernel (1.0.0) path.
        Err(_) => {
            // [1.0.0] doesn't expose aslr/stack region information so we have to do this dirty hack to detect it.
            // Forgive me.
            let is_legacy_kernel = true;

            // Try to unmap memory to detect kernel bitness
            let res = nx_svc::mem::unmap_memory(
                0xFFFFFFFFFFFFE000usize as *mut _,
                0xFFFFFE000usize as *mut _,
                0x1000,
            );
            let (aslr, stack) = match res {
                // Invalid src-address error means that a valid 36-bit address was rejected.
                // Thus we are 32-bit.
                Err(UnmapMemoryError::InvalidCurrentMemory) => {
                    let aslr = MemRegion::new(0x200000, 0x200000 + 0x100000000);
                    let stack = MemRegion::new(0x200000, 0x200000 + 0x40000000);
                    (aslr, stack)
                }

                // Invalid dst-address error means our 36-bit src-address was valid.
                // Thus we are 36-bit.
                Err(UnmapMemoryError::InvalidMemoryRegion) => {
                    let aslr = MemRegion::new(0x8000000, 0x8000000 + 0x1000000000);
                    let stack = MemRegion::new(0x8000000, 0x8000000 + 0x80000000);
                    (aslr, stack)
                }

                // Should *never* succeed – treat as weird kernel
                _ => {
                    // TODO: diagAbortWithResult(MAKERESULT(Module_Libnx, LibnxError_WeirdKernel));
                    break_event(BreakReason::Panic, 0, 0);
                }
            };

            (aslr, stack, is_legacy_kernel)
        }
    };

    VirtmemState {
        alias_region,
        heap_region,
        aslr_region,
        stack_region,
        is_legacy_kernel,
        reservations: LinkedList::new(ReservationAdapter::new()),
    }
}

/// Intrusive linked-list node representing a memory reservation.
///
/// The layout is compatible with the C `VirtmemReservation` struct so that the
/// returned raw pointer can be passed back to `remove_reservation` unchanged.
pub struct VirtmemReservation {
    /// Link used by the intrusive linked list.
    link: LinkedListLink,
    /// Reserved virtual‐memory range.
    pub(super) region: MemRegion,
}

impl VirtmemReservation {
    fn new(start: usize, size: usize) -> Self {
        Self {
            link: LinkedListLink::new(),
            region: MemRegion::new(start, start + size),
        }
    }
}

// Generate the intrusive-collections adapter so the list knows how to obtain
// the link inside `VirtmemReservation`.
intrusive_adapter!(ReservationAdapter = Box<VirtmemReservation>: VirtmemReservation { link: LinkedListLink });

/// Virtual memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// General purpose ASLR region
    Aslr,
    /// Stack region
    Stack,
    /// Code memory region (version-dependent)
    CodeMemory,
}

/// Memory region bounds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemRegion {
    start: usize,
    end: usize,
}

impl MemRegion {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn is_inside(&self, start: usize, end: usize) -> bool {
        start >= self.start && end <= self.end
    }

    #[inline]
    pub fn overlaps_with(&self, other: &MemRegion) -> bool {
        other.start < self.end && self.start < other.end
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn base(&self) -> *mut c_void {
        self.start as *mut c_void
    }
}
