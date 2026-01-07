use core::{ffi::c_void, ptr};

use self::meta::{Allocation, Layout};
use crate::global as global_allocator;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_alloc__malloc(size: usize) -> *mut c_void {
    let Ok(layout) = Layout::from_size(size) else {
        return ptr::null_mut();
    };

    // Critical section
    let alloc_ptr = {
        let mut alloc = global_allocator::lock();

        let raw_alloc_ptr = unsafe { alloc.malloc(layout.size(), layout.align()) };
        let Some(alloc_ptr) = ptr::NonNull::new(raw_alloc_ptr) else {
            return ptr::null_mut();
        };

        alloc_ptr
    };

    let allocation = unsafe { Allocation::new_with_metadata(alloc_ptr, layout) };
    allocation.data_ptr() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_alloc__aligned_alloc(align: usize, size: usize) -> *mut c_void {
    let Ok(layout) = Layout::from_size_align(size, align) else {
        return ptr::null_mut();
    };

    // Critical section
    let alloc_ptr = {
        let mut alloc = global_allocator::lock();

        let raw_alloc_ptr = unsafe { alloc.malloc(layout.size(), layout.align()) };
        let Some(alloc_ptr) = ptr::NonNull::new(raw_alloc_ptr) else {
            return ptr::null_mut();
        };

        alloc_ptr
    };

    let allocation = unsafe { Allocation::new_with_metadata(alloc_ptr, layout) };
    allocation.data_ptr() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_alloc__free(ptr: *mut c_void) {
    let Some(alloc_ptr) = ptr::NonNull::new(ptr) else {
        return; // If the pointer is null, no-op
    };

    let allocation = unsafe { Allocation::from_data_ptr(alloc_ptr) };

    let mut alloc = global_allocator::lock();
    unsafe { alloc.free(allocation.as_ptr(), allocation.size(), allocation.align()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_alloc__calloc(nmemb: usize, size: usize) -> *mut c_void {
    // Check for overflow
    let total = match nmemb.checked_mul(size) {
        Some(t) => t,
        None => return ptr::null_mut(),
    };
    let Ok(layout) = Layout::from_size(total) else {
        return ptr::null_mut();
    };

    // Critical section
    let alloc_ptr = {
        let mut alloc = global_allocator::lock();

        // Allocate new block
        let raw_alloc_ptr = unsafe { alloc.malloc(layout.size(), layout.align()) };
        let Some(alloc_ptr) = ptr::NonNull::new(raw_alloc_ptr) else {
            return ptr::null_mut();
        };

        alloc_ptr
    };

    // Zero the allocation
    unsafe { ptr::write_bytes(alloc_ptr.as_ptr(), 0, layout.size()) };

    let allocation = unsafe { Allocation::new_with_metadata(alloc_ptr, layout) };
    allocation.data_ptr() as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_alloc__realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    // If ptr is null, realloc is equivalent to malloc
    if ptr.is_null() {
        return unsafe { __nx_alloc__malloc(new_size) };
    }

    // If new_size is zero, free and return null
    if new_size == 0 {
        unsafe { __nx_alloc__free(ptr) };
        return ptr::null_mut();
    }

    // Get old allocation metadata
    let Some(alloc_ptr) = ptr::NonNull::new(ptr) else {
        return ptr::null_mut();
    };
    let allocation = unsafe { Allocation::from_data_ptr(alloc_ptr) };
    let old_size = allocation.size();
    let align = allocation.align();

    let Ok(layout) = Layout::from_size_align(new_size, align) else {
        return ptr::null_mut();
    };

    // Critical section
    let new_alloc_ptr = {
        let mut alloc = global_allocator::lock();

        // Allocate new block
        let raw_alloc_ptr = unsafe { alloc.malloc(layout.size(), layout.align()) };
        let Some(new_alloc_ptr) = ptr::NonNull::new(raw_alloc_ptr) else {
            return ptr::null_mut();
        };

        // Copy old data to new allocation (up to the minimum of old and new size)
        let copy_size = old_size.min(layout.size());

        // Safety: The pointers are valid and the size is non-zero
        unsafe { ptr::copy_nonoverlapping(allocation.as_ptr(), new_alloc_ptr.as_ptr(), copy_size) };

        // Free old allocation
        unsafe { alloc.free(allocation.as_ptr(), old_size, align) };

        new_alloc_ptr
    };

    // Write new metadata and return pointer to data
    let new_allocation = unsafe { Allocation::new_with_metadata(new_alloc_ptr, layout) };
    new_allocation.data_ptr() as *mut c_void
}

mod newlib {
    use core::ffi::c_void;

    use super::{
        __nx_alloc__aligned_alloc, __nx_alloc__calloc, __nx_alloc__free, __nx_alloc__malloc,
        __nx_alloc__realloc,
    };

    /// Opaque newlib reentrant struct
    #[repr(C)]
    pub struct Reent {
        _priv: [u8; 0],
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __nx_alloc__newlib_malloc_r(
        _: *mut Reent,
        size: usize,
    ) -> *mut c_void {
        unsafe { __nx_alloc__malloc(size) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __nx_alloc__newlib_calloc_r(
        _: *mut Reent,
        nmemb: usize,
        size: usize,
    ) -> *mut c_void {
        unsafe { __nx_alloc__calloc(nmemb, size) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __nx_alloc__newlib_realloc_r(
        _: *mut Reent,
        ptr: *mut c_void,
        new_size: usize,
    ) -> *mut c_void {
        unsafe { __nx_alloc__realloc(ptr, new_size) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __nx_alloc__newlib_memalign_r(
        _: *mut Reent,
        align: usize,
        size: usize,
    ) -> *mut c_void {
        unsafe { __nx_alloc__aligned_alloc(align, size) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn __nx_alloc__newlib_free_r(_: *mut Reent, ptr: *mut c_void) {
        unsafe { __nx_alloc__free(ptr) }
    }
}

mod meta {
    use core::{alloc::Layout as AllocLayout, ffi::c_void, mem, ptr};

    /// A memory allocation with metadata.
    ///
    /// This is a wrapper around a pointer to the allocated memory.
    pub struct Allocation(ptr::NonNull<AllocationRepr>);

    impl Allocation {
        /// Create a new allocation, with the given layout metadata.
        ///
        /// This function writes the layout metadata to the given pointer.
        ///
        /// # Safety
        /// The pointer must be valid and point to the start of the allocation.
        /// The layout must be valid.
        pub unsafe fn new_with_metadata<T>(ptr: ptr::NonNull<T>, layout: Layout) -> Self {
            let mut alloc = ptr.cast::<AllocationRepr>();

            // Write layout metadata to the allocation
            unsafe {
                ptr::write(
                    &mut alloc.as_mut().meta,
                    MetaRepr {
                        size: layout.size,
                        align: layout.align,
                        offset: layout.offset,
                    },
                )
            };

            Self(alloc)
        }

        /// Create an allocation from a pointer to the allocated memory.
        ///
        /// # Safety
        /// The pointer must be valid and point to the start of the allocation data.
        pub unsafe fn from_data_ptr(ptr: ptr::NonNull<c_void>) -> Self {
            let data_ptr = ptr.as_ptr() as *mut u8;

            // The offset is stored just before the data pointer.
            let offset_ptr = unsafe { data_ptr.sub(mem::size_of::<usize>()) };
            let offset = unsafe { ptr::read(offset_ptr as *const usize) };

            // The allocation starts at `data_ptr - offset`
            let alloc_ptr = unsafe { data_ptr.sub(offset) };

            Self(unsafe { ptr::NonNull::new_unchecked(alloc_ptr.cast()) })
        }

        /// Get the allocation size from the metadata.
        pub fn size(&self) -> usize {
            let this = unsafe { self.0.as_ref() };
            this.meta.size
        }

        /// Get the allocation alignment from the metadata.
        pub fn align(&self) -> usize {
            let this = unsafe { self.0.as_ref() };
            this.meta.align
        }

        /// Get a raw pointer to the allocated memory.
        pub fn as_ptr(&self) -> *mut u8 {
            self.0.as_ptr() as *mut u8
        }

        /// Get the pointer to the allocated memory.
        pub fn data_ptr(&self) -> *mut c_void {
            let this = unsafe { self.0.as_ref() };
            let alloc_ptr = self.0.as_ptr() as *mut u8;
            unsafe {
                let data_ptr = alloc_ptr.add(this.meta.offset);
                // Store the offset right before the data pointer for `from_data_ptr`.
                let offset_ptr = data_ptr.sub(mem::size_of::<usize>()) as *mut usize;
                ptr::write(offset_ptr, this.meta.offset);
                data_ptr as *mut c_void
            }
        }
    }

    /// Allocation layout metadata
    pub struct Layout {
        size: usize,
        align: usize,
        offset: usize,
    }

    impl Layout {
        /// Create a new layout with the given size.
        ///
        /// The final size of the allocation will be `size` + size of the layout metadata.
        /// The alignment will be the maximum of the alignment of the layout metadata and 8.
        pub fn from_size(size: usize) -> Result<Self, LayoutError> {
            Self::from_size_align(size, mem::align_of::<*mut c_void>())
        }

        /// Create a new layout with the given size and alignment.
        ///
        /// The final size of the allocation will be `size` + padding + size of the layout metadata.
        /// The alignment will be the given alignment.
        pub fn from_size_align(size: usize, align: usize) -> Result<Self, LayoutError> {
            // Reject zero, non-power-of-two, or otherwise invalid alignments.
            if align == 0 || !align.is_power_of_two() {
                return Err(LayoutError);
            }

            // We need to store the metadata, plus a usize for the offset, before the user's data.
            let meta_size = mem::size_of::<MetaRepr>() + mem::size_of::<usize>();
            let data_align = align;

            let data_layout = AllocLayout::from_size_align(size, data_align)
                .map_err(|_| LayoutError)?
                .pad_to_align();

            // The offset to the user data must be a multiple of the alignment,
            // and large enough to hold the metadata.
            let offset = (meta_size + data_align - 1) & !(data_align - 1);

            let total_size = offset.checked_add(data_layout.size()).ok_or(LayoutError)?;

            // The allocation must be aligned to the user's requested alignment.
            let alloc_align = data_align;

            Ok(Self {
                size: total_size,
                align: alloc_align,
                offset,
            })
        }

        /// Get the size of the layout.
        ///
        /// The returned size includes the size of the layout metadata: `size + size_of(metadata)`.
        pub fn size(&self) -> usize {
            self.size
        }

        /// Get the alignment of the layout.
        pub fn align(&self) -> usize {
            self.align
        }
    }

    /// Internal allocation representation
    #[repr(C)]
    struct AllocationRepr {
        meta: MetaRepr,
        data_ptr: [u8; 0],
    }

    /// Internal layout metadata representation
    #[repr(C)]
    struct MetaRepr {
        size: usize,
        align: usize,
        offset: usize,
    }

    /// An error that can occur when creating a layout.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    #[error("invalid parameters to Layout::from_size_align")]
    pub struct LayoutError;
}
