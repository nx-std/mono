//! Shared memory helpers for Horizon OS kernel.
//!
//! Provides safe wrappers around the low-level SVCs involved in creating
//! and managing shared memory kernel objects.

use core::{ffi::c_void, ptr::NonNull};

use bitflags::bitflags;

use crate::{
    error::{KernelError as KError, ToRawResultCode},
    raw,
    result::{Error, ResultCode, raw::Result as RawResult},
};

define_handle_type! {
    /// A handle to a shared memory kernel object.
    pub struct Handle
}

/// Creates a shared memory kernel object.
///
/// On success returns the newly created shared memory kernel [`Handle`].
pub fn create_shared_memory(
    size: usize,
    local_perm: LocalShmemPermission,
    remote_perm: RemoteShmemPermission,
) -> Result<Handle, CreateSharedMemoryError> {
    let mut handle = raw::INVALID_HANDLE;
    let rc = unsafe {
        raw::create_shared_memory(&mut handle, size, local_perm.bits(), remote_perm.bits())
    };

    RawResult::from_raw(rc).map(Handle(handle), |rc| match rc.description() {
        desc if KError::OutOfMemory == desc => CreateSharedMemoryError::OutOfMemory,
        desc if KError::LimitReached == desc => CreateSharedMemoryError::LimitReached,
        _ => CreateSharedMemoryError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum CreateSharedMemoryError {
    #[error("Out of memory")]
    OutOfMemory,
    #[error("Limit reached")]
    LimitReached,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for CreateSharedMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::OutOfMemory => KError::OutOfMemory.to_rc(),
            Self::LimitReached => KError::LimitReached.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Maps a shared memory object into the current process.
pub fn map_shared_memory(
    handle: Handle,
    addr: NonNull<c_void>,
    size: usize,
    perm: MemoryPermission,
) -> Result<(), MapSharedMemoryError> {
    let rc = unsafe { raw::map_shared_memory(handle.0, addr.as_ptr(), size, perm.bits()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => MapSharedMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => MapSharedMemoryError::InvalidAddress,
        desc if KError::InvalidCurrentMemory == desc => MapSharedMemoryError::InvalidCurrentMemory,
        desc if KError::InvalidMemoryRegion == desc => MapSharedMemoryError::InvalidMemoryRegion,
        desc if KError::InvalidSize == desc => MapSharedMemoryError::InvalidSize,
        desc if KError::InvalidNewMemoryPermission == desc => {
            MapSharedMemoryError::InvalidPermission
        }
        desc if KError::OutOfResource == desc => MapSharedMemoryError::OutOfResource,
        desc if KError::OutOfMemory == desc => MapSharedMemoryError::OutOfMemory,
        _ => MapSharedMemoryError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum MapSharedMemoryError {
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Invalid memory state")]
    InvalidCurrentMemory,
    #[error("Invalid memory region")]
    InvalidMemoryRegion,
    #[error("Invalid size")]
    InvalidSize,
    #[error("Invalid permission")]
    InvalidPermission,
    #[error("Out of resource")]
    OutOfResource,
    #[error("Out of memory")]
    OutOfMemory,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for MapSharedMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::InvalidMemoryRegion => KError::InvalidMemoryRegion.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidPermission => KError::InvalidNewMemoryPermission.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::OutOfMemory => KError::OutOfMemory.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Unmaps a previously mapped shared memory kernel object.
pub fn unmap_shared_memory(
    handle: Handle,
    addr: NonNull<c_void>,
    size: usize,
) -> Result<(), UnmapSharedMemoryError> {
    let rc = unsafe { raw::unmap_shared_memory(handle.0, addr.as_ptr(), size) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidCurrentMemory == desc => {
            UnmapSharedMemoryError::InvalidCurrentMemory
        }
        desc if KError::InvalidSize == desc => UnmapSharedMemoryError::InvalidSize,
        desc if KError::InvalidMemoryRegion == desc => UnmapSharedMemoryError::InvalidMemoryRange,
        desc if KError::OutOfResource == desc => UnmapSharedMemoryError::OutOfResource,
        _ => UnmapSharedMemoryError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum UnmapSharedMemoryError {
    #[error("Invalid memory state")]
    InvalidCurrentMemory,
    #[error("Invalid size")]
    InvalidSize,
    #[error("Invalid memory range")]
    InvalidMemoryRange,
    #[error("Out of resource")]
    OutOfResource,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for UnmapSharedMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidMemoryRange => KError::InvalidMemoryRegion.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Closes a shared memory kernel object handle.
pub fn close_handle(handle: Handle) -> Result<(), CloseHandleError> {
    let rc = unsafe { raw::close_handle(handle.0) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => CloseHandleError::InvalidHandle,
        _ => CloseHandleError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum CloseHandleError {
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for CloseHandleError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

bitflags! {
    /// Local shared memory permissions
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct LocalShmemPermission: u32 {
        /// Read permission
        const R = 1 << 0;
        /// Write permission (used only for RW combination)
        #[doc(hidden)]
        const _W = 1 << 1;
        /// Read/write permissions
        const RW = Self::R.bits() | Self::_W.bits();
    }
}

bitflags! {
    /// Remote shared memory permissions
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct RemoteShmemPermission: u32 {
        /// Read permission
        const R = 1 << 0;
        /// Write permission (used only for RW combination)
        #[doc(hidden)]
        const _W = 1 << 1;
        /// Read/write permissions
        const RW = Self::R.bits() | Self::_W.bits();
        /// Don't care permission
        const DONT_CARE = 1 << 28;
    }
}

bitflags! {
    /// Memory permissions for shared memory objects
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct MemoryPermission: u32 {
        /// Read permission
        const R = 1 << 0;
        /// Write permission (used only for RW combination)
        #[doc(hidden)]
        const _W = 1 << 1;
        /// Read/write permissions
        const RW = Self::R.bits() | Self::_W.bits();
    }
}
