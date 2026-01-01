//! Miscellaneous system calls and utilities for the Horizon OS kernel.
//!
//! This module provides safe wrappers around various system calls that don't fit into other
//! categories. It includes functionality for retrieving system information, managing memory,
//! and other miscellaneous operations.
//!
//! The main functionality is centered around the [`get_info`] system call, which provides
//! a type-safe way to query various system properties and kernel object information.

pub use super::raw::{CUR_PROCESS_HANDLE, Handle, INVALID_HANDLE};
use super::{
    error::{KernelError, Module, ResultCode, ToRawResultCode},
    raw,
    result::{Error, Result, raw::Result as RawResult},
};

/// Retrieves random entropy.
///
/// This is a convenience wrapper around [`get_info`] for the [`RandomEntropy`] info type.
///
/// # Arguments
///
/// * `source` - The entropy source sub-ID (usually 0 for the default source)
///
/// Returns the random entropy value on success, or a [`GetInfoError`] on failure.
// TODO: Review the actual error codes this fn returns
pub fn get_random_entropy(source: u64) -> Result<u64, GetInfoError> {
    Ok(get_info(
        InfoType::RandomEntropy { source },
        INVALID_HANDLE,
    )?)
}

/// Retrieves the total amount of memory available for the current process.
///
/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of the total amount of memory available for the current process.
///
/// Returns the total amount of memory available for the current process on success,
/// or a [`GetInfoError`] on failure.
// TODO: Review the actual error codes this fn returns
pub fn get_total_memory_size() -> Result<usize, GetInfoError> {
    get_info(InfoType::TotalMemorySize, raw::CUR_PROCESS_HANDLE).map(|size| size as usize)
}

/// Retrieves the amount of memory currently used by the current process.
///
/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of the amount of memory currently used by the current process.
///
/// Returns the amount of memory currently used by the current process on success,
/// or a [`GetInfoError`] on failure.
// TODO: Review the actual error codes this fn returns
pub fn get_used_memory_size() -> Result<usize, GetInfoError> {
    get_info(InfoType::UsedMemorySize, raw::CUR_PROCESS_HANDLE).map(|size| size as usize)
}

/// Retrieves the base address and size of the alias region for the current process.
///
/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of the base address and size of the alias region for the current process.
///
/// Returns the base address and size of the alias region on success, or a [`GetInfoError`] on failure.
pub fn get_alias_region_info() -> Result<(usize, usize), GetInfoError> {
    let base = get_info(InfoType::AliasRegionAddress, raw::CUR_PROCESS_HANDLE)?;
    let size = get_info(InfoType::AliasRegionSize, raw::CUR_PROCESS_HANDLE)?;
    Ok((base as usize, size as usize))
}

/// Retrieves the extra size added to the reserved alias region.
///
/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of the extra size added to the reserved alias region.
///
/// Returns the extra size added to the reserved alias region on success,
/// or a [`GetInfoError`] on failure.
pub fn get_alias_region_extra_size() -> Result<usize, GetInfoError> {
    get_info(InfoType::AliasRegionExtraSize, raw::CUR_PROCESS_HANDLE).map(|size| size as usize)
}

/// Retrieves the base address and size of the heap region for the current process.
///
/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of the base address and size of the heap region for the current process.
///
/// Returns the base address and size of the heap region on success, or a [`GetInfoError`] on failure.
pub fn get_heap_region_info() -> Result<(usize, usize), GetInfoError> {
    let base = get_info(InfoType::HeapRegionAddress, raw::CUR_PROCESS_HANDLE)?;
    let size = get_info(InfoType::HeapRegionSize, raw::CUR_PROCESS_HANDLE)?;
    Ok((base as usize, size as usize))
}

/// Retrieves the base address and size of the ASLR region for the current process.
///
/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of the base address and size of the ASLR region for the current process.
///
/// Returns the base address and size of the ASLR region on success, or a [`GetInfoError`] on failure.
pub fn get_aslr_region_info() -> Result<(usize, usize), GetInfoError> {
    let base = get_info(InfoType::AslrRegionAddress, raw::CUR_PROCESS_HANDLE)?;
    let size = get_info(InfoType::AslrRegionSize, raw::CUR_PROCESS_HANDLE)?;
    Ok((base as usize, size as usize))
}

/// Retrieves the base address and size of the stack region for the current process.

