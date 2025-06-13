//! Memory management system calls and utilities for the Horizon OS kernel.
//!
//! This module provides safe wrappers around memory-related system calls for querying
//! memory properties and unmapping memory.

use core::ffi::c_void;

use super::{
    error::{KernelError as KError, ResultCode, ToRawResultCode},
    raw,
    result::{Error, raw::Result as RawResult},
};

/// Page information
pub type PageInfo = u32;

/// Queries information about a memory address.
///
/// This function is used to get detailed information about a memory address,
/// including its type, attributes, permissions, and reference counts.
///
/// # Arguments
///
/// * `addr` - The address to query
///
/// Returns `Ok((MemoryInfo, PageInfo))` containing the memory information and page info if successful,
/// or a [`MemoryError`] on failure.
pub fn query_memory(addr: usize) -> Result<(MemoryInfo, PageInfo), MemoryError> {
    let mut mem_info = Default::default();
    let mut page_info = Default::default();

    let rc = unsafe { raw::__nx_svc_query_memory(&mut mem_info, &mut page_info, addr) };
    RawResult::from_raw(rc).map((mem_info.into(), page_info), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => MemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => MemoryError::InvalidAddress,
        _ => MemoryError::Unknown(rc.into()),
    })
}

/// Unmaps a memory range.
///
/// This function is used to unmap a previously mapped memory range.
/// It's commonly used in legacy kernel detection code.
///
/// # Arguments
///
/// * `dst_addr` - The destination address
/// * `src_addr` - The source address
/// * `size` - The size of the memory range to unmap
///
/// Returns `Ok(())` if the memory was successfully unmapped, or a [`MemoryError`] on failure.
pub fn unmap_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> Result<(), MemoryError> {
    let rc = unsafe { raw::__nx_svc_unmap_memory(dst_addr, src_addr, size) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => MemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => MemoryError::InvalidAddress,
        _ => MemoryError::Unknown(rc.into()),
    })
}

/// Error type for memory operations.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// The handle is invalid.
    #[error("Invalid handle")]
    InvalidHandle,
    /// The memory address is invalid.
    #[error("Invalid address")]
    InvalidAddress,
    /// The memory state is invalid.
    #[error("Invalid memory state")]
    InvalidMemState,
    /// The memory range is invalid.
    #[error("Invalid memory range")]
    InvalidMemRange,
    /// An unknown error occurred.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for MemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            MemoryError::InvalidHandle => KError::InvalidHandle.to_rc(),
            MemoryError::InvalidAddress => KError::InvalidAddress.to_rc(),
            MemoryError::InvalidMemState => KError::InvalidAddress.to_rc(),
            MemoryError::InvalidMemRange => KError::InvalidAddress.to_rc(),
            MemoryError::Unknown(err) => err.to_raw(),
        }
    }
}

/// Information about a memory region.
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// Base address
    pub addr: usize,
    /// Size
    pub size: usize,
    /// Memory type
    pub typ: MemoryType,
    /// Memory state
    pub state: MemoryState,
    /// Memory attributes
    pub attr: MemoryAttribute,
    /// Memory permissions
    pub perm: MemoryPermission,
    /// IPC reference count
    pub ipc_refcount: u32,
    /// Device reference count
    pub device_refcount: u32,
}

impl From<raw::MemoryInfo> for MemoryInfo {
    fn from(value: raw::MemoryInfo) -> Self {
        let (mem_state, mem_type) = parse_mem_type(value.typ);
        Self {
            addr: value.addr,
            size: value.size,
            typ: mem_type,
            state: mem_state,
            attr: MemoryAttribute(raw::MemoryAttribute::from_bits_truncate(value.attr)),
            perm: MemoryPermission(raw::MemoryPermission::from_bits_truncate(value.perm)),
            ipc_refcount: value.ipc_refcount,
            device_refcount: value.device_refcount,
        }
    }
}

