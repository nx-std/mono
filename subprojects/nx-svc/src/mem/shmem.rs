//! Shared memory helpers for Horizon OS kernel.
//!
//! Provides safe wrappers around the low-level SVCs involved in creating
//! and managing shared memory kernel objects.

use core::ffi::c_void;

use crate::{
    error::{KernelError as KError, ToRawResultCode},
    raw::{self, Handle as RawHandle, INVALID_HANDLE},
    result::{Error, ResultCode, raw::Result as RawResult},
};

/// A handle to a shared memory kernel object.
///
/// The handle is invalid until the shared memory object is created, after
/// that it will remain valid until the shared memory object is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Handle(RawHandle);

impl Handle {
    /// Returns `true` if the handle is valid.
    pub fn is_valid(&self) -> bool {
        self.0 != INVALID_HANDLE
    }
}

/// Creates a shared memory kernel object.
///
/// On success returns the newly created shared memory kernel [`Handle`].
pub fn create_shared_memory(
    size: usize,
    local_perm: u32,
    remote_perm: u32,
) -> Result<Handle, CreateSharedMemoryError> {
    let mut handle: RawHandle = INVALID_HANDLE;
    let rc = unsafe { raw::create_shared_memory(&mut handle, size, local_perm, remote_perm) };

    RawResult::from_raw(rc).map(Handle(handle), |rc| match rc.description() {
        desc if KError::InvalidSize == desc => CreateSharedMemoryError::InvalidSize,
        desc if KError::OutOfResource == desc => CreateSharedMemoryError::OutOfResource,
        desc if KError::OutOfMemory == desc => CreateSharedMemoryError::OutOfMemory,
        desc if KError::InvalidNewMemoryPermission == desc => {
            CreateSharedMemoryError::InvalidPermission
        }
        desc if KError::InvalidMemoryRegion == desc => CreateSharedMemoryError::InvalidMemoryRegion,
        desc if KError::LimitReached == desc => CreateSharedMemoryError::LimitReached,
        _ => CreateSharedMemoryError::Unknown(rc.into()),
    })
}

/// Maps a shared memory object into the current process.
pub fn map_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32,
) -> Result<(), MapSharedMemoryError> {
    let rc = unsafe { raw::map_shared_memory(handle.0, addr, size, perm) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => MapSharedMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => MapSharedMemoryError::InvalidAddress,
        desc if KError::InvalidCurrentMemory == desc => MapSharedMemoryError::InvalidCurrentMemory,
        desc if KError::InvalidMemoryRegion == desc => MapSharedMemoryError::InvalidMemoryRegion,
        desc if KError::OutOfResource == desc => MapSharedMemoryError::OutOfResource,
        _ => MapSharedMemoryError::Unknown(rc.into()),
    })
}

/// Unmaps a previously mapped shared memory kernel object.
pub fn unmap_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
) -> Result<(), UnmapSharedMemoryError> {
    let rc = unsafe { raw::unmap_shared_memory(handle.0, addr, size) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => UnmapSharedMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => UnmapSharedMemoryError::InvalidAddress,
        desc if KError::InvalidCurrentMemory == desc => {
            UnmapSharedMemoryError::InvalidCurrentMemory
        }
        desc if KError::InvalidMemoryRegion == desc => UnmapSharedMemoryError::InvalidMemoryRegion,
        _ => UnmapSharedMemoryError::Unknown(rc.into()),
    })
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
pub enum CreateSharedMemoryError {
    #[error("Invalid size")]
    InvalidSize,
    #[error("Out of resource")]
    OutOfResource,
    #[error("Out of memory")]
    OutOfMemory,
    #[error("Invalid permission")]
    InvalidPermission,
    #[error("Invalid memory region")]
    InvalidMemoryRegion,
    #[error("Limit reached")]
    LimitReached,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for CreateSharedMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::OutOfMemory => KError::OutOfMemory.to_rc(),
            Self::InvalidPermission => KError::InvalidNewMemoryPermission.to_rc(),
            Self::InvalidMemoryRegion => KError::InvalidMemoryRegion.to_rc(),
            Self::LimitReached => KError::LimitReached.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
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
    #[error("Out of resource")]
    OutOfResource,
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
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UnmapSharedMemoryError {
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Invalid memory state")]
    InvalidCurrentMemory,
    #[error("Invalid memory region")]
    InvalidMemoryRegion,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for UnmapSharedMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::InvalidMemoryRegion => KError::InvalidMemoryRegion.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
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