/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of the base address and size of the stack region for the current process.
///
/// Returns the base address and size of the stack region on success, or a [`GetInfoError`] on failure.
pub fn get_stack_region_info() -> Result<(usize, usize), GetInfoError> {
    let base = get_info(InfoType::StackRegionAddress, raw::CUR_PROCESS_HANDLE)?;
    let size = get_info(InfoType::StackRegionSize, raw::CUR_PROCESS_HANDLE)?;
    Ok((base as usize, size as usize))
}

/// Retrieves information about the system or a kernel object.
///
/// This function provides a safe wrapper around the `svcGetInfo` system call, allowing
/// retrieval of various system properties and kernel object information.
///
/// # Arguments
///
/// * `out` - Pointer to where the retrieved information will be stored
/// * `id0` - Type of information to retrieve, specified using the [`InfoType`] enum
/// * `handle` - Handle of the object to retrieve information from, or [`INVALID_HANDLE`] to retrieve system information
///
/// # Returns
///
/// Returns `Ok(())` if the information was successfully retrieved, or an error if the
/// operation failed.
///
/// # Safety
///
/// This function is unsafe because:
/// * The caller must ensure the handle is valid if one is provided
pub fn get_info(info_type: InfoType, handle: Handle) -> Result<u64, GetInfoError> {
    let (id0, id1) = info_type.to_ids();
    let mut out = 0u64;

    let rc = unsafe { raw::get_info(&mut out, id0, handle, id1) };
    RawResult::from_raw(rc).map(out, |rc| {
        let desc = rc.description();

        // Map kernel error codes to the appropriate error enum variant
        if desc == KernelError::InvalidHandle {
            GetInfoError::InvalidHandle
        } else if desc == KernelError::InvalidAddress {
            GetInfoError::InvalidAddress
        } else if desc == KernelError::InvalidEnumValue {
            // Check if it's an info type or ID error based on the error code
            if rc.module() == Module::Kernel {
                if desc == KernelError::InvalidEnumValue {
                    GetInfoError::InvalidInfoType
                } else {
                    GetInfoError::InvalidInfoId
                }
            } else {
                GetInfoError::Unknown(Error::from(rc))
            }
        } else {
            GetInfoError::Unknown(Error::from(rc))
        }
    })
}

/// InfoType for svcGetInfo. Only some variants require a sub-ID (id1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoType {
    /// Bitmask of allowed Core IDs.
    CoreMask,
    /// Bitmask of allowed Thread Priorities.
    PriorityMask,
    /// Base of the Alias memory region.
    AliasRegionAddress,
    /// Size of the Alias memory region.
    AliasRegionSize,
    /// Base of the Heap memory region.
    HeapRegionAddress,
    /// Size of the Heap memory region.
    HeapRegionSize,
    /// Total amount of memory available for process.
    TotalMemorySize,
    /// Amount of memory currently used by process.
    UsedMemorySize,
    /// Whether current process is being debugged.
    DebuggerAttached,
    /// Current process's resource limit handle.
    ResourceLimit,
    /// [2.0.0+] Number of idle ticks on CPU. Requires a core sub-ID.
    IdleTickCount { core: u64 },
    /// [2.0.0+] Random entropy for current process. Requires a source sub-ID.
    RandomEntropy { source: u64 },
    /// [2.0.0+] Base of the process's address space.
    AslrRegionAddress,
    /// [2.0.0+] Size of the process's address space.
    AslrRegionSize,
    /// [2.0.0+] Base of the Stack memory region.
    StackRegionAddress,
    /// [2.0.0+] Size of the Stack memory region.
    StackRegionSize,
    /// [3.0.0+] Total memory allocated for process memory management.
    SystemResourceSizeTotal,
    /// [3.0.0+] Amount of memory currently used by process memory management.
    SystemResourceSizeUsed,
    /// [3.0.0+] Program ID for the process.
    ProgramId,
    /// [4.0.0-4.1.0] Min/max initial process IDs. Requires a sub-ID.
    InitialProcessIdRange { which: u64 },
    /// [5.0.0+] Address of the process's exception context (for break).
    UserExceptionContextAddress,
    /// [6.0.0+] Total amount of memory available for process, excluding that for process memory management.
    TotalNonSystemMemorySize,
    /// [6.0.0+] Amount of memory used by process, excluding that for process memory management.
    UsedNonSystemMemorySize,
    /// [9.0.0+] Whether the specified process is an Application.
    IsApplication,
    /// [11.0.0+] The number of free threads available to the process's resource limit.
    FreeThreadCount,
    /// [13.0.0+] Number of ticks spent on thread. Requires a core sub-ID.
    ThreadTickCount { core: u64 },
    /// [14.0.0+] Does process have access to SVC (only usable with svcSynchronizePreemptionState at present).
    IsSvcPermitted,
    /// [16.0.0+] Low bits of the physical address for a KIoRegion.
    IoRegionHint,
    /// [18.0.0+] Extra size added to the reserved region.
    AliasRegionExtraSize,
    /// [19.0.0+] Low bits of the process address for a KTransferMemory.
    TransferMemoryHint,
    /// [1.0.0-12.1.0] Number of ticks spent on thread (deprecated).
    ThreadTickCountDeprecated,
}

