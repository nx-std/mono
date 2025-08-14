//! High-level helpers for Horizon OS shared-memory objects.
//!
//! This module provides a safe and idiomatic wrapper around the kernel SVCs
//! responsible for creating, mapping and destroying shared-memory kernel
//! objects.  The C-compatible API declared in `nx_shmem.h` is implemented in
//! [`crate::shmem::ffi`]; that thin FFI layer delegates all the heavy
//! lifting to the routines defined here.
//!
//! Key differences from the original C implementation that ships with
//! libnx (`libnx/nx/source/kernel/shmem.c`):
//!
//! 1.  All kernel calls are routed through the high-level wrappers in
//!     [`nx_svc::mem::shmem`], drastically reducing the amount of `unsafe`
//!     scattered throughout the code-base.
//! 2.  Virtual-address allocation is handled by the cross-crate virtual memory
//!     manager [`vmm`](crate::vmm). The classic `virtmemLock()` /
//!     `virtmemUnlock()` pair is therefore expressed with the RAII guard
//!     returned by [`vmm::lock`](crate::vmm::sys::lock).
//! 3.  Instead of a single mutable C struct, the state of a shared-memory
//!     object is encoded in the type system via the zero-cost phantom-type
//!     wrapper [`SharedMemory<S>`], where the type parameter captures whether
//!     the segment is currently [`Unmapped`] or [`Mapped`].  This makes many
//!     classes of misuse (double-mapping, premature unmapping, etc.)
//!     impossible to represent in safe Rust.

use core::{ffi::c_void, ptr::NonNull};

use nx_svc::mem::shmem::{
    self as svc, Handle, LocalShmemPermission, MemoryPermission, RemoteShmemPermission,
};

use crate::vmm::sys as vmm;

/// Size of the guard region for shared memory mappings (4 KiB).
///
/// This constant defines the size of the guard region that is placed around
/// shared memory mappings to prevent accidental access beyond the mapped region.
/// The guard size matches the implementation in libnx to ensure compatibility.
///
/// # Value
///
/// The guard size is set to 0x1000 (4,096 bytes or 4 KiB), which is:
/// - 1 memory page (assuming 4 KiB page size)
/// - Standard size used by libnx (`libnx/nx/source/kernel/shmem.c`)
/// - Sufficient to catch most buffer overflow scenarios
const GUARD_SIZE: usize = 0x1000;

/// Memory-permission bitmask for shared memory operations.
///
/// Defines the access permissions (read/write/execute) for shared memory regions.
/// This is a re-export of the kernel's `MemoryPermission` type.
pub type Permissions = MemoryPermission;

/// Memory permissions for the local process when creating shared memory.
///
/// Specifies what operations the creating process can perform on the shared memory.
/// This is a re-export of the kernel's `LocalShmemPermission` type.
pub type LocalPermissions = LocalShmemPermission;

/// Memory permissions for remote processes when accessing shared memory.
///
/// Specifies what operations other processes can perform when the shared memory
/// is transferred to them. This is a re-export of the kernel's `RemoteShmemPermission` type.
pub type RemotePermissions = RemoteShmemPermission;

/// Shared memory kernel object
///
/// The `handle` field is guaranteed to be valid since the creation of the
/// shared memory object until it is successfully closed.
#[derive(Debug, Clone)]
pub struct SharedMemory<S: ShmState + core::fmt::Debug>(S);

/// Create a new shared-memory object owned by the calling process.
///
/// The shared memory `handle` field is guaranteed to be valid since the
/// creation of the shared memory object until it is successfully closed.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn create(
    size: usize,
    local_perm: LocalPermissions,
    remote_perm: RemotePermissions,
) -> Result<SharedMemory<Unmapped>, CreateError> {
    match svc::create_shared_memory(size, local_perm, remote_perm) {
        Ok(handle) => Ok(SharedMemory(Unmapped {
            handle,
            size,
            perm: Permissions::from_bits_truncate(local_perm.bits()),
        })),
        Err(err) => Err(CreateError(err)),
    }
}

/// Populate a `SharedMemory` struct coming from a remote process.
///
/// Creates a [`SharedMemory`] object from a handle that was received from another process.
/// This is used when shared memory is transferred between processes via IPC.
///
/// # Arguments
///
/// * `handle` - The kernel handle for the shared memory object received from another process
/// * `size` - The size of the shared memory region in bytes
/// * `perm` - The memory permissions to use when mapping this shared memory
///
/// # Returns
///
/// A [`SharedMemory<Unmapped>`] object that can be mapped into the current process's
/// address space using the [`map`] function.
#[inline]
pub fn load_remote(handle: Handle, size: usize, perm: Permissions) -> SharedMemory<Unmapped> {
    SharedMemory(Unmapped { handle, size, perm })
}