/// Parses the memory type and state from a raw value
fn parse_mem_type(value: u32) -> (MemoryState, MemoryType) {
    let mem_state_bits = value & !raw::MEMORY_TYPE_MASK;
    let mem_type_bits = (value & raw::MEMORY_TYPE_MASK) as u8;

    let mem_state = MemoryState(raw::MemoryState::from_bits_truncate(mem_state_bits));
    let mem_type = unsafe { core::mem::transmute::<u8, raw::MemoryType>(mem_type_bits) }.into();

    (mem_state, mem_type)
}

/// Memory state flags that control memory region behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MemoryState(raw::MemoryState);

impl MemoryState {
    /// Returns whether permission changes are allowed
    pub fn can_change_permissions(&self) -> bool {
        self.0.contains(raw::MemoryState::PERM_CHANGE_ALLOWED)
    }

    /// Returns whether this memory region can be mapped
    pub fn can_map(&self) -> bool {
        self.0.contains(raw::MemoryState::MAP_ALLOWED)
    }

    /// Returns whether this memory region can be used for IPC
    pub fn can_ipc(&self) -> bool {
        self.0.contains(raw::MemoryState::IPC_BUFFER_ALLOWED)
    }

    /// Returns whether this memory region can be force read/written by debug syscalls
    pub fn can_force_rw_by_debug_syscalls(&self) -> bool {
        self.0
            .contains(raw::MemoryState::FORCE_RW_BY_DEBUG_SYSCALLS)
    }

    /// Returns whether IPC type 0 send is allowed
    pub fn can_ipc_send_type0(&self) -> bool {
        self.0.contains(raw::MemoryState::IPC_SEND_ALLOWED_TYPE0)
    }

    /// Returns whether IPC type 3 send is allowed
    pub fn can_ipc_send_type3(&self) -> bool {
        self.0.contains(raw::MemoryState::IPC_SEND_ALLOWED_TYPE3)
    }

    /// Returns whether IPC type 1 send is allowed
    pub fn can_ipc_send_type1(&self) -> bool {
        self.0.contains(raw::MemoryState::IPC_SEND_ALLOWED_TYPE1)
    }

    /// Returns whether process permission changes are allowed
    pub fn can_change_process_permissions(&self) -> bool {
        self.0
            .contains(raw::MemoryState::PROCESS_PERM_CHANGE_ALLOWED)
    }

    /// Returns whether unmapping process code memory is allowed
    pub fn can_unmap_process_code_memory(&self) -> bool {
        self.0
            .contains(raw::MemoryState::UNMAP_PROCESS_CODE_MEM_ALLOWED)
    }

    /// Returns whether transfer memory operations are allowed
    pub fn can_transfer_memory(&self) -> bool {
        self.0.contains(raw::MemoryState::TRANSFER_MEM_ALLOWED)
    }

    /// Returns whether querying physical addresses is allowed
    pub fn can_query_physical_address(&self) -> bool {
        self.0.contains(raw::MemoryState::QUERY_PADDR_ALLOWED)
    }

    /// Returns whether mapping device memory is allowed
    pub fn can_map_device(&self) -> bool {
        self.0.contains(raw::MemoryState::MAP_DEVICE_ALLOWED)
    }

    /// Returns whether mapping aligned device memory is allowed
    pub fn can_map_device_aligned(&self) -> bool {
        self.0
            .contains(raw::MemoryState::MAP_DEVICE_ALIGNED_ALLOWED)
    }

    /// Returns whether this memory region is pool allocated
    pub fn is_pool_allocated(&self) -> bool {
        self.0.contains(raw::MemoryState::IS_POOL_ALLOCATED)
    }

    /// Returns whether this memory region is reference counted
    pub fn is_ref_counted(&self) -> bool {
        self.0.contains(raw::MemoryState::IS_REF_COUNTED)
    }

    /// Returns whether mapping process memory is allowed
    pub fn can_map_process(&self) -> bool {
        self.0.contains(raw::MemoryState::MAP_PROCESS_ALLOWED)
    }

    /// Returns whether changing attributes is allowed
    pub fn can_change_attributes(&self) -> bool {
        self.0.contains(raw::MemoryState::ATTR_CHANGE_ALLOWED)
    }

    /// Returns whether code memory operations are allowed
    pub fn can_code_memory(&self) -> bool {
        self.0.contains(raw::MemoryState::CODE_MEM_ALLOWED)
    }
}

