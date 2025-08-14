//! High-level helpers for Horizon OS stack memory management.
//!
//! This module provides safe abstractions for managing thread stack memory on Horizon OS.
//! It handles the allocation, mapping, and unmapping of stack memory with proper guard pages
//! for stack overflow protection.
//!
//! # Overview
//!
//! Stack memory management on Horizon OS involves several steps:
//! 1. **Allocation**: Create or provide a memory buffer implementing the [`Buf`] trait
//! 2. **Mapping**: Map the memory into the process address space with [`map`]
//! 3. **Usage**: Use the mapped memory as thread stack
//! 4. **Unmapping**: Unmap the memory when done with [`unmap`]
//!
//! # Memory Layout
//!
//! The stack memory includes a guard region (16 KiB) to protect against stack overflow.
//! All memory must be page-aligned (4 KiB boundaries) as required by the Horizon OS kernel.

use core::{ffi::c_void, ptr::NonNull};

use nx_svc::mem::core as svc;

use crate::{buf::Buf, vmm};

/// Size of the guard region for stack overflow protection (16 KiB).
///
/// This constant defines the size of the guard region that is placed at the
/// bottom of each thread's stack to detect and prevent stack overflow. The guard
/// region is a protected memory area that causes a fault when accessed, providing
/// early detection of stack overflow conditions.
///
/// # Value
///
/// The guard size is set to 0x4000 (16,384 bytes or 16 KiB), which is:
/// - 4 memory pages (assuming 4 KiB page size)
/// - Standard size used by _libnx_ and the Horizon OS kernel
/// - Sufficient to catch most stack overflow scenarios
///
/// # Purpose
///
/// The guard region serves as a safety mechanism by:
/// - Creating an inaccessible memory region below the stack
/// - Triggering a memory access fault if the stack grows beyond its limit
/// - Preventing stack overflow from corrupting adjacent memory regions
///
/// # Compatibility
///
/// This value matches the implementation in libnx (`libnx/source/kernel/thread.c`)
/// to ensure compatibility with existing Horizon OS applications and libraries.
const GUARD_SIZE: usize = 0x4000;

/// Map the [`UnmappedStackMemory`] instance into the current process.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn map<B>(buffer: B) -> Result<MappedStackMemory<B>, MapError>
where
    B: Buf,
{
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
pub unsafe fn unmap<B>(sm: MappedStackMemory<B>) -> Result<B, UnmapError>
where
    B: Buf,
{
    let MappedStackMemory {
        buffer,
        mapped_mem_ptr,
    } = sm;

    // Ensure the memory is properly unmapped from the process address space.
    svc::unmap_memory(mapped_mem_ptr, buffer.ptr(), buffer.size()).map_err(UnmapError::Svc)?;

    Ok(buffer)
}

/// Errors that can occur when unmapping stack memory.
#[derive(Debug, thiserror::Error)]
pub enum UnmapError {
    /// System call to unmap memory failed.
    #[error(transparent)]
    Svc(#[from] svc::UnmapMemoryError),
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
pub struct MappedStackMemory<B> {
    buffer: B,
    mapped_mem_ptr: NonNull<c_void>,
}

impl<B> MappedStackMemory<B> {
    /// Creates a new `MappedStackMemory` instance from the provided buffer and
    /// mapped memory pointer.
    ///
    /// # Safety
    /// This function is unsafe because it assumes that the `buffer` is valid and
    /// the `mapped_mem_ptr` points to a valid memory region that has been
    /// mapped into the process address space.
    pub unsafe fn from_raw_parts(buffer: B, mapped_mem_ptr: NonNull<c_void>) -> Self {
        MappedStackMemory {
            buffer,
            mapped_mem_ptr,
        }
    }
}

impl<B> MappedStackMemory<B>
where
    B: Buf,
{
    /// Returns the underlying memory buffer pointer.
    pub fn buffer_ptr(&self) -> NonNull<c_void> {
        self.buffer.ptr()
    }

    /// Returns the size of the maemory buffer.
    pub fn size(&self) -> usize {
        self.buffer.size()
    }

    /// Returns the pointer to the mapped memory.
    pub fn mapped_mem_ptr(&self) -> NonNull<c_void> {
        self.mapped_mem_ptr
    }
}
