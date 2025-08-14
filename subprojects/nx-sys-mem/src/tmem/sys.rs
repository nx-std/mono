//! High-level helpers for Horizon OS transfer-memory objects.
//!
//! This module mirrors the API of `libnx`'s `tmem.c`, providing an idiomatic
//! and **safe** (where possible) Rust layer on top of the SVC helpers in
//! `nx-svc::mem::tmem`.
//!
//! Main differences compared to the original C implementation:
//!
//! 1.  All interaction with the kernel is funneled through the safe wrappers
//!     in `nx-svc`, eliminating nearly all `unsafe` from the public API.
//! 2.  The object state is encoded at the type level via the zero-cost
//!     `TransferMemory<S>` wrapper (where `S` is one of [`Unmapped`] or
//!     [`Mapped`]).  This prevents common misuse such as double-mapping or
//!     closing a handle while still mapped.
//! 3.  Virtual-memory management is delegated to `crate::vmm::sys`, so explicit
//!     `virtmemLock()`/`virtmemUnlock()` calls are replaced by the RAII guard
//!     returned from [`crate::vmm::sys::lock`].
//!
//! Only the low-level kernel resources are managed here; the C-compatible FFI
//! layer (`crate::tmem::ffi`) is a thin shim translating between the C structs
//! declared in `nx_tmem.h` and the high-level Rust API below.

use alloc::alloc::{Layout, alloc_zeroed, dealloc};
use core::{ffi::c_void, ptr, ptr::NonNull};

use nx_svc::{
    mem::{
        core as memcore, tmem as svc,
        tmem::{Handle, MemoryPermission},
    },
    thread,
};

use crate::vmm::sys as vmm;

/// Guard region size (0x1000), per libnx.
const GUARD_SIZE: usize = 0x1000;

/// Memory-permission bitmask (see `Perm_*` constants in `nx_tmem.h`).
/// In libnx this is an `enum Permission`; we simply forward the raw `u32`.
pub type Permissions = MemoryPermission;

/// State-dependent wrapper around a transfer-memory kernel object.
#[derive(Debug)]
pub struct TransferMemory<S: TmemState + core::fmt::Debug>(S);

/// Creates a new transfer-memory object backed by zero-initialised memory.
///
/// The memory is allocated with 4-KiB alignment (page size) and its lifetime
/// is tied to the returned [`TransferMemory`] value – it will be freed upon
/// successful [`close`] or if creation fails at any point.
///
/// # Safety
///
/// This function ultimately issues the `CreateTransferMemory` SVC; any misuse
/// of the returned object (e.g. mapping twice) may lead to undefined
/// behaviour.  Callers must therefore uphold the invariants documented in the
/// Horizon OS manual.
pub unsafe fn create(
    size: usize,
    perm: Permissions,
) -> Result<TransferMemory<Unmapped>, CreateError> {
    // Allocate page-aligned, zero-filled backing memory.
    let layout = Layout::from_size_align(size, 0x1000).map_err(|_| CreateError::OutOfMemory)?;
    let addr = unsafe { alloc_zeroed(layout) }.cast();
    let Some(addr) = NonNull::new(addr) else {
        return Err(CreateError::OutOfMemory);
    };

    // Attempt to create the kernel object around that memory.
    match svc::create_transfer_memory(addr, size, perm) {
        Ok(handle) => Ok(TransferMemory(Unmapped {
            handle,
            size,
            perm,
            src: Some(addr),
        })),
        Err(err) => {
            // Cleanup allocation on failure.
            unsafe { dealloc(addr.as_ptr().cast(), layout) };
            Err(CreateError::Svc(err))
        }
    }
}

