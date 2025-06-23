//! High-level helpers for Horizon OS stack memory management.

use alloc::alloc::{Layout, alloc_zeroed, dealloc};
use core::{ffi::c_void, ptr::NonNull};

use nx_svc::{
    debug::{BreakReason, break_event},
    mem::core as svc,
};

use crate::vmm::sys as vmm;

/// Guard size 0x4000, per libnx
///
/// See `libnx/source/kernel/thread.c` for more details.
const GUARD_SIZE: usize = 0x4000;

const PAGE_SIZE: usize = 0x1000;

/// Stack memory management object
#[derive(Debug)]
pub struct StackMemory<S: StackState + core::fmt::Debug>(S);

impl<S> StackMemory<S>
where
    S: StackState + core::fmt::Debug,
{
    /// Allocate a new, page-aligned stack buffer of `size` bytes and wrap it
    /// in an `Unmapped` `StackMemory` that **owns** the allocation.
    ///
    /// Returns `None` if the allocation fails or if `size` is not page-aligned.
    pub fn alloc_owned(size: usize) -> Result<StackMemory<Unmapped>, AllocError> {
        // Validate size (non-zero, page aligned).
        if size == 0 || size & (PAGE_SIZE - 1) != 0 {
            return Err(AllocError::InvalidSize);
        }

        // SAFETY: `Layout` guarantees non-zero size and valid alignment.
        let layout =
            Layout::from_size_align(size, PAGE_SIZE).map_err(|_| AllocError::InvalidSize)?;
        let ptr = unsafe { alloc_zeroed(layout) } as *mut c_void;
        let Some(backing) = NonNull::new(ptr) else {
            return Err(AllocError::AllocFailed);
        };

        Ok(StackMemory(Unmapped {
            backing,
            size,
            owned: true,
        }))
    }

    /// Wrap an **existing** stack buffer into an `Unmapped` `StackMemory` that
    /// does **not** own the memory. The caller is responsible for ensuring the
    /// pointer remains valid for the lifetime of the returned object.
    ///
    /// # Safety
    /// The caller must guarantee that:
    /// - `backing` points to a region of memory of at least `size` bytes,
    ///   page-aligned (4 KiB), and that the memory outlives the returned
    ///   `StackMemory`.
    /// - `size` is a multiple of the 4 KiB page size expected by the kernel.
    /// - The caller is responsible for ensuring the memory outlives the returned
    ///   `StackMemory`.
    pub unsafe fn from_raw(
        backing: *mut c_void,
        size: usize,
    ) -> Result<StackMemory<Unmapped>, AllocError> {
        if size == 0 || size & (PAGE_SIZE - 1) != 0 {
            return Err(AllocError::InvalidSize);
        }

        let Some(backing) = NonNull::new(backing) else {
            return Err(AllocError::NullPtr);
        };

        Ok(StackMemory(Unmapped {
            backing,
            size,
            owned: false,
        }))
    }
}

impl<S> StackMemory<S>
where
    S: StackState + core::fmt::Debug,
{
    /// Returns the size of the stack backing buffer.
    pub fn size(&self) -> usize {
        self.0.size()
    }

    /// Returns `true` if the stack backing buffer is owned by the [`StackMemory`]
    /// instance, `false` otherwise.
    pub fn is_owned(&self) -> bool {
        self.0.is_owned()
    }

    /// Returns the physical backing pointer of the stack pages.
    ///
    /// This is the pointer originally allocated (or provided by the caller)
    /// before being mapped into the process address space.
    pub fn backing_ptr(&self) -> NonNull<c_void> {
        self.0.backing_ptr()
    }
}

impl StackMemory<Mapped> {
    /// Returns the virtual address where this stack is mapped, if any.
    pub fn addr(&self) -> NonNull<c_void> {
        let StackMemory(Mapped { addr, .. }) = self;
        *addr
    }
}

