//! Virtual memory management for Nintendo Switch
//!
//! This module provides C-compatible virtual memory management functions
//! that match the original libnx virtmem API.

use core::{ffi::c_void, ptr::NonNull};

use nx_rand::sys::next_u64;
use nx_svc::mem::{self, MemoryType, UnmapMemoryError};
use nx_sys_sync::data::{Mutex, MutexGuard};

use super::reservation::{
    MANAGED_PAGES, RADIX, RadixBacking, RadixReservationMap, Reservation, ReservationMap,
};

/// Global virtual memory manager
pub(super) static VIRTMEM: Mutex<VirtmemManager> = Mutex::new(VirtmemManager::new_uninit());

/// Lock the virtual memory manager
///
/// This function is equivalent to the C `virtmemLock()` function.
pub fn lock() -> MutexGuard<'static, VirtmemManager> {
    VIRTMEM.lock()
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

    /// Reserves a range of virtual address space, returning the recorded
    /// [`Reservation`].
    ///
    /// Returns `None` when `mem` is null, when `[mem, mem + size)` is not a
    /// non-empty page-aligned range, when the range falls outside the managed
    /// span, or when it overlaps an existing reservation.
    pub fn add_reservation(&mut self, mem: *mut c_void, size: usize) -> Option<Reservation> {
        if mem.is_null() {
            return None;
        }

        // Parse the raw FFI arguments into a page-aligned value handle.
        let range = Reservation::new(mem as usize, size)?;

        let state = self.0.get_or_insert_with(init_state);

        // Reject out-of-span requests (D-3): every in-tree caller reserves an
        // address obtained from `find_*`, which always lies in the managed
        // span — anything else cannot be tracked by the bitmap.
        if !state.reservations.contains(range) {
            return None;
        }

        // Validate the non-overlap invariant at the boundary (IC-3): the
        // one-bit-per-page bitmap is correct only while reservations are
        // disjoint page ranges.
        if state.reservations.is_reserved(range) {
            return None;
        }

        state.reservations.reserve(range);
        Some(range)
    }

    /// Releases a previously recorded virtual address space reservation.
    ///
    /// Releasing a range that is not currently reserved is a safe no-op.
    pub fn remove_reservation(&mut self, range: Reservation) {
        let state = self.0.get_or_insert_with(init_state);
        state.reservations.release(range);
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
    reservations: RadixReservationMap,
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
            panic!("Failed to query memory: BAD_QUERY_MEMORY");
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
        // Guard-expand the query, then snap it to page granularity: round the
        // start down and the end up so the bitmap test covers every page the
        // guarded range touches.
        let query_start = region.start.saturating_sub(guard) & !PAGE_MASK;
        let query_end = (region.end.saturating_add(guard) + PAGE_MASK) & !PAGE_MASK;

        // The expanded range is non-empty and page-aligned, so construction
        // succeeds; a degenerate range would simply reserve nothing.
        match Reservation::new(query_start, query_end - query_start) {
            Some(range) => self.reservations.is_reserved(range),
            None => false,
        }
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
            .expect("Failed to get alias region info: WEIRD_KERNEL");

        // Account for the alias region extra size.
        if let Ok(extra) = nx_svc::misc::get_alias_region_extra_size() {
            alias_region_size -= extra;
        }

        MemRegion::new(alias_region_start, alias_region_start + alias_region_size)
    };

    // Reserve the heap region
    let heap_region = {
        let (heap_region_start, heap_region_size) = nx_svc::misc::get_heap_region_info()
            .expect("Failed to get heap region info: BAD_GET_INFO_HEAP");
        MemRegion::new(heap_region_start, heap_region_start + heap_region_size)
    };

    // Retrieve memory region information for the aslr/stack regions
    let (aslr_region, stack_region, is_legacy_kernel) = match nx_svc::misc::get_aslr_region_info() {
        // Modern kernels (2.0.0+) expose ASLR/stack info directly.
        Ok((aslr_region_start, aslr_region_size)) => {
            let (stack_region_start, stack_region_size) = nx_svc::misc::get_stack_region_info()
                .expect("Failed to get stack region info: BAD_GET_INFO_STACK");

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
                unsafe { NonNull::new_unchecked(0xFFFFFFFFFFFFE000usize as *mut _) },
                unsafe { NonNull::new_unchecked(0xFFFFFE000usize as *mut _) },
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
                    panic!("Unmap memory should not have succeeded: WEIRD_KERNEL");
                }
            };

            (aslr, stack, is_legacy_kernel)
        }
    };

    // The reservation bitmap spans a single contiguous range covering both the
    // ASLR and stack regions (D-3): one map naturally handles the legacy-kernel
    // case where the stack region nests inside the ASLR region.
    let span_start = aslr_region.start.min(stack_region.start);
    let span_end = aslr_region.end.max(stack_region.end);
    // Page count is clamped to `MANAGED_PAGES`: the `RADIX` directory covers
    // exactly the 64 GiB worst case, so a span wider than that (no supported
    // kernel reports one) tracks fewer pages rather than indexing past it.
    let pages = ((span_end - span_start) >> 12).min(MANAGED_PAGES);

    // SAFETY: `init_state` runs at most once per process — `init` and
    // `get_or_insert_with` only call it while `VirtmemManager`'s state is
    // `None` — so exactly one `&'static mut` to `RADIX` is ever created. The
    // borrow then lives inside `VirtmemState` behind the `VIRTMEM` mutex, which
    // serialises every later access to the backing.
    let backing: &'static mut RadixBacking = unsafe { &mut *RADIX.get() };
    let reservations = RadixReservationMap::new(span_start, pages, backing);

    VirtmemState {
        alias_region,
        heap_region,
        aslr_region,
        stack_region,
        is_legacy_kernel,
        reservations,
    }
}

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
