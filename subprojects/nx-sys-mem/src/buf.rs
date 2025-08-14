//! Memory buffer management with flexible allocation strategies.
//!
//! This module provides abstractions for working with memory buffers, including
//! general-purpose buffers and specialized page-aligned buffers commonly used
//! for stack memory and system-level allocations.
//!
//! # Overview
//!
//! The module defines:
//! - [`Buf`] trait: A common interface for memory buffer implementations
//! - [`Buffer`]: An owned buffer with custom layout support
//! - [`BufferRef`]: A non-owning reference to externally managed memory
//! - [`PageAlignedBuffer`]: An owned buffer with page-aligned memory allocation

use alloc::alloc::{Layout, alloc_zeroed, dealloc};
use core::{ffi::c_void, marker::PhantomData, mem::align_of, ptr::NonNull};

use crate::alignment::{PAGE_SIZE, is_page_aligned};

/// Trait for memory buffer implementations.
///
/// This trait provides a common interface for different types of memory buffers,
/// allowing generic code to work with any buffer implementation that provides
/// a pointer, size, and alignment.
///
/// # Safety
///
/// Implementations must ensure that:
/// - The pointer returned by `ptr()` remains valid for the lifetime of the buffer
/// - The layout returned by `layout()` accurately describes the buffer's memory characteristics
/// - The memory region from `ptr` to `ptr + size` is valid and accessible
pub trait Buf {
    /// Get the pointer to the buffer's memory.
    ///
    /// Returns a non-null pointer to the beginning of the buffer's memory region.
    /// This pointer remains valid for the lifetime of the buffer.
    fn ptr(&self) -> NonNull<c_void>;

    /// Get the memory layout of the buffer.
    ///
    /// Returns the layout describing the buffer's size and alignment.
    fn layout(&self) -> Layout;

    /// Get the size of the buffer in bytes.
    ///
    /// Returns the total size of the allocated memory buffer.
    fn size(&self) -> usize {
        self.layout().size()
    }

    /// Get the alignment of the buffer in bytes.
    ///
    /// Returns the alignment constraint of the allocated memory buffer.
    fn align(&self) -> usize {
        self.layout().align()
    }
}

/// Buffer for memory that is owned and will be deallocated on drop.
///
/// `Buffer` provides an RAII wrapper around memory allocations with custom layouts.
/// The buffer is automatically deallocated when it goes out of scope, preventing memory leaks.
///
/// # Memory Characteristics
///
/// - **Alignment**: Determined by the provided layout
/// - **Initialization**: Memory is zero-initialized upon allocation
/// - **Ownership**: The buffer owns the memory and deallocates it on drop
///
/// # Use Cases
///
/// This buffer type is useful for:
/// - General purpose memory allocations with specific layout requirements
/// - Buffers that don't require page alignment
/// - Memory pools with custom alignment needs
#[derive(Debug)]
pub struct Buffer {
    /// The memory layout used for allocation and deallocation.
    ///
    /// This layout ensures proper size and alignment for the memory.
    layout: Layout,

    /// The pointer to the memory buffer.
    ///
    /// This is the pointer to the raw memory allocated via the system allocator.
    ptr: NonNull<c_void>,
}

impl Buffer {
    /// Allocate a new buffer with the specified layout.
    ///
    /// Creates a new buffer with the specified size and alignment from the layout.
    /// The memory is zero-initialized to ensure no uninitialized data is exposed.
    ///
    /// # Panics
    ///
    /// Panics if memory allocation fails.
    pub fn with_layout(layout: Layout) -> Self {
        Self::try_with_layout(layout).expect("failed to allocate buffer with specified layout")
    }

    /// Try to allocate a new buffer with the specified layout.
    ///
    /// Creates a new buffer with the specified size and alignment from the layout.
    /// The memory is zero-initialized to ensure no uninitialized data is exposed.
    pub fn try_with_layout(layout: Layout) -> Result<Self, AllocationError> {
        let ptr = unsafe { alloc_zeroed(layout) } as *mut c_void;
        let Some(ptr) = NonNull::new(ptr) else {
            return Err(AllocationError);
        };

        Ok(Self { ptr, layout })
    }

    /// Allocate a new buffer with the specified capacity.
    ///
    /// Creates a new buffer with at least the specified capacity in bytes.
    /// The actual size may be larger to satisfy alignment requirements.
    /// The buffer will have a default alignment matching `c_void`.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The minimum capacity in bytes. Must be non-zero.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The capacity is zero
    /// - Memory allocation fails
    pub fn with_capacity(capacity: usize) -> Self {
        Self::try_with_capacity(capacity)
            .expect("failed to allocate buffer with specified capacity")
    }

    /// Try to allocate a new buffer with the specified capacity.
    ///
    /// Creates a new buffer with at least the specified capacity in bytes.
    /// The actual size may be larger to satisfy alignment requirements.
    /// The buffer will have a default alignment matching `c_void`.
    ///
    /// Returns `Ok(Buffer)` on success, or a [`BufWithCapacityError`] if:
    /// - Layout creation fails ([`BufWithCapacityError::InvalidLayout`])
    /// - Memory allocation fails ([`BufWithCapacityError::AllocationFailed`])
    pub fn try_with_capacity(capacity: usize) -> Result<Self, BufWithCapacityError> {
        // Capacity must be non-zero
        debug_assert!(capacity != 0, "capacity must be non-zero");

        let layout = Layout::from_size_align(capacity, align_of::<c_void>())
            .map_err(|_| BufWithCapacityError::InvalidLayout)?;

        Self::try_with_layout(layout).map_err(|_| BufWithCapacityError::AllocationFailed)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: The memory was allocated with `alloc_zeroed` using the stored layout,
        // so we can safely deallocate it using the same layout. The pointer is guaranteed
        // to be valid as it was checked during allocation.
        unsafe { dealloc(self.ptr.as_ptr().cast(), self.layout) };
    }
}

