use core::{ffi::c_void, ptr};

pub use crate::raw::{Handle, INVALID_HANDLE};
use crate::{
    error::{KernelError as KError, ResultCode, ToRawResultCode},
    raw,
    result::{Error, raw::Result as RawResult},
};

/// Page information
pub type PageInfo = u32;

/// Sets the process heap to a given size.
///
/// It can extend and shrink the heap.
///
/// Returns the address of the heap (randomized and fixed by the kernel) if the heap was
/// successfully set, or a [`SetHeapSizeError`] on failure.
pub fn set_heap_size(size: usize) -> Result<*mut c_void, SetHeapSizeError> {
    let mut addr = ptr::null_mut();
    let rc = unsafe { raw::set_heap_size(&mut addr, size) };
    RawResult::from_raw(rc).map(addr, |rc| match rc.description() {
        desc if KError::InvalidSize == desc => SetHeapSizeError::InvalidSize,
        desc if KError::OutOfResource == desc => SetHeapSizeError::OutOfResource,
        desc if KError::OutOfMemory == desc => SetHeapSizeError::OutOfMemory,
        desc if KError::InvalidCurrentMemory == desc => SetHeapSizeError::InvalidCurrentMemory,
        desc if KError::InvalidNewMemoryPermission == desc => {
            SetHeapSizeError::InvalidNewMemoryPermission
        }
        desc if KError::InvalidMemoryRegion == desc => SetHeapSizeError::InvalidMemoryRegion,
        desc if KError::InvalidState == desc => SetHeapSizeError::InvalidState,
        desc if KError::LimitReached == desc => SetHeapSizeError::LimitReached,
        _ => SetHeapSizeError::Unknown(rc.into()),
    })
}

/// Error type for set_heap_size operations.
#[derive(Debug, thiserror::Error)]
pub enum SetHeapSizeError {
    /// The size parameter is invalid.
    ///
    /// This occurs when:
    /// - The size is not aligned to 4KB
    /// - The size is 0
    /// - The size would cause an overflow
    #[error("Invalid size")]
    InvalidSize,

    /// System resources are exhausted.
    ///
    /// This occurs when:
    /// - The system has no more physical memory available
    /// - The system has no more virtual memory available
    #[error("Out of resource")]
    OutOfResource,

    /// Not enough memory available.
    ///
    /// This occurs when:
    /// - The process has reached its memory limit
    /// - The system cannot allocate the requested amount of memory
    #[error("Out of memory")]
    OutOfMemory,

    /// Current memory state is invalid.
    ///
    /// This occurs when:
    /// - The memory region is not in the correct state for heap operations
    /// - The memory region is not properly mapped
    /// - The memory region has incorrect permissions
    #[error("Invalid memory state")]
    InvalidCurrentMemory,

    /// Memory permissions are invalid.
    ///
    /// This occurs when:
    /// - The requested permissions are not allowed for heap memory
    /// - The permissions would conflict with existing memory attributes
    #[error("Invalid memory permission")]
    InvalidNewMemoryPermission,

    /// Memory region is invalid.
    ///
    /// This occurs when:
    /// - The requested size exceeds the maximum heap size
    /// - The requested size exceeds the available heap region
    /// - The operation would exceed the process's memory limit
    #[error("Invalid memory region")]
    InvalidMemoryRegion,

    /// Operation is invalid for current state.
    ///
    /// This occurs when:
    /// - The heap is in an invalid state for the requested operation
    /// - The operation cannot be performed in the current context
    #[error("Invalid state")]
    InvalidState,

    /// Resource limit reached.
    ///
    /// This occurs when:
    /// - The process has reached its resource limit
    /// - The system cannot allocate more resources
    #[error("Resource limit reached")]
    LimitReached,