/// Memory type of a memory region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Unmapped memory
    Unmapped,
    /// IO memory mapped by kernel capability parsing
    Io,
    /// Normal memory mapped by kernel capability parsing
    Normal,
    /// Static code memory mapped during process creation
    CodeStatic,
    /// Mutable code memory, transitioned from CodeStatic
    CodeMutable,
    /// Heap memory mapped using set_heap_size
    Heap,
    /// Shared memory mapped using map_shared_memory
    SharedMem,
    /// Memory mapped using map_memory
    WeirdMappedMem,
    /// Static module code mapped using map_process_code_memory
    ModuleCodeStatic,
    /// Mutable module code, transitioned from ModuleCodeStatic
    ModuleCodeMutable,
    /// IPC buffer with descriptor flags=0
    IpcBuffer0,
    /// Memory mapped using map_memory
    MappedMemory,
    /// Thread local memory mapped during thread creation
    ThreadLocal,
    /// Isolated transfer memory mapped using map_transfer_memory
    TransferMemIsolated,
    /// Transfer memory mapped using map_transfer_memory
    TransferMem,
    /// Process memory mapped using map_process_memory
    ProcessMem,
    /// Reserved memory
    Reserved,
    /// IPC buffer with descriptor flags=1
    IpcBuffer1,
    /// IPC buffer with descriptor flags=3
    IpcBuffer3,
    /// Kernel stack mapped during thread creation
    KernelStack,
    /// Read-only code memory mapped during control_code_memory
    CodeReadOnly,
    /// Writable code memory mapped during control_code_memory
    CodeWritable,
    /// Coverage memory (not available)
    Coverage,
    /// Insecure memory mapped during map_insecure_physical_memory
    Insecure,
}

impl From<raw::MemoryType> for MemoryType {
    fn from(value: raw::MemoryType) -> Self {
        match value {
            raw::MemoryType::Unmapped => MemoryType::Unmapped,
            raw::MemoryType::Io => MemoryType::Io,
            raw::MemoryType::Normal => MemoryType::Normal,
            raw::MemoryType::CodeStatic => MemoryType::CodeStatic,
            raw::MemoryType::CodeMutable => MemoryType::CodeMutable,
            raw::MemoryType::Heap => MemoryType::Heap,
            raw::MemoryType::SharedMem => MemoryType::SharedMem,
            raw::MemoryType::WeirdMappedMem => MemoryType::WeirdMappedMem,
            raw::MemoryType::ModuleCodeStatic => MemoryType::ModuleCodeStatic,
            raw::MemoryType::ModuleCodeMutable => MemoryType::ModuleCodeMutable,
            raw::MemoryType::IpcBuffer0 => MemoryType::IpcBuffer0,
            raw::MemoryType::MappedMemory => MemoryType::MappedMemory,
            raw::MemoryType::ThreadLocal => MemoryType::ThreadLocal,
            raw::MemoryType::TransferMemIsolated => MemoryType::TransferMemIsolated,
            raw::MemoryType::TransferMem => MemoryType::TransferMem,
            raw::MemoryType::ProcessMem => MemoryType::ProcessMem,
            raw::MemoryType::Reserved => MemoryType::Reserved,
            raw::MemoryType::IpcBuffer1 => MemoryType::IpcBuffer1,
            raw::MemoryType::IpcBuffer3 => MemoryType::IpcBuffer3,
            raw::MemoryType::KernelStack => MemoryType::KernelStack,
            raw::MemoryType::CodeReadOnly => MemoryType::CodeReadOnly,
            raw::MemoryType::CodeWritable => MemoryType::CodeWritable,
            raw::MemoryType::Coverage => MemoryType::Coverage,
            raw::MemoryType::Insecure => MemoryType::Insecure,
        }
    }
}