impl<S> Drop for StackMemory<S>
where
    S: StackState + core::fmt::Debug,
{
    /// Automatically handles resource-cleanup when a `StackMemory` handle goes
    /// out of scope.
    ///
    /// The implementation follows the rules below:
    ///
    /// • **Not owned** – If `self.is_owned()` returns `false`, the function is a
    ///   no-op: responsibility for the backing buffer lies elsewhere.
    ///
    /// • **Owned _and_ still mapped** – Releasing pages that are still mapped
    ///   would violate the kernel contract. This condition signals a logic error
    ///   in the caller (they forgot to invoke [`unmap`]).
    ///   – In release builds the drop silently aborts early, leaking the pages so
    ///     the process can keep running.
    ///   – In debug builds we trigger a debugger break via
    ///     [`nx_svc::debug::break_event`] with [`BreakReason::Panic`].
    ///
    /// • **Owned _and_ unmapped** – This is the happy path: the backing pages were
    ///   allocated by [`alloc_owned`], are no longer mapped, and can be returned
    ///   to the allocator using the exact same layout (page-aligned, same size).
    ///
    /// The method uses `unsafe` code internally because it performs a raw
    /// `dealloc`; the safety invariants are upheld by the surrounding checks.
    fn drop(&mut self) {
        // If we don't own the backing buffer, do nothing.
        if !self.0.is_owned() {
            return;
        }

        // It is responsibiloty of the caller to unmap the buffer before dropping.
        // If the buffer is mapped, panic in debug mode, otherwise do nothing.
        if self.0.is_mapped() {
            #[cfg(not(debug_assertions))]
            {
                return;
            }
            #[cfg(debug_assertions)]
            {
                // TODO: Panic with a more descriptive error message.
                break_event(BreakReason::Panic, 0, 0);
            }
        }

        // If we own the backing buffer and it is *not* mapped anymore,
        // free the pages.
        //
        // SAFETY: `backing` was allocated with `alloc_owned` using the same
        // layout (size, 4 KiB alignment), so freeing with that layout is
        // correct.
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.0.size(), PAGE_SIZE);
            dealloc(self.0.backing_ptr().as_ptr() as *mut u8, layout);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AllocError {
    /// Size must be non-zero and a multiple of the 4 KiB page size.
    #[error("Invalid size: must be page-aligned (0x1000) and non-zero")]
    InvalidSize,
    /// Provided backing pointer was null.
    #[error("Backing pointer is null")]
    NullPtr,
    /// Memory allocation failed.
    #[error("Memory allocation failed")]
    AllocFailed,
}

/// Map the [`StackMemory`] instance into the current process.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn map(sm: StackMemory<Unmapped>) -> Result<StackMemory<Mapped>, MapError> {
    let StackMemory(Unmapped {
        backing,
        size,
        owned,
    }) = sm;

    let mut vmm = vmm::lock();

    // Ask the VMM for a free slice of stack address-space.
    let Some(addr) = vmm.find_stack(size, GUARD_SIZE) else {
        return Err(MapError::VirtAddressAllocFailed);
    };

    // Attempt to map the shared memory into that slice.
    match svc::map_memory(addr.as_ptr(), backing.as_ptr(), size) {
        Ok(()) => Ok(StackMemory(Mapped {
            backing,
            size,
            owned,
            addr,
        })),
        Err(err) => Err(MapError::Svc(err)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    /// Failed to allocate a virtual address range
    #[error("Failed to allocate virtual address range")]
    VirtAddressAllocFailed,
    #[error(transparent)]
    Svc(#[from] svc::MapMemoryError),
}

/// Unmap the shared-memory object from the current process.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn unmap(sm: StackMemory<Mapped>) -> Result<StackMemory<Unmapped>, UnmapError> {
    let StackMemory(Mapped {
        backing,
        size,
        owned,
        addr,
    }) = sm;

    match svc::unmap_memory(addr.as_ptr(), backing.as_ptr(), size) {
        Ok(()) => Ok(StackMemory(Unmapped {
            backing,
            size,
            owned,
        })),
        Err(err) => Err(UnmapError { reason: err, sm }),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Shared memory unmap failed: {reason}")]
pub struct UnmapError {
    /// The error returned by the kernel
    #[source]
    pub reason: svc::UnmapMemoryError,
    /// The shared memory object that was unmapped
    pub sm: StackMemory<Mapped>,
}

pub trait StackState: _priv::Sealed {
    fn backing_ptr(&self) -> NonNull<c_void>;
    fn size(&self) -> usize;
    fn is_owned(&self) -> bool;
    fn is_mapped(&self) -> bool;
    fn get_addr(&self) -> Option<NonNull<c_void>>;
}

#[derive(Debug)]
pub struct Unmapped {
    backing: NonNull<c_void>,
    size: usize,
    owned: bool,
}

impl StackState for Unmapped {
    fn backing_ptr(&self) -> NonNull<c_void> {
        self.backing
    }

    fn size(&self) -> usize {
        self.size
    }

    fn is_owned(&self) -> bool {
        self.owned
    }

    fn is_mapped(&self) -> bool {
        false
    }

    fn get_addr(&self) -> Option<NonNull<c_void>> {
        None
    }
}

impl _priv::Sealed for Unmapped {}

#[derive(Debug)]
pub struct Mapped {
    backing: NonNull<c_void>,
    size: usize,
    owned: bool,
    addr: NonNull<c_void>,
}

impl StackState for Mapped {
    fn backing_ptr(&self) -> NonNull<c_void> {
        self.backing
    }

    fn size(&self) -> usize {
        self.size
    }

    fn is_owned(&self) -> bool {
        self.owned
    }

    fn is_mapped(&self) -> bool {
        true
    }

    fn get_addr(&self) -> Option<NonNull<c_void>> {
        Some(self.addr)
    }
}

impl _priv::Sealed for Mapped {}

#[allow(unused)]
mod _priv {
    pub trait Sealed {}
}
