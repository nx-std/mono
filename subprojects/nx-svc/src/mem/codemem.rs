//! Code memory kernel objects.
//!
//! A code memory object is created over pages a process already owns, and then mapped somewhere
//! they may be executed: into the creating process, or into the process that owns the pages. It is
//! how a program that generates code at run time gets that code executable without asking a
//! loader to do it.
//!
//! ## The operation decides the permission
//!
//! The kernel accepts exactly one permission per operation: mapping into the creator is
//! read-write, mapping to the owner is read or read-execute, and both unmaps take none at all.
//! There is therefore no permission parameter on three of the four functions here, and the fourth
//! takes [`OwnerPermission`], which is the choice the kernel actually offers.
//!
//! ## Handles here name, they do not own
//!
//! [`Handle`] is `Copy` and closes nothing, like every other handle this crate hands out.
//! [`close_handle`] is the only closer, and whichever layer decides a code memory object's
//! lifetime is where the owning type belongs.

use core::{
    ffi::c_void,
    ptr::NonNull,
};

use crate::{
    error::{
        _sealed,
        KernelError as KError,
        ToResultCode,
    },
    mem::core::MemoryPermission,
    raw,
    result::{
        Error,
        ResultCode,
        raw::Result as RawResult,
    },
};

define_handle_type! {
    /// A handle to a code memory kernel object.
    pub struct Handle
}

/// Creates a code memory object over `size` bytes at `src`.
///
/// The pages stay mapped in the calling process while the object exists; the object is what lets
/// them be mapped a second time, as code.
///
/// Both `src` and `size` are page aligned, and the range is one the calling process already has
/// mapped.
///
/// # Errors
///
/// Returns [`CreateCodeMemoryError::InvalidAddress`] when `src` is not page aligned,
/// [`CreateCodeMemoryError::InvalidSize`] when `size` is zero or not page aligned,
/// [`CreateCodeMemoryError::InvalidCurrentMemory`] when the range is not one this process has
/// mapped, and [`CreateCodeMemoryError::OutOfResource`] when the kernel has no room for another
/// object. Nothing is created on any of them.
pub fn create_code_memory(
    src: NonNull<c_void>,
    size: usize,
) -> Result<Handle, CreateCodeMemoryError> {
    let mut handle = raw::INVALID_HANDLE;
    let rc = unsafe { raw::create_code_memory(&mut handle, src.as_ptr(), size as u64) };

    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidAddress == desc => CreateCodeMemoryError::InvalidAddress,
        desc if KError::InvalidSize == desc => CreateCodeMemoryError::InvalidSize,
        desc if KError::InvalidCurrentMemory == desc => CreateCodeMemoryError::InvalidCurrentMemory,
        desc if KError::OutOfResource == desc => CreateCodeMemoryError::OutOfResource,
        _ => CreateCodeMemoryError::Unknown(rc.into()),
    })?;

    // Wrapped only after the result code says the kernel filled the out-param; on the failure path
    // it still holds `INVALID_HANDLE`.
    Handle::try_from(handle).map_err(CreateCodeMemoryError::NoCodeMemoryHandle)
}

/// Error returned by [`create_code_memory`].
#[derive(Debug, thiserror::Error)]
pub enum CreateCodeMemoryError {
    /// The address is not page aligned
    ///
    /// Detected before anything is created, so no object exists and the pages are untouched.
    #[error("Invalid address")]
    InvalidAddress,

    /// The size is zero, or not page aligned
    ///
    /// Detected before anything is created, so no object exists and the pages are untouched.
    #[error("Invalid size")]
    InvalidSize,

    /// The range is not one this process has mapped
    ///
    /// Occurs when the pages are outside the process's address space, or the range wraps. No
    /// object exists.
    #[error("Invalid memory state")]
    InvalidCurrentMemory,

    /// The kernel has no room for another object
    ///
    /// Occurs when the kernel's own allocation for the object failed, or the process's handle
    /// table is full. No object exists.
    #[error("Out of resource")]
    OutOfResource,

