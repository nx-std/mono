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
//!     manager (`crate::vmm`).  The classic `virtmemLock()` /
//!     `virtmemUnlock()` pair is therefore expressed with the RAII guard
//!     returned by [`crate::vmm::sys::lock`].
//! 3.  Instead of a single mutable C struct, the state of a shared-memory
//!     object is encoded in the type system via the zero-cost phantom-type
//!     wrapper [`SharedMemory<S>`], where the type parameter captures whether
//!     the segment is currently [`Unmapped`] or [`Mapped`].  This makes many
//!     classes of misuse (double-mapping, premature unmapping, etc.)
//!     impossible to represent in safe Rust.
//!
//! Unless you are contributing to `nx-sys-mem` itself you should access the
//! shared-memory functionality through the safe API exposed by this module;
//! C callers must use the `__nx_shmem_*` entry points exported by
//! `crate::shmem::ffi`.

use core::{ffi::c_void, ptr::NonNull};

use nx_svc::{
    error::ToRawResultCode,
    mem::shmem::{
        self as svc, Handle, LocalShmemPermission, MemoryPermission, RemoteShmemPermission,
    },
};

use crate::vmm::sys as vmm;

/// Guard size 0x1000, per libnx
const GUARD_SIZE: usize = 0x1000;

/// Memory-permission bitmask for shared memory operations.
pub type Permissions = MemoryPermission;
pub type LocalPermissions = LocalShmemPermission;
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
#[inline]
pub fn load_remote(
    shm: &mut SharedMemory<Unmapped>,
    handle: Handle,
    size: usize,
    perm: Permissions,
) {
    shm.0 = Unmapped { handle, size, perm };
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
        addr,
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
        addr,
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
    // SAFETY: The handle is guaranteed to be valid since the creation
    // of the shared memory object is guarded by the `create` function.
    #[cfg(debug_assertions)]
    if !handle.is_valid() {
        panic!("Invalid shared memory handle: INVALID_SHMEM_HANDLE");
    }

    svc::close_handle(handle).map_err(|err| CloseError { reason: err, shm })
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CreateError(#[from] svc::CreateSharedMemoryError);

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    /// Failed to allocate a virtual address range
    #[error("Failed to allocate virtual address range")]
    VirtAddressAllocFailed,
    #[error(transparent)]
    Svc(#[from] svc::MapSharedMemoryError),
}

#[derive(Debug, thiserror::Error)]
#[error("Shared memory unmap failed: {reason}")]
pub struct UnmapError {
    /// The error returned by the kernel
    #[source]
    pub reason: svc::UnmapSharedMemoryError,
    /// The shared memory object that was unmapped
    pub shm: SharedMemory<Mapped>,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to close shared memory: {reason}")]
pub struct CloseError {
    /// The error returned by the kernel
    #[source]
    pub reason: svc::CloseHandleError,
    /// The shared memory object that was closed
    pub shm: SharedMemory<Unmapped>,
}

pub trait ShmState: _priv::Sealed {
    fn handle(&self) -> Handle;
    fn size(&self) -> usize;
    fn perm(&self) -> Permissions;
    fn is_mapped(&self) -> bool;
    fn get_addr(&self) -> Option<*mut c_void>;
}

#[derive(Debug)]
pub struct Unmapped {
    handle: Handle,
    size: usize,
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

#[derive(Debug)]
pub struct Mapped {
    handle: Handle,
    size: usize,
    perm: Permissions,
    addr: NonNull<c_void>,
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
        Some(self.addr.as_ptr())
    }
}

impl _priv::Sealed for Mapped {}

#[allow(unused)]
mod _priv {
    pub trait Sealed {}
}

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
            addr,
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

impl CreateError {
    /// Converts the error into the raw `u32` result-code expected by C callers.
    pub fn into_rc(self) -> u32 {
        self.0.to_rc()
    }
}