impl InfoType {
    /// Returns the (id0, id1) pair for [`__nx_svc__svc_get_info`].
    pub fn to_ids(&self) -> (u32, u64) {
        match *self {
            InfoType::CoreMask => (0, 0),
            InfoType::PriorityMask => (1, 0),
            InfoType::AliasRegionAddress => (2, 0),
            InfoType::AliasRegionSize => (3, 0),
            InfoType::HeapRegionAddress => (4, 0),
            InfoType::HeapRegionSize => (5, 0),
            InfoType::TotalMemorySize => (6, 0),
            InfoType::UsedMemorySize => (7, 0),
            InfoType::DebuggerAttached => (8, 0),
            InfoType::ResourceLimit => (9, 0),
            InfoType::IdleTickCount { core } => (10, core),
            InfoType::RandomEntropy { source } => (11, source),
            InfoType::AslrRegionAddress => (12, 0),
            InfoType::AslrRegionSize => (13, 0),
            InfoType::StackRegionAddress => (14, 0),
            InfoType::StackRegionSize => (15, 0),
            InfoType::SystemResourceSizeTotal => (16, 0),
            InfoType::SystemResourceSizeUsed => (17, 0),
            InfoType::ProgramId => (18, 0),
            InfoType::InitialProcessIdRange { which } => (19, which),
            InfoType::UserExceptionContextAddress => (20, 0),
            InfoType::TotalNonSystemMemorySize => (21, 0),
            InfoType::UsedNonSystemMemorySize => (22, 0),
            InfoType::IsApplication => (23, 0),
            InfoType::FreeThreadCount => (24, 0),
            InfoType::ThreadTickCount { core } => (25, core),
            InfoType::IsSvcPermitted => (26, 0),
            InfoType::IoRegionHint => (27, 0),
            InfoType::AliasRegionExtraSize => (28, 0),
            InfoType::TransferMemoryHint => (34, 0),
            InfoType::ThreadTickCountDeprecated => (0xF0000002, 0),
        }
    }
}

/// Error type for [`get_info`] operations.
///
/// This enum represents the various error conditions that can occur when attempting to
/// retrieve system information or kernel object properties.
#[derive(Debug, thiserror::Error)]
pub enum GetInfoError {
    /// The handle is invalid.
    #[error("Invalid handle")]
    InvalidHandle,
    /// The output memory address cannot be accessed.
    #[error("Invalid memory state")]
    InvalidMemState,
    /// The address is invalid.
    #[error("Invalid address")]
    InvalidAddress,
    /// The info type is invalid.
    #[error("Invalid info type")]
    InvalidInfoType,
    /// The info ID is invalid.
    #[error("Invalid info ID")]
    InvalidInfoId,
    /// An unknown error occurred.
    ///
    /// This variant is used when the error code is not recognized.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for GetInfoError {
    /// Converts the error into its raw result code representation.
    fn to_rc(self) -> ResultCode {
        match self {
            GetInfoError::InvalidHandle => KernelError::InvalidHandle.to_rc(),
            GetInfoError::InvalidMemState => KernelError::InvalidAddress.to_rc(),
            GetInfoError::InvalidAddress => KernelError::InvalidAddress.to_rc(),
            GetInfoError::InvalidInfoType => KernelError::InvalidEnumValue.to_rc(),
            GetInfoError::InvalidInfoId => KernelError::InvalidEnumValue.to_rc(),
            GetInfoError::Unknown(err) => err.to_raw(),
        }
    }
}