impl From<MemoryType> for raw::MemoryType {
    fn from(value: MemoryType) -> Self {
        match value {
            MemoryType::Unmapped => raw::MemoryType::Unmapped,
            MemoryType::Io => raw::MemoryType::Io,
            MemoryType::Normal => raw::MemoryType::Normal,
            MemoryType::CodeStatic => raw::MemoryType::CodeStatic,
            MemoryType::CodeMutable => raw::MemoryType::CodeMutable,
            MemoryType::Heap => raw::MemoryType::Heap,
            MemoryType::SharedMem => raw::MemoryType::SharedMem,
            MemoryType::WeirdMappedMem => raw::MemoryType::WeirdMappedMem,
            MemoryType::ModuleCodeStatic => raw::MemoryType::ModuleCodeStatic,
            MemoryType::ModuleCodeMutable => raw::MemoryType::ModuleCodeMutable,
            MemoryType::IpcBuffer0 => raw::MemoryType::IpcBuffer0,
            MemoryType::MappedMemory => raw::MemoryType::MappedMemory,
            MemoryType::ThreadLocal => raw::MemoryType::ThreadLocal,
            MemoryType::TransferMemIsolated => raw::MemoryType::TransferMemIsolated,
            MemoryType::TransferMem => raw::MemoryType::TransferMem,
            MemoryType::ProcessMem => raw::MemoryType::ProcessMem,
            MemoryType::Reserved => raw::MemoryType::Reserved,
            MemoryType::IpcBuffer1 => raw::MemoryType::IpcBuffer1,
            MemoryType::IpcBuffer3 => raw::MemoryType::IpcBuffer3,
            MemoryType::KernelStack => raw::MemoryType::KernelStack,
            MemoryType::CodeReadOnly => raw::MemoryType::CodeReadOnly,
            MemoryType::CodeWritable => raw::MemoryType::CodeWritable,
            MemoryType::Coverage => raw::MemoryType::Coverage,
            MemoryType::Insecure => raw::MemoryType::Insecure,
        }
    }
}

/// Memory attributes that describe memory region properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MemoryAttribute(raw::MemoryAttribute);

impl MemoryAttribute {
    /// Returns whether this memory region is uncached
    pub fn is_uncached(&self) -> bool {
        self.0.contains(raw::MemoryAttribute::IS_UNCACHED)
    }

    /// Returns whether this memory region is borrowed
    pub fn is_borrowed(&self) -> bool {
        self.0.contains(raw::MemoryAttribute::IS_BORROWED)
    }

    /// Returns whether this memory region is IPC mapped
    pub fn is_ipc_mapped(&self) -> bool {
        self.0.contains(raw::MemoryAttribute::IS_IPC_MAPPED)
    }

    /// Returns whether this memory region is device mapped
    pub fn is_device_mapped(&self) -> bool {
        self.0.contains(raw::MemoryAttribute::IS_DEVICE_MAPPED)
    }

    /// Returns whether this memory region is permission locked
    pub fn is_permission_locked(&self) -> bool {
        self.0.contains(raw::MemoryAttribute::IS_PERMISSION_LOCKED)
    }
}
/// Memory permissions for a memory region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MemoryPermission(raw::MemoryPermission);

impl MemoryPermission {
    /// Returns whether this permission includes read access
    pub fn is_readable(&self) -> bool {
        self.0.contains(raw::MemoryPermission::R)
    }

    /// Returns whether this permission includes write access
    pub fn is_writable(&self) -> bool {
        self.0.contains(raw::MemoryPermission::W)
    }

    /// Returns whether this permission includes execute access
    pub fn is_executable(&self) -> bool {
        self.0.contains(raw::MemoryPermission::X)
    }

    /// Returns whether this permission has no access rights
    pub fn is_none(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns whether this permission has read/write access
    pub fn is_read_write(&self) -> bool {
        self.0.contains(raw::MemoryPermission::RW)
    }

    /// Returns whether this permission has read/execute access
    pub fn is_read_execute(&self) -> bool {
        self.0.contains(raw::MemoryPermission::RX)
    }

    /// Returns whether this permission is set to don't care
    pub fn is_dont_care(&self) -> bool {
        self.0.contains(raw::MemoryPermission::DONT_CARE)
    }
}

impl From<MemoryPermission> for raw::MemoryPermission {
    fn from(perm: MemoryPermission) -> Self {
        perm.0
    }
}