/// Map the [`SharedMemory`] instance into the current process.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn map(shm: SharedMemory<Unmapped>) -> Result<SharedMemory<Mapped>, MapError> {
    let SharedMemory(Unmapped { handle, size, perm }) = shm;

    // Ask the VMM for a free slice of ASLR address-space.
    let Some(addr) = vmm::lock().find_aslr(size, GUARD_SIZE) else {
        return Err(MapError::VirtAddressAllocFailed);
    };

    // Attempt to map the shared memory into that slice.
    svc::map_shared_memory(handle, addr, size, perm).map_err(MapError::Svc)?;

    Ok(SharedMemory(Mapped {
        handle,
        size,
        perm,
        mapped_mem_ptr: addr,
    }))
}

/// Unmap the shared-memory object from the current process.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn unmap(shm: SharedMemory<Mapped>) -> Result<SharedMemory<Unmapped>, UnmapError> {
    let SharedMemory(Mapped {
        handle,
        size,
        perm,
        mapped_mem_ptr: addr,
    }) = shm;

    svc::unmap_shared_memory(handle, addr, size).map_err(|reason| UnmapError { reason, shm })?;

    Ok(SharedMemory(Unmapped { handle, size, perm }))
}

/// Close the shared-memory object, freeing kernel resources.
///
/// The shared memory `handle` field is guaranteed to be valid since the
/// creation of the shared memory object until it is successfully closed.
///
/// # Safety
///
/// This function is unsafe because it interacts with the kernel directly,
/// which is inherently unsafe.
pub unsafe fn close(shm: SharedMemory<Unmapped>) -> Result<(), CloseError> {
    let SharedMemory(Unmapped { handle, .. }) = shm;

    svc::close_handle(handle).map_err(|err| CloseError { reason: err, shm })
}