impl Buf for Buffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.ptr
    }

    fn layout(&self) -> Layout {
        self.layout
    }
}

/// Implementation of [`Buf`] for references to [`Buffer`].
///
/// This allows borrowed references to be used wherever the [`Buf`] trait
/// is required.
impl<'a> Buf for &'a Buffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.ptr
    }

    fn layout(&self) -> Layout {
        self.layout
    }
}

/// Error that occurs when memory buffer allocation fails.
///
/// The system allocator was unable to allocate the requested memory.
/// This typically occurs when the system is out of memory or the
/// requested size is too large.
#[derive(Debug, thiserror::Error)]
#[error("Memory allocation failed")]
pub struct AllocationError;

/// Errors that can occur when creating a buffer with a capacity.
#[derive(Debug, thiserror::Error)]
pub enum BufWithCapacityError {
    /// Invalid layout parameters.
    ///
    /// The capacity and alignment combination resulted in an invalid layout.
    #[error("Invalid layout parameters")]
    InvalidLayout,

    /// Memory allocation failed.
    ///
    /// The system allocator was unable to allocate the requested memory.
    #[error("Memory allocation failed")]
    AllocationFailed,
}

/// A non-owning reference to a memory buffer.
///
/// `BufferRef` wraps a pointer to memory that is managed externally. It does not
/// allocate or deallocate memory, but provides a way to access an existing memory region
/// with explicit lifetime tracking.
///
/// # Memory Characteristics
/// - **Ownership**: The buffer does not own the memory; it is provided by the caller.
/// - **Lifetime**: The memory must remain valid for the lifetime `'a`.
///
/// # Example Use Cases
/// - Main thread stack (with `'static` lifetime)
/// - Borrowed memory regions from other buffers
/// - Memory mapped from external sources
pub struct BufferRef<'a> {
    /// The memory layout used for the buffer.
    ///
    /// This layout ensures proper size and alignment for the memory.
    layout: Layout,

    /// The pointer to the provided memory buffer.
    ///
    /// This is a non-owning pointer to memory that is managed externally.
    ptr: NonNull<c_void>,

    /// Phantom data to track the lifetime of the referenced memory.
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> BufferRef<'a> {
    /// Create a new buffer reference from a raw pointer and layout.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - The pointer is valid and points to a memory region matching the layout's size and alignment.
    /// - The memory layout accurately describes the buffer's characteristics.
    /// - The memory remains valid for the lifetime `'a`.
    pub unsafe fn from_parts(ptr: NonNull<c_void>, layout: Layout) -> Self {
        Self {
            ptr,
            layout,
            _lifetime: PhantomData,
        }
    }
}

impl<'a> Buf for BufferRef<'a> {
    fn ptr(&self) -> NonNull<c_void> {
        self.ptr
    }

    fn layout(&self) -> Layout {
        self.layout
    }
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
pub struct PageAlignedBuffer(Buffer);

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
    /// Returns `Ok(PageAlignedBuffer)` on success, or a [`PageAlignedBufError`] if:
    /// - The size is zero ([`PageAlignedBufError::InvalidSize`])
    /// - The size is not page-aligned ([`PageAlignedBufError::InvalidAlignment`])
    /// - Memory allocation fails ([`PageAlignedBufError::AllocationFailed`])
    pub fn alloc(size: usize) -> Result<Self, PageAlignedBufError> {
        // Size must be non-zero
        if size == 0 {
            return Err(PageAlignedBufError::InvalidSize);
        }

        // Ensure size must be page-aligned (multiple of PAGE_SIZE)
        if !is_page_aligned(size) {
            return Err(PageAlignedBufError::InvalidAlignment);
        }

        let layout = Layout::from_size_align(size, PAGE_SIZE)
            .map_err(|_| PageAlignedBufError::InvalidSize)?;
        let inner =
            Buffer::try_with_layout(layout).map_err(|_| PageAlignedBufError::AllocationFailed)?;

        Ok(Self(inner))
    }
}

impl<'a> From<&'a PageAlignedBuffer> for BufferRef<'a> {
    fn from(buffer: &'a PageAlignedBuffer) -> Self {
        Self {
            ptr: buffer.0.ptr(),
            layout: buffer.0.layout(),
            _lifetime: PhantomData,
        }
    }
}

impl Buf for PageAlignedBuffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.0.ptr()
    }

    fn layout(&self) -> Layout {
        self.0.layout()
    }
}

/// Implementation of [`Buf`] for references to [`PageAlignedBuffer`].
///
/// This allows borrowed references to be used wherever the [`Buf`] trait
/// is required.
impl<'a> Buf for &'a PageAlignedBuffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.0.ptr()
    }

    fn layout(&self) -> Layout {
        self.0.layout()
    }
}

/// Errors that can occur during allocation of a page-aligned buffer.
#[derive(Debug, thiserror::Error)]
pub enum PageAlignedBufError {
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