    /// The kernel reported success but left the handle out-param unset
    ///
    /// Indicates a mismatch with the SVC ABI rather than anything the caller passed. A code memory
    /// object may exist and be unreachable, so nothing here can close it.
    #[error("The kernel reported success without returning a code memory handle")]
    NoCodeMemoryHandle(#[source] crate::handle::InvalidHandleError),

    /// An unknown error occurred
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToResultCode for CreateCodeMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::InvalidSize => KError::InvalidSize.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::NoCodeMemoryHandle(_) => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for CreateCodeMemoryError {}

/// Maps the code memory into the calling process, at `dst`, as read-write.
///
/// This is the writable view: a caller writes the code it generated here, then maps the same
/// object to its owner with [`map_code_memory_to_owner`] to make it executable.
///
/// # Errors
///
/// See [`ControlCodeMemoryError`]. Nothing is mapped on any of them.
pub fn map_code_memory(
    handle: Handle,
    dst: NonNull<c_void>,
    size: usize,
) -> Result<(), ControlCodeMemoryError> {
    control(
        handle,
        raw::CodeMapOperation::Map,
        dst,
        size,
        MAP_PERMISSION,
    )
}

/// Unmaps from the calling process what [`map_code_memory`] mapped.
///
/// # Errors
///
/// See [`ControlCodeMemoryError`]. Nothing is unmapped on any of them.
pub fn unmap_code_memory(
    handle: Handle,
    dst: NonNull<c_void>,
    size: usize,
) -> Result<(), ControlCodeMemoryError> {
    control(
        handle,
        raw::CodeMapOperation::Unmap,
        dst,
        size,
        NO_PERMISSION,
    )
}

/// Maps the code memory into the process that owns its pages, at `dst`, with `perm`.
///
/// # Errors
///
/// See [`ControlCodeMemoryError`]. Nothing is mapped on any of them.
pub fn map_code_memory_to_owner(
    handle: Handle,
    dst: NonNull<c_void>,
    size: usize,
    perm: OwnerPermission,
) -> Result<(), ControlCodeMemoryError> {
    control(
        handle,
        raw::CodeMapOperation::MapToOwner,
        dst,
        size,
        perm.to_raw(),
    )
}

/// Unmaps from the owning process what [`map_code_memory_to_owner`] mapped.
///
/// # Errors
///
/// See [`ControlCodeMemoryError`]. Nothing is unmapped on any of them.
pub fn unmap_code_memory_from_owner(
    handle: Handle,
    dst: NonNull<c_void>,
    size: usize,
) -> Result<(), ControlCodeMemoryError> {
    control(
        handle,
        raw::CodeMapOperation::UnmapFromOwner,
        dst,
        size,
        NO_PERMISSION,
    )
}

/// Error returned by the four operations on a code memory object.
///
/// The four are one SVC under a fixed operation each, and they fail in the same ways, so they
/// report through one type rather than four identical ones.
#[derive(Debug, thiserror::Error)]
pub enum ControlCodeMemoryError {
    /// The handle names no code memory object
    ///
    /// Occurs when the handle was closed, names another kind of object, or, on a kernel that does
    /// not allow it, names an object this process created itself. Nothing was mapped or unmapped.
    #[error("Invalid handle")]
    InvalidHandle,

    /// The address is not page aligned
    ///
    /// Detected before the object is looked up. Nothing was mapped or unmapped.
    #[error("Invalid address")]
    InvalidAddress,

    /// The size is zero, or not page aligned
    ///
    /// Detected before the object is looked up. Nothing was mapped or unmapped.
    #[error("Invalid size")]
    InvalidSize,

    /// The range is not one the target process can hold
    ///
    /// Occurs when the range wraps, or the pages are outside the address space it would be mapped
    /// into. Nothing was mapped or unmapped.
    #[error("Invalid memory state")]
    InvalidCurrentMemory,

    /// The range is outside the region this operation maps into
    ///
    /// Occurs when the destination is not inside the region the kernel reserves for the kind of
    /// mapping asked for. Nothing was mapped or unmapped.
    #[error("Invalid memory range")]
    InvalidMemoryRegion,

    /// An unknown error occurred
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToResultCode for ControlCodeMemoryError {
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

impl _sealed::Sealed for ControlCodeMemoryError {}

/// What the owning process may do with a mapping [`map_code_memory_to_owner`] made.
///
/// The kernel accepts these two and nothing else for that operation, so the type carries the
/// choice rather than a permission word that is mostly invalid values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerPermission {
    /// The owner may read the mapping.
    Read,
    /// The owner may read and execute the mapping, which is what generated code is mapped for.
    ReadExecute,
}

impl OwnerPermission {
    /// Returns the permission word the SVC carries.
    pub fn to_raw(self) -> u64 {
        match self {
            Self::Read => MemoryPermission::R.bits() as u64,
            Self::ReadExecute => (MemoryPermission::R | MemoryPermission::X).bits() as u64,
        }
    }
}

/// Closes a code memory handle.
///
/// The object itself lives until every handle to it is closed and every mapping it made is
/// unmapped.
///
/// # Errors
///
/// Returns [`CloseHandleError::InvalidHandle`] when the handle names nothing, which means it was
/// closed already.
pub fn close_handle(handle: Handle) -> Result<(), CloseHandleError> {
    let rc = unsafe { raw::close_handle(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => CloseHandleError::InvalidHandle,
        _ => CloseHandleError::Unknown(rc.into()),
    })
}

/// Error returned by [`close_handle`].
#[derive(Debug, thiserror::Error)]
pub enum CloseHandleError {
    /// The handle names no kernel object
    ///
    /// Occurs when the handle was closed already. Nothing was closed a second time.
    #[error("Invalid handle")]
    InvalidHandle,

    /// An unknown error occurred
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

/// The permission the kernel requires when mapping into the creating process: read-write.
const MAP_PERMISSION: u64 = MemoryPermission::R.bits() as u64 | MemoryPermission::W.bits() as u64;

/// The permission the kernel requires for both unmaps: none.
const NO_PERMISSION: u64 = 0;

/// Issues the operation, which is the whole of the four functions above but the operation itself.
fn control(
    handle: Handle,
    op: raw::CodeMapOperation,
    dst: NonNull<c_void>,
    size: usize,
    perm: u64,
) -> Result<(), ControlCodeMemoryError> {
    let rc =
        unsafe { raw::control_code_memory(handle.to_raw(), op, dst.as_ptr(), size as u64, perm) };

    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => ControlCodeMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => ControlCodeMemoryError::InvalidAddress,
        desc if KError::InvalidSize == desc => ControlCodeMemoryError::InvalidSize,
        desc if KError::InvalidCurrentMemory == desc => {
            ControlCodeMemoryError::InvalidCurrentMemory
        }
        desc if KError::InvalidMemoryRegion == desc => ControlCodeMemoryError::InvalidMemoryRegion,
        _ => ControlCodeMemoryError::Unknown(rc.into()),
    })
}
