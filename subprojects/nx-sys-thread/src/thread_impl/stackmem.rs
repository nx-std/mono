use core::{alloc::Layout, ffi::c_void, ptr::NonNull};

use nx_sys_mem::{
    alignment::{PAGE_SIZE, is_page_aligned},
    buf::{Buf, Buffer, BufferRef},
    stack::{
        self as stack_mem, MapError as StackMemMapError, MappedStackMemory,
        MappedStackMemory as StackMem,
    },
};

/// Thread stack memory
///
/// This memory, backed by a page-aligned memory buffer is mapped into the process stack address
/// space.
pub struct ThreadStackMem<B>(StackMem<B>);

impl<B> ThreadStackMem<B>
where
    B: Buf,
{
    /// Given a buffer, maps it into the thread stack memory address space
    pub fn map(buffer: B) -> Result<Self, StackMemMapError> {
        unsafe { stack_mem::map(buffer) }.map(ThreadStackMem)
    }

    /// Returns a pointer to the thread stack memory
    pub fn memory_ptr(&self) -> NonNull<c_void> {
        self.0.buffer_ptr()
    }

    /// Returns a pointer to the thread stack memory mirror
    pub fn mirror_ptr(&self) -> NonNull<c_void> {
        self.0.mapped_mem_ptr()
    }

    /// Returns the size of the thread stack memory
    pub fn size(&self) -> usize {
        self.0.size()
    }
}

impl<'a> ThreadStackMem<BufferRef<'a>> {
    /// Creates a new `ThreadStackMem` from a provided pointer and size.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// * `ptr` is a valid, non-null pointer to a memory region that is aligned to `PAGE_SIZE`.
    /// * `size` is a multiple of `PAGE_SIZE`.
    /// * The memory region pointed to by `ptr` is mapped into the process stack address space
    ///   and is not used by any other thread or process.
    /// * The memory region must remain valid for the lifetime of the `ThreadStackMem`.
    pub unsafe fn from_provided(ptr: NonNull<c_void>, size: usize) -> Self {
        // SAFETY: The caller must ensure that `ptr` is valid and aligned to `PAGE_SIZE`
        // and that `size` is a multiple of `PAGE_SIZE`.
        let layout = unsafe { Layout::from_size_align_unchecked(size, PAGE_SIZE) };

        // SAFETY: The pointer is non-null and the layout is valid.
        let buffer = unsafe { BufferRef::from_raw_parts(ptr, layout) };

        // SAFETY: The buffer is created from a valid pointer and size, and it is
        // guaranteed to be page-aligned.
        unsafe { ThreadStackMem(MappedStackMemory::from_raw_parts(buffer, ptr)) }
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
        // SAFETY: The buffer is guaranteed to be valid and page-aligned.
        // The `PageAlignedBuffer` is constructed from a valid `Buffer`, so we can
        // safely create a `BufferRef` from it.
        unsafe { BufferRef::from_raw_parts(buffer.0.ptr(), buffer.0.layout()) }
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
