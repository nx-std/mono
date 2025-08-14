//! High-level helpers for Horizon OS stack memory management.
//!
//! This module provides safe abstractions for managing thread stack memory on Horizon OS.
//! It handles the allocation, mapping, and unmapping of stack memory with proper guard pages
//! for stack overflow protection.
//!
//! # Overview
//!
//! Stack memory management on Horizon OS involves several steps:
//! 1. **Allocation**: Create or provide a memory buffer (via [`AlignedBuffer`] or [`ProvidedMemBuffer`])
//! 2. **Mapping**: Map the memory into the process address space with [`map`]
//! 3. **Usage**: Use the mapped memory as thread stack
//! 4. **Unmapping**: Unmap the memory when done with [`unmap`]
//!
//! # Memory Layout
//!
//! The stack memory includes a guard region (4 KiB) to protect against stack overflow.
//! All memory must be page-aligned (4 KiB boundaries) as required by the Horizon OS kernel.

use alloc::alloc::{Layout, alloc_zeroed, dealloc};
use core::{ffi::c_void, ptr::NonNull};

use nx_svc::mem::core as svc;

use crate::vmm::sys as vmm;

/// Guard size 0x4000, per libnx
///
/// See `libnx/source/kernel/thread.c` for more details.
const GUARD_SIZE: usize = 0x4000;

const PAGE_SIZE: usize = 0x1000;

/// Represents stack memory that has been allocated but not yet mapped into the process address space.
///
/// This is the initial state of stack memory after allocation. The memory exists but is not
/// accessible until it is mapped using the [`map`] function.
#[derive(Debug)]
pub struct UnmappedStackMemory<B: MemBuf> {
    buffer: B,
}

impl<B> UnmappedStackMemory<B>
where
    B: MemBuf,
{
    /// Create a new `UnmappedStackMemory` with the given buffer.
    pub fn new(buffer: B) -> Self {
        Self { buffer }
    }
}

/// Represents stack memory that has been mapped into the process address space.
///
/// This structure holds both the underlying memory buffer and the virtual address where
/// the stack memory has been mapped. The mapped memory can be accessed through the
/// `mapped_mem_ptr` pointer.
///
/// When dropped or explicitly unmapped using [`unmap`], the memory will be unmapped
/// from the process address space.
#[derive(Debug)]
pub struct MappedStackMemory<B: MemBuf> {
    buffer: B,
    mapped_mem_ptr: NonNull<c_void>,
}

impl<B> MappedStackMemory<B>
where
    B: MemBuf,
{
    /// Returns the pointer to the mapped memory.
    pub fn mapped_mem_ptr(&self) -> NonNull<c_void> {
        self.mapped_mem_ptr
    }
}

/// Map the [`StackMemory`] instance into the current process.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn map<B>(sm: UnmappedStackMemory<B>) -> Result<MappedStackMemory<B>, MapError>
where
    B: MemBuf,
{
    let UnmappedStackMemory { buffer } = sm;

    // Lock the VMM and reserve a virtual address range for the stack memory.
    let Some(ptr) = vmm::lock().find_stack(buffer.size(), GUARD_SIZE) else {
        return Err(MapError::VirtAddrAllocFailed);
    };

    // Attempt to map the shared memory into the process stack address space.
    svc::map_memory(ptr, buffer.ptr(), buffer.size()).map_err(MapError::Svc)?;

    Ok(MappedStackMemory {
        buffer,
        mapped_mem_ptr: ptr,
    })
}

/// Errors that can occur when mapping stack memory.
#[derive(Debug, thiserror::Error)]
pub enum MapError {
    /// Failed to find available virtual address space for the stack.
    ///
    /// This occurs when the virtual memory manager cannot locate a suitable
    /// contiguous region in the process's address space to map the stack memory.
    #[error("Failed to allocate virtual address range for stack mapping")]
    VirtAddrAllocFailed,

    /// System call to map memory failed.
    #[error(transparent)]
    Svc(#[from] svc::MapMemoryError),
}

/// Unmap the shared-memory object from the current process.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn unmap<B>(sm: MappedStackMemory<B>) -> Result<UnmappedStackMemory<B>, UnmapError>
where
    B: MemBuf,
{
    let MappedStackMemory {
        buffer,
        mapped_mem_ptr,
    } = sm;

    // Ensure the memory is properly unmapped from the process address space.
    svc::unmap_memory(mapped_mem_ptr, buffer.ptr(), buffer.size()).map_err(UnmapError::Svc)?;

    Ok(UnmappedStackMemory { buffer })
}

/// Errors that can occur when unmapping stack memory.
#[derive(Debug, thiserror::Error)]
pub enum UnmapError {
    /// System call to unmap memory failed.
    #[error(transparent)]
    Svc(#[from] svc::UnmapMemoryError),
}

/// Trait for memory buffer implementations.
pub trait MemBuf {
    /// Get the pointer to the buffer's memory.
    fn ptr(&self) -> NonNull<c_void>;

    /// Get the size of the buffer in bytes.
    fn size(&self) -> usize;
}

/// Buffer for stack memory that is owned and will be deallocated on drop.
#[derive(Debug)]
pub struct AlignedBuffer {
    /// The memory layout used for allocation and deallocation.
    ///
    /// This layout ensures proper size and alignment for the stack memory.
    layout: Layout,

    /// The pointer to the stack memory buffer.
    ///
    /// This is the pointer to the raw memory allocated via the system allocator.
    ptr: NonNull<c_void>,
}

impl AlignedBuffer {
    /// Allocate a new owned memory buffer of the specified size.
    ///
    /// The memory is zero-initialized and page-aligned.
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
    #[error("Size must be non-zero")]
    InvalidSize,

    /// Size must be a multiple of the page size (4 KiB).
    #[error("Size must be page-aligned (0x1000)")]
    InvalidAlignment,

    /// Memory allocation failed.
    #[error("Memory allocation failed")]
    AllocationFailed,
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        // SAFETY: The memory was allocated with `alloc`, so we can safely deallocate it using the
        // same layout.
        unsafe { dealloc(self.ptr.as_ptr().cast(), self.layout) };
    }
}

impl MemBuf for AlignedBuffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.ptr
    }

    fn size(&self) -> usize {
        self.layout.size()
    }
}

impl<'a> MemBuf for &'a AlignedBuffer {
    fn ptr(&self) -> NonNull<c_void> {
        self.ptr
    }

    fn size(&self) -> usize {
        self.layout.size()
    }
}
