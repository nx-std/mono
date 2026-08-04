//! Transfer memory helpers for Horizon OS kernel.
//!
//! Provides safe wrappers around the low-level SVCs involved in creating and
//! managing transfer memory kernel objects.

use core::{
    ffi::c_void,
    ptr::NonNull,
};

use bitflags::bitflags;

use crate::{
    error::{
        _sealed,
        KernelError as KError,
        ToResultCode,
    },
    raw,
    result::{
        Error,
        ResultCode,
        raw::Result as RawResult,
    },
};

define_handle_type! {
    /// A handle to a transfer memory kernel object.
    pub struct Handle
}

/// Creates a transfer memory kernel object from an existing memory region.
///
/// `addr` **must** be page-aligned (4 KiB) and have at least `size` bytes
/// allocated. On success this function returns the newly created transferWha
/// memory [`Handle`].
pub fn create_transfer_memory(
    addr: NonNull<c_void>,
    size: usize,
    perm: MemoryPermission,
) -> Result<Handle, CreateTransferMemoryError> {
    let mut handle = raw::INVALID_HANDLE;
    let rc = unsafe { raw::create_transfer_memory(&mut handle, addr.as_ptr(), size, perm.bits()) };

    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidSize == desc => CreateTransferMemoryError::InvalidSize,
        desc if KError::InvalidAddress == desc => CreateTransferMemoryError::InvalidAddress,
        desc if KError::InvalidNewMemoryPermission == desc => {
            CreateTransferMemoryError::InvalidPermission
        }
        desc if KError::InvalidCurrentMemory == desc => CreateTransferMemoryError::InvalidMemState,
        desc if KError::LimitReached == desc => CreateTransferMemoryError::LimitReached,
        _ => CreateTransferMemoryError::Unknown(rc.into()),
    })?;

    // Wrapped only after the result code says the kernel filled the out-param;
    // on the failure path it still holds `INVALID_HANDLE`.
    Handle::try_from(handle).map_err(CreateTransferMemoryError::NoTransferMemoryHandle)
}

/// Maps a transfer memory object into the current process.
pub fn map_transfer_memory(
    handle: Handle,
    addr: NonNull<c_void>,
    size: usize,
    perm: MemoryPermission,
) -> Result<(), MapTransferMemoryError> {
    let rc = unsafe { raw::map_transfer_memory(handle.to_raw(), addr.as_ptr(), size, perm.bits()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => MapTransferMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => MapTransferMemoryError::InvalidAddress,
        desc if KError::InvalidCurrentMemory == desc => {
            MapTransferMemoryError::InvalidCurrentMemory
        }
        desc if KError::InvalidSize == desc => MapTransferMemoryError::InvalidSize,
        desc if KError::InvalidMemoryRegion == desc => MapTransferMemoryError::InvalidMemoryRegion,
        desc if KError::InvalidNewMemoryPermission == desc => {
            MapTransferMemoryError::InvalidPermission
        }
        _ => MapTransferMemoryError::Unknown(rc.into()),
    })
}

/// Unmaps a previously mapped transfer memory object.
pub fn unmap_transfer_memory(
    handle: Handle,
    addr: NonNull<c_void>,
    size: usize,
) -> Result<(), UnmapTransferMemoryError> {
    let rc = unsafe { raw::unmap_transfer_memory(handle.to_raw(), addr.as_ptr(), size) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => UnmapTransferMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => UnmapTransferMemoryError::InvalidAddress,
        desc if KError::InvalidCurrentMemory == desc => {
            UnmapTransferMemoryError::InvalidCurrentMemory
        }
        desc if KError::InvalidSize == desc => UnmapTransferMemoryError::InvalidSize,
        desc if KError::InvalidMemoryRegion == desc => {
            UnmapTransferMemoryError::InvalidMemoryRegion
        }
        _ => UnmapTransferMemoryError::Unknown(rc.into()),
    })
}

/// Closes a transfer memory handle.
pub fn close_handle(handle: Handle) -> Result<(), CloseHandleError> {
    let rc = unsafe { raw::close_handle(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => CloseHandleError::InvalidHandle,
        _ => CloseHandleError::Unknown(rc.into()),
    })
}

bitflags! {
    /// Memory permissions for transfer memory operations
    ///
    /// Only a subset of memory permissions are valid for transfer memory:
    /// - NONE: No permissions
    /// - R: Read permission
    /// - RW: Read/write permissions
    ///
    /// Other permission combinations (Write-only, Execute, etc.) are rejected
    /// by the kernel and will result in InvalidPermission errors.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    #[repr(transparent)]
    pub struct MemoryPermission: u32 {
        /// No permissions
        const NONE = 0;
        /// Read permission
        const R = 1 << 0;
        /// Write permission (used only for RW combination)
        #[doc(hidden)]
        const _W = 1 << 1;
        /// Read/write permissions
        const RW = Self::R.bits() | Self::_W.bits();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateTransferMemoryError {
    #[error("Invalid size")]
    InvalidSize,
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Invalid permission")]
    InvalidPermission,
    #[error("Invalid memory state")]
    InvalidMemState,
    #[error("Limit reached")]
    LimitReached,
    /// The kernel reported success but left the handle out-param unset.
    ///
    /// Indicates a mismatch with the SVC ABI rather than anything the caller
    /// passed, and no transfer memory object this process owns was created.
    #[error("The kernel reported success without returning a transfer memory handle")]
    NoTransferMemoryHandle(#[source] crate::handle::InvalidHandleError),
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToResultCode for CreateTransferMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::NoTransferMemoryHandle(_) => KError::InvalidHandle.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::InvalidPermission => KError::InvalidNewMemoryPermission.to_rc(),
            Self::InvalidMemState => KError::InvalidCurrentMemory.to_rc(),
            Self::LimitReached => KError::LimitReached.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for CreateTransferMemoryError {}

#[derive(Debug, thiserror::Error)]
pub enum MapTransferMemoryError {
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Invalid size")]
    InvalidSize,
    #[error("Invalid memory state")]
    InvalidCurrentMemory,
    #[error("Invalid memory region")]
    InvalidMemoryRegion,
    #[error("Invalid permission")]
    InvalidPermission,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToResultCode for MapTransferMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::InvalidMemoryRegion => KError::InvalidMemoryRegion.to_rc(),
            Self::InvalidPermission => KError::InvalidNewMemoryPermission.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for MapTransferMemoryError {}

#[derive(Debug, thiserror::Error)]
pub enum UnmapTransferMemoryError {
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Invalid size")]
    InvalidSize,
    #[error("Invalid memory state")]
    InvalidCurrentMemory,
    #[error("Invalid memory region")]
    InvalidMemoryRegion,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToResultCode for UnmapTransferMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::InvalidMemoryRegion => KError::InvalidMemoryRegion.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for UnmapTransferMemoryError {}

#[derive(Debug, thiserror::Error)]
pub enum CloseHandleError {
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToResultCode for CloseHandleError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for CloseHandleError {}
