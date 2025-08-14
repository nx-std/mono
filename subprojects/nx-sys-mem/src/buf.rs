//! Memory buffer management for page-aligned allocations.
//!
//! This module provides abstractions for working with page-aligned memory buffers,
//! which are commonly used for stack memory and other system-level memory allocations
//! that require specific alignment constraints.
//!
//! # Overview
//!
//! The module defines:
//! - [`Buf`] trait: A common interface for memory buffer implementations
//! - [`PageAlignedBuffer`]: An owned buffer with page-aligned memory allocation
//! - [`BufAllocError`]: Error types for buffer allocation failures
//!
//! # Memory Alignment
//!
//! All buffers allocated through this module are aligned to page boundaries (4 KiB).
//! This alignment is required for certain system operations and helps improve
//! performance for memory-mapped operations.
//!
//! # Example
//!
//! ```no_run
//! use nx_sys_mem::buf::PageAlignedBuffer;
//!
//! // Allocate a 16 KiB page-aligned buffer
//! let buffer = PageAlignedBuffer::alloc(0x4000).unwrap();
//!
//! // The buffer automatically deallocates when dropped
//! ```

use alloc::alloc::{alloc_zeroed, dealloc};
use core::{alloc::Layout, ffi::c_void, ptr::NonNull};

/// Standard page size for memory alignment (4 KiB).
///
/// This constant defines the alignment boundary for all page-aligned
/// allocations in the system.
const PAGE_SIZE: usize = 0x1000;

/// Trait for memory buffer implementations.
///
/// This trait provides a common interface for different types of memory buffers,
/// allowing generic code to work with any buffer implementation that provides
/// a pointer and size.
///
/// # Safety
///
/// Implementations must ensure that:
/// - The pointer returned by `ptr()` remains valid for the lifetime of the buffer
/// - The size returned by `size()` accurately reflects the allocated memory size
/// - The memory region from `ptr` to `ptr + size` is valid and accessible
pub trait Buf {
    /// Get the pointer to the buffer's memory.
    ///
    /// Returns a non-null pointer to the beginning of the buffer's memory region.
    /// This pointer remains valid for the lifetime of the buffer.
    fn ptr(&self) -> NonNull<c_void>;

    /// Get the size of the buffer in bytes.
    ///
    /// Returns the total size of the allocated memory buffer.
    fn size(&self) -> usize;
}

/// Buffer for page-aligned memory that is owned and will be deallocated on drop.
///
/// `PageAlignedBuffer` provides an RAII wrapper around page-aligned memory allocations.
/// The buffer is automatically deallocated when it goes out of scope, preventing memory leaks.
///
/// # Memory Characteristics
///
/// - **Alignment**: All buffers are aligned to page boundaries (4 KiB)
/// - **Initialization**: Memory is zero-initialized upon allocation
/// - **Ownership**: The buffer owns the memory and deallocates it on drop
///
/// # Use Cases
///
/// This buffer type is particularly useful for:
/// - Stack memory for thread creation
/// - Memory-mapped I/O buffers
/// - DMA transfer buffers
/// - Any scenario requiring page-aligned memory
#[derive(Debug)]
pub struct PageAlignedBuffer {
    /// The memory layout used for allocation and deallocation.
    ///
    /// This layout ensures proper size and alignment for the stack memory.
    layout: Layout,

    /// The pointer to the stack memory buffer.
    ///
    /// This is the pointer to the raw memory allocated via the system allocator.
    ptr: NonNull<c_void>,
}

impl PageAlignedBuffer {
    /// Allocate a new owned memory buffer of the specified size.
    ///
    /// Creates a new page-aligned buffer with the specified size. The memory
    /// is zero-initialized to ensure no uninitialized data is exposed.
    ///
    /// # Arguments
    ///
    /// * `size` - The size of the buffer in bytes. Must be:
    ///   - Non-zero
    ///   - A multiple of the page size (0x1000 / 4 KiB)
    ///
    /// Returns `Ok(PageAlignedBuffer)` on success, or a [`BufAllocError`] if:
    /// - The size is zero ([`BufAllocError::InvalidSize`])
    /// - The size is not page-aligned ([`BufAllocError::InvalidAlignment`])
    /// - Memory allocation fails ([`BufAllocError::AllocationFailed`])
    pub fn alloc(size: usize) -> Result<Self, BufAllocError> {
        // Size must be non-zero
        if size == 0 {
            return Err(BufAllocError::InvalidSize);
        }

        // Ensure size must be page-aligned (multiple of PAGE_SIZE)
        if size & (PAGE_SIZE - 1) != 0 {
            return Err(BufAllocError::InvalidAlignment);
        }

        // SAFETY: Size and alignment are guaranteed to be valid.
        let layout = unsafe { Layout::from_size_align_unchecked(size, PAGE_SIZE) };
        let ptr = unsafe { alloc_zeroed(layout) } as *mut c_void;
        let Some(ptr) = NonNull::new(ptr) else {
            return Err(BufAllocError::AllocationFailed);
        };

        Ok(Self { ptr, layout })
    }
}

/// Errors that can occur during memory buffer allocation.
#[derive(Debug, thiserror::Error)]
pub enum BufAllocError {
    /// Size must be non-zero.
    ///
    /// Attempted to allocate a buffer with size 0. All buffers must have
    /// a positive size.
    #[error("Size must be non-zero")]
    InvalidSize,

    /// Size must be a multiple of the page size (4 KiB).
    ///
    /// The requested size is not aligned to page boundaries. All sizes must
    /// be multiples of 0x1000 (4096 bytes).
    #[error("Size must be page-aligned (0x1000)")]
    InvalidAlignment,

    /// Memory allocation failed.
    ///
    /// The system allocator was unable to allocate the requested memory.
    /// This typically occurs when the system is out of memory or the
    /// requested size is too large.
    #[error("Memory allocation failed")]
    AllocationFailed,
}

impl Drop for PageAlignedBuffer {
    fn drop(&mut self) {
        // SAFETY: The memory was allocated with `alloc_zeroed` using the stored layout,
        // so we can safely deallocate it using the same layout. The pointer is guaranteed
        // to be valid as it was checked during allocation.
        unsafe { dealloc(self.ptr.as_ptr().cast(), self.layout) };
    }
}

impl Buf for PageAlignedBuffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.ptr
    }

    fn size(&self) -> usize {
        self.layout.size()
    }
}

/// Implementation of [`Buf`] for references to [`PageAlignedBuffer`].
///
/// This allows borrowed references to be used wherever the [`Buf`] trait
/// is required.
impl<'a> Buf for &'a PageAlignedBuffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.ptr
    }

    fn size(&self) -> usize {
        self.layout.size()
    }
}