/// Error that occurs when creating a shared memory object fails.
///
/// This error wraps the underlying kernel error from the `svcCreateSharedMemory` system call.
/// Common causes include:
/// - Invalid size (e.g., not page-aligned)
/// - Invalid permission flags
/// - Insufficient system resources
/// - Kernel quota exceeded
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CreateError(#[from] svc::CreateSharedMemoryError);

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    /// Failed to allocate a virtual address range.
    ///
    /// This error occurs when the virtual memory manager cannot find a suitable
    /// contiguous region in the process's ASLR address space to map the shared memory.
    /// This typically happens when:
    /// - The process address space is fragmented
    /// - The requested size is too large for available regions
    /// - The virtual memory manager is unable to reserve the required space
    #[error("Failed to allocate virtual address range")]
    VirtAddressAllocFailed,

    /// System call to map shared memory failed.
    ///
    /// This error wraps the underlying kernel error from the `svcMapSharedMemory` system call.
    /// Common causes include:
    /// - Invalid handle
    /// - Invalid address alignment
    /// - Permission denied
    /// - Address range already in use
    #[error(transparent)]
    Svc(#[from] svc::MapSharedMemoryError),
}

/// Error that occurs when unmapping shared memory fails.
///
/// This error contains both the underlying kernel error and the shared memory
/// object that failed to unmap. The shared memory object is preserved so that
/// the caller can attempt recovery or proper cleanup.
///
/// # Common Causes
///
/// - Invalid handle (shared memory was already closed)
/// - Invalid address (memory was not mapped at the specified address)
/// - Permission denied (process lacks permission to unmap)
/// - Memory region is still in use by another component
#[derive(Debug, thiserror::Error)]
#[error("Shared memory unmap failed: {reason}")]
pub struct UnmapError {
    /// The error returned by the kernel from the `svcUnmapSharedMemory` system call.
    #[source]
    pub reason: svc::UnmapSharedMemoryError,
    /// The shared memory object that failed to unmap.
    ///
    /// This is returned so the caller can attempt recovery or cleanup.
    /// The shared memory remains in the [`Mapped`] state.
    pub shm: SharedMemory<Mapped>,
}

/// Error that occurs when closing a shared memory object fails.
///
/// This error contains both the underlying kernel error and the shared memory
/// object that failed to close. The shared memory object is preserved so that
/// the caller can attempt recovery or proper cleanup.
///
/// # Common Causes
///
/// - Invalid handle (handle was already closed or never valid)
/// - Handle type mismatch (handle is not a shared memory handle)
/// - Kernel resource management issues
#[derive(Debug, thiserror::Error)]
#[error("Failed to close shared memory: {reason}")]
pub struct CloseError {
    /// The error returned by the kernel from the `svcCloseHandle` system call.
    #[source]
    pub reason: svc::CloseHandleError,
    /// The shared memory object that failed to close.
    ///
    /// This is returned so the caller can attempt recovery or cleanup.
    /// The shared memory remains in the [`Unmapped`] state.
    pub shm: SharedMemory<Unmapped>,
}

/// Trait representing the state of a shared memory object.
///
/// This trait is sealed and cannot be implemented outside this module.
/// It provides a common interface for accessing shared memory properties
/// regardless of whether the memory is currently mapped or unmapped.
///
/// The two implementations are:
/// - [`Unmapped`]: Shared memory that exists but is not mapped to any address
/// - [`Mapped`]: Shared memory that is mapped to a specific virtual address
pub trait ShmState: _priv::Sealed {
    /// Returns the kernel handle for the shared memory object.
    fn handle(&self) -> Handle;

    /// Returns the size of the shared memory region in bytes.
    fn size(&self) -> usize;

    /// Returns the memory permissions for the shared memory.
    fn perm(&self) -> Permissions;

    /// Returns `true` if the shared memory is currently mapped to an address.
    fn is_mapped(&self) -> bool;

    /// Returns the mapped address if the memory is currently mapped, or `None` if unmapped.
    fn get_addr(&self) -> Option<*mut c_void>;
}

/// State representing a shared memory object that is not mapped to any address.
///
/// This state indicates that the shared memory object exists in the kernel
/// but has not been mapped into the process's address space. The memory
/// cannot be accessed until it is mapped using the [`map`] function.
#[derive(Debug)]
pub struct Unmapped {
    /// The kernel handle for the shared memory object
    handle: Handle,
    /// The size of the shared memory region in bytes
    size: usize,
    /// The memory permissions that will be used when mapping
    perm: Permissions,
}

impl ShmState for Unmapped {
    fn handle(&self) -> Handle {
        self.handle
    }

    fn size(&self) -> usize {
        self.size
    }

    fn perm(&self) -> Permissions {
        self.perm
    }

    fn is_mapped(&self) -> bool {
        false
    }

    fn get_addr(&self) -> Option<*mut c_void> {
        None
    }
}

impl _priv::Sealed for Unmapped {}

/// State representing a shared memory object that is mapped to a virtual address.
///
/// This state indicates that the shared memory has been successfully mapped
/// into the process's address space and can be accessed through the mapped
/// address. The memory remains accessible until it is unmapped using the
/// [`unmap`] function.
#[derive(Debug)]
pub struct Mapped {
    /// The kernel handle for the shared memory object
    handle: Handle,
    /// The size of the mapped memory region in bytes
    size: usize,
    /// The memory permissions applied to the mapping
    perm: Permissions,
    /// The virtual address where the memory is mapped
    ///
    /// This address is guaranteed to be non-null and valid as long as the memory is mapped.
    mapped_mem_ptr: NonNull<c_void>,
}

impl ShmState for Mapped {
    fn handle(&self) -> Handle {
        self.handle
    }

    fn size(&self) -> usize {
        self.size
    }

    fn perm(&self) -> Permissions {
        self.perm
    }

    fn is_mapped(&self) -> bool {
        true
    }

    fn get_addr(&self) -> Option<*mut c_void> {
        Some(self.mapped_mem_ptr.as_ptr())
    }
}

impl _priv::Sealed for Mapped {}

#[cfg(feature = "ffi")]
impl SharedMemory<Mapped> {
    /// Construct a `Mapped` shared-memory object from its constituent parts.
    ///
    /// Internal constructor used by the FFI layer.
    pub(super) unsafe fn from_parts(
        handle: Handle,
        size: usize,
        perm: Permissions,
        addr: NonNull<c_void>,
    ) -> Self {
        Self(Mapped {
            handle,
            size,
            perm,
            mapped_mem_ptr: addr,
        })
    }
}

#[cfg(feature = "ffi")]
impl SharedMemory<Unmapped> {
    /// Construct a `Unmapped` shared-memory object from its constituent parts.
    ///
    /// Internal constructor used by the FFI layer.
    pub(super) unsafe fn from_parts(handle: Handle, size: usize, perm: Permissions) -> Self {
        Self(Unmapped { handle, size, perm })
    }
}

#[cfg(feature = "ffi")]
impl<S> SharedMemory<S>
where
    S: ShmState + core::fmt::Debug,
{
    /// Kernel handle backing the shared-memory object.
    pub fn handle(&self) -> Handle {
        self.0.handle()
    }

    /// Size (in bytes) of the shared-memory object.
    pub fn size(&self) -> usize {
        self.0.size()
    }

    /// Memory permissions requested when mapping locally.
    pub fn perm(&self) -> Permissions {
        self.0.perm()
    }

    /// Returns the mapped address, if the segment is currently mapped.
    pub fn addr(&self) -> Option<*mut c_void> {
        self.0.get_addr()
    }
}

#[cfg(feature = "ffi")]
impl CreateError {
    /// Converts the error into the raw `u32` result-code expected by C callers.
    pub fn into_rc(self) -> u32 {
        use nx_svc::error::ToRawResultCode;

        self.0.to_rc()
    }
}

#[allow(unused)]
mod _priv {
    pub trait Sealed {}
}