    /// An unknown error occurred
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for SetHeapSizeError {
    fn to_rc(self) -> ResultCode {
        match self {
            SetHeapSizeError::InvalidSize => KError::InvalidSize.to_rc(),
            SetHeapSizeError::OutOfResource => KError::OutOfResource.to_rc(),
            SetHeapSizeError::OutOfMemory => KError::OutOfMemory.to_rc(),
            SetHeapSizeError::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            SetHeapSizeError::InvalidNewMemoryPermission => {
                KError::InvalidNewMemoryPermission.to_rc()
            }
            SetHeapSizeError::InvalidMemoryRegion => KError::InvalidMemoryRegion.to_rc(),
            SetHeapSizeError::InvalidState => KError::InvalidState.to_rc(),
            SetHeapSizeError::LimitReached => KError::LimitReached.to_rc(),
            SetHeapSizeError::Unknown(err) => err.to_raw(),
        }
    }
}

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
/// or a [`QueryMemoryError`] on failure.
pub fn query_memory(addr: usize) -> Result<(MemoryInfo, PageInfo), QueryMemoryError> {
    let mut mem_info = Default::default();
    let mut page_info = Default::default();

    let rc = unsafe { raw::query_memory(&mut mem_info, &mut page_info, addr) };
    RawResult::from_raw(rc).map((mem_info.into(), page_info), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => QueryMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => QueryMemoryError::InvalidAddress,
        desc if KError::InvalidCurrentMemory == desc => QueryMemoryError::InvalidCurrentMemory,
        _ => QueryMemoryError::Unknown(rc.into()),
    })
}

/// Error type for query_memory operations.
#[derive(Debug, thiserror::Error)]
pub enum QueryMemoryError {
    /// The process handle is invalid or not found.
    ///
    /// This occurs when trying to query memory from a process that doesn't exist
    /// or when the handle table lookup fails.
    #[error("Invalid handle")]
    InvalidHandle,

    /// The address is invalid or not properly aligned.
    ///
    /// This occurs when:
    /// - The address is not aligned to 4KB
    /// - The address is outside the process's address space
    /// - The address would cause an overflow when used in calculations
    #[error("Invalid address")]
    InvalidAddress,

    /// The memory state is invalid for the operation.
    ///
    /// This occurs when:
    /// - The memory region is not in a valid state for querying
    /// - The memory region is not mapped
    /// - The memory region is not accessible to the current process
    #[error("Invalid memory state")]
    InvalidCurrentMemory,

    /// An unknown error occurred
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for QueryMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidAddress => KError::InvalidAddress.to_rc(),
            Self::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
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
/// Returns `Ok(())` if the memory was successfully unmapped, or a [`UnmapMemoryError`] on failure.
pub fn unmap_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> Result<(), UnmapMemoryError> {
    let rc = unsafe { raw::unmap_memory(dst_addr, src_addr, size) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => UnmapMemoryError::InvalidHandle,
        desc if KError::InvalidAddress == desc => UnmapMemoryError::InvalidAddress,
        desc if KError::InvalidCurrentMemory == desc => UnmapMemoryError::InvalidCurrentMemory,
        desc if KError::InvalidMemoryRegion == desc => UnmapMemoryError::InvalidMemoryRegion,
        _ => UnmapMemoryError::Unknown(rc.into()),
    })
}

/// Error type for unmap_memory operations.
#[derive(Debug, thiserror::Error)]
pub enum UnmapMemoryError {
    /// The process handle is invalid or not found.
    ///
    /// This occurs when trying to unmap memory from a process that doesn't exist
    /// or when the handle table lookup fails.
    #[error("Invalid handle")]
    InvalidHandle,

    /// The memory address is invalid or not properly aligned.
    ///
    /// This occurs when either the source or destination address is not aligned to 4KB,
    /// or when the address range would cause an overflow.
    #[error("Invalid address")]
    InvalidAddress,

    /// The memory state is invalid for the operation.
    ///
    /// This occurs when:
    /// - The source address range is not within the process's address space
    /// - The address range would cause an overflow (address + size <= address)
    /// - The memory region is not in a valid state for unmapping
    #[error("Invalid memory state")]
    InvalidCurrentMemory,

    /// The memory range is invalid for the operation.
    ///
    /// This occurs when:
    /// - The destination is outside the stack region
    /// - The destination is inside the heap region
    /// - The destination is inside the alias region
    #[error("Invalid memory range")]
    InvalidMemoryRegion,

    /// An unknown error occurred
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for UnmapMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            UnmapMemoryError::InvalidHandle => KError::InvalidHandle.to_rc(),
            UnmapMemoryError::InvalidAddress => KError::InvalidAddress.to_rc(),
            UnmapMemoryError::InvalidCurrentMemory => KError::InvalidCurrentMemory.to_rc(),
            UnmapMemoryError::InvalidMemoryRegion => KError::InvalidMemoryRegion.to_rc(),
            UnmapMemoryError::Unknown(err) => err.to_raw(),
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