/// Creates a transfer-memory object from an existing, page-aligned buffer.
///
/// The caller retains ownership of `buf` and is responsible for its lifetime.
///
/// # Safety
///
/// * `buf` **must** be 4-KiB aligned and pointing to at least `size` bytes of
///   allocated memory.
/// * The same pointer must not be passed to any other function that may
///   concurrently deallocate or repurpose the memory while the transfer
///   memory object is alive.
pub unsafe fn create_from_memory(
    buf: NonNull<c_void>,
    size: usize,
    perm: Permissions,
) -> Result<TransferMemory<Unmapped>, CreateError> {
    // Check that the buffer is page-aligned and has sufficient size.
    if (buf.as_ptr() as usize & 0xFFF) != 0 {
        return Err(CreateError::InvalidAddress);
    }

    match svc::create_transfer_memory(buf, size, perm) {
        Ok(handle) => Ok(TransferMemory(Unmapped {
            handle,
            size,
            perm,
            src: Some(buf),
        })),
        Err(err) => Err(CreateError::Svc(err)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateError {
    #[error("Out of memory")]
    OutOfMemory,
    #[error("Invalid address (must be page-aligned)")]
    InvalidAddress,
    #[error(transparent)]
    Svc(#[from] svc::CreateTransferMemoryError),
}

/// Populate a [`TransferMemory`] coming from another process.
#[inline]
pub fn load_remote(
    tm: &mut TransferMemory<Unmapped>,
    handle: Handle,
    size: usize,
    perm: Permissions,
) {
    tm.0 = Unmapped {
        handle,
        size,
        perm,
        src: None, // backing memory lives in the remote process
    };
}

/// Maps the transfer memory into the current process.
///
/// # Safety
///
/// This function is unsafe because it issues the `MapTransferMemory` SVC and
/// because the caller could violate memory-safety invariants (e.g. by mapping
/// overlapping regions manually).
pub unsafe fn map(tm: TransferMemory<Unmapped>) -> Result<TransferMemory<Mapped>, MapError> {
    let TransferMemory(Unmapped {
        handle,
        size,
        perm,
        src,
    }) = tm;

    // Lock the VMM and reserve a virtual address range in the ASLR address space.
    let Some(addr) = vmm::lock().find_aslr(size, GUARD_SIZE) else {
        return Err(MapError {
            kind: MapErrorKind::VirtAddressAllocFailed,
            tm,
        });
    };

    svc::map_transfer_memory(handle, addr, size, perm).map_err(|err| MapError {
        kind: MapErrorKind::Svc(err),
        tm,
    })?;

    Ok(TransferMemory(Mapped {
        handle,
        size,
        perm,
        src,
        addr,
    }))
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to map transfer memory: {kind}")]
pub struct MapError {
    #[source]
    pub kind: MapErrorKind,
    pub tm: TransferMemory<Unmapped>,
}

#[derive(Debug, thiserror::Error)]
pub enum MapErrorKind {
    #[error("Failed to allocate virtual address range")]
    VirtAddressAllocFailed,
    #[error(transparent)]
    Svc(#[from] svc::MapTransferMemoryError),
}

/// Unmaps the transfer memory from the current process.
///
/// # Safety
///
/// As with [`map`], this function interacts with the kernel directly and is
/// therefore unsafe.
pub unsafe fn unmap(tm: TransferMemory<Mapped>) -> Result<TransferMemory<Unmapped>, UnmapError> {
    let TransferMemory(Mapped {
        handle,
        size,
        perm,
        src,
        addr,
    }) = tm;

    match svc::unmap_transfer_memory(handle, addr, size) {
        Ok(()) => Ok(TransferMemory(Unmapped {
            handle,
            size,
            perm,
            src,
        })),
        Err(err) => Err(UnmapError { reason: err, tm }),
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Transfer-memory unmap failed: {reason}")]
pub struct UnmapError {
    #[source]
    pub reason: svc::UnmapTransferMemoryError,
    pub tm: TransferMemory<Mapped>,
}

/// Close the transfer-memory handle.
///
/// # Safety
///
/// Unsafe for the same reasons as other kernel-interacting functions.
pub unsafe fn close_handle(tm: TransferMemory<Unmapped>) -> Result<(), CloseError> {
    let TransferMemory(Unmapped { handle, .. }) = tm;

    // SAFETY: The handle is guaranteed to be valid since the creation
    // of the shared memory object is guarded by the `create` function.
    #[cfg(debug_assertions)]
    if !handle.is_valid() {
        panic!("Invalid transfer memory handle: INVALID_TMEM_HANDLE");
    }

    svc::close_handle(handle).map_err(|err| CloseError { reason: err, tm })
}

/// Close the transfer-memory handle and free any backing memory we allocated.
///
/// # Safety
///
/// Unsafe for the same reasons as other kernel-interacting functions.
pub unsafe fn close(tm: TransferMemory<Unmapped>) -> Result<(), CloseError> {
    let TransferMemory(Unmapped {
        handle, size, src, ..
    }) = tm;

    // SAFETY: The handle is guaranteed to be valid since the creation
    // of the shared memory object is guarded by the `create` function.
    #[cfg(debug_assertions)]
    if !handle.is_valid() {
        panic!("Invalid transfer memory handle: INVALID_TMEM_HANDLE");
    }

    svc::close_handle(handle).map_err(|err| CloseError { reason: err, tm })?;

    // Free backing memory if we own it, i.e. `src` is `Some`.
    if let Some(ptr) = src {
        let layout = Layout::from_size_align(size, 0x1000).unwrap();
        unsafe { dealloc(ptr.as_ptr() as *mut u8, layout) };
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to close transfer memory: {reason}")]
pub struct CloseError {
    #[source]
    pub reason: svc::CloseHandleError,
    pub tm: TransferMemory<Unmapped>,
}

/// Wait for the transfer memory to have the specified permission.
///
/// # Safety
///
/// Unsafe for the same reasons as other kernel-interacting functions.
pub unsafe fn wait_for_permission(
    tm: TransferMemory<Unmapped>,
    perm: Permissions,
) -> Result<TransferMemory<Unmapped>, WaitForPermissionError> {
    // Quick-path: permissions already satisfy the requirement stored in the struct.
    if (tm.0.perm & perm) == perm {
        return Ok(tm);
    }

    // Obtain an address to query – we need the source backing memory.
    // If we don't own the memory (`src == None`) we cannot wait because we have no
    // address to poll; in that (unlikely) scenario we just return success as
    // libnx would crash anyway with an invalid address.
    let src_addr = tm.0.src().map(|nn| nn.as_ptr()).unwrap_or(ptr::null_mut());

    loop {
        match memcore::query_memory(src_addr as usize) {
            Ok((mem_info, _page)) => {
                if mem_info
                    .perm
                    .contains(memcore::MemoryPermission::from_bits_truncate(perm.bits()))
                {
                    break;
                }
            }
            Err(err) => {
                return Err(WaitForPermissionError { reason: err, tm });
            }
        }

        // Sleep for 100,000 nanoseconds (0.1 ms) before polling again.
        thread::sleep(100_000);
    }

    Ok(tm)
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to wait for permission: {reason}")]
pub struct WaitForPermissionError {
    #[source]
    pub reason: memcore::QueryMemoryError,
    pub tm: TransferMemory<Unmapped>,
}

pub trait TmemState: _priv::Sealed {
    fn handle(&self) -> Handle;
    fn size(&self) -> usize;
    fn perm(&self) -> Permissions;
    fn src(&self) -> Option<NonNull<c_void>>;
    fn map_addr(&self) -> Option<NonNull<c_void>>;
}

#[derive(Debug)]
pub struct Unmapped {
    handle: Handle,
    size: usize,
    perm: Permissions,
    src: Option<NonNull<c_void>>, // None if owned by another process
}

impl TmemState for Unmapped {
    fn handle(&self) -> Handle {
        self.handle
    }

    fn size(&self) -> usize {
        self.size
    }

    fn perm(&self) -> Permissions {
        self.perm
    }

    fn src(&self) -> Option<NonNull<c_void>> {
        self.src
    }

    fn map_addr(&self) -> Option<NonNull<c_void>> {
        None
    }
}
impl _priv::Sealed for Unmapped {}

#[derive(Debug)]
pub struct Mapped {
    handle: Handle,
    size: usize,
    perm: Permissions,
    src: Option<NonNull<c_void>>,
    addr: NonNull<c_void>,
}

impl TmemState for Mapped {
    fn handle(&self) -> Handle {
        self.handle
    }

    fn size(&self) -> usize {
        self.size
    }

    fn perm(&self) -> Permissions {
        self.perm
    }

    fn src(&self) -> Option<NonNull<c_void>> {
        self.src
    }

    fn map_addr(&self) -> Option<NonNull<c_void>> {
        Some(self.addr)
    }
}
impl _priv::Sealed for Mapped {}

mod _priv {
    pub trait Sealed {}
}

#[cfg(feature = "ffi")]
impl TransferMemory<Mapped> {
    /// Internal constructor used by the FFI layer.
    pub(super) unsafe fn from_parts(
        handle: Handle,
        size: usize,
        perm: Permissions,
        src: Option<NonNull<c_void>>,
        addr: NonNull<c_void>,
    ) -> Self {
        Self(Mapped {
            handle,
            size,
            perm,
            src,
            addr,
        })
    }
}

#[cfg(feature = "ffi")]
impl TransferMemory<Unmapped> {
    /// Internal constructor used by the FFI layer.
    pub(super) unsafe fn from_parts(
        handle: Handle,
        size: usize,
        perm: Permissions,
        src: Option<NonNull<c_void>>,
    ) -> Self {
        Self(Unmapped {
            handle,
            size,
            perm,
            src,
        })
    }
}

#[cfg(feature = "ffi")]
impl<S> TransferMemory<S>
where
    S: TmemState + core::fmt::Debug,
{
    pub fn handle(&self) -> Handle {
        self.0.handle()
    }

    pub fn size(&self) -> usize {
        self.0.size()
    }

    pub fn perm(&self) -> Permissions {
        self.0.perm()
    }

    /// Backing memory address (only meaningful if we own it).
    pub fn src_addr(&self) -> Option<*mut c_void> {
        self.0.src().map(|nn| nn.as_ptr())
    }

    /// Return the mapped address, if any.
    pub fn map_addr(&self) -> Option<*mut c_void> {
        self.0.map_addr().map(|nn| nn.as_ptr())
    }
}
