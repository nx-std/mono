//! Raw _Supervisor Call (SVC)_ API.

use core::ffi::{c_char, c_int, c_void};

use bitflags::bitflags;

use crate::{code::*, result::ResultCode};

//<editor-fold desc="Types and Constants">

/// A raw handle type.
///
/// Alias for `u32`.
pub type Handle = u32;

/// Invalid handle
pub const INVALID_HANDLE: Handle = 0;

/// Pseudo handle for the current thread
pub const CUR_THREAD_HANDLE: Handle = 0xFFFF8000;

/// Pseudo handle for the current process
pub const CUR_PROCESS_HANDLE: Handle = 0xFFFF8001;

/// Process Activity
#[repr(u32)]
pub enum ProcessActivity {
    /// Process can run
    Runnable = 0,
    /// Process is paused
    Paused = 1,
}

/// Thread Activity
#[repr(u32)]
pub enum ThreadActivity {
    /// Thread can run
    Runnable = 0,
    /// Thread is paused
    Paused = 1,
}

/// SignalToAddress behaviors
#[repr(u32)]
pub enum SignalType {
    /// Signals the address
    Signal = 0,
    /// Signals the address and increments its value if equal to the argument
    SignalAndIncrementIfEqual = 1,
    /// Signals the address and updates its value if equal to the argument
    SignalAndModifyBasedOnWaitingThreadCountIfEqual = 2,
}

/// Memory mapping type
#[repr(u32)]
pub enum MemoryMapping {
    /// Mapping IO registers
    IoRegister = 0,
    /// Mapping normal memory without cache
    Uncached = 1,
    /// Mapping normal memory
    Memory = 2,
}

/// Io Pools
#[repr(u32)]
pub enum IoPoolType {
    /// Physical address range `0x12000000`-`0x1FFFFFFF`
    PcieA2 = 0,
}

/// Yielding types
///
/// Ref: <https://switchbrew.org/wiki/SVC#SleepThread>
#[repr(i64)]
pub enum YieldType {
    /// Yielding without core migration
    NoMigration = 0,
    /// Yielding with core migration
    WithMigration = -1,
    /// Yielding to any other thread
    ToAnyThread = -2,
}

/// WaitForAddress behaviors
#[repr(u32)]
pub enum ArbitrationType {
    /// Wait if the 32-bit value is less than argument
    WaitIfLessThan = 0,
    /// Decrement the 32-bit value and wait if it is less than argument
    DecrementAndWaitIfLessThan = 1,
    /// Wait if the 32-bit value is equal to argument
    WaitIfEqual = 2,
    /// [19.0.0+] Wait if the 64-bit value is equal to argument
    WaitIfEqual64 = 3,
}

/// Code memory mapping operations
#[repr(u32)]
pub enum CodeMapOperation {
    /// Map owner
    MapOwner = 0,
    /// Map slave
    MapSlave = 1,
    /// Unmap owner
    UnmapOwner = 2,
    /// Unmap slave
    UnmapSlave = 3,
}

/// Process Information
#[repr(u32)]
pub enum ProcessInfoType {
    /// What state is a process in
    ProcessState = 0,
}

/// Debug Thread Parameters
#[repr(u32)]
pub enum DebugThreadParam {
    /// Actual priority of the thread
    ActualPriority = 0,
    /// State of the thread
    State = 1,
    /// Ideal core for the thread
    IdealCore = 2,
    /// Current core the thread is running on
    CurrentCore = 3,
    /// Core mask of the thread
    CoreMask = 4,
}

/// Break reasons
#[repr(u32)]
pub enum BreakReason {
    /// Panic
    Panic = 0,
    /// Assert
    Assert = 1,
    /// User
    User = 2,
    /// PreLoadDll
    PreLoadDll = 3,
    /// PostLoadDll
    PostLoadDll = 4,
    /// PreUnloadDll
    PreUnloadDll = 5,
    /// PostUnloadDll
    PostUnloadDll = 6,
    /// CppException
    CppException = 7,

    /// NotificationOnlyFlag
    NotificationOnlyFlag = 0x80000000,
}

/// Limitable Resources
#[repr(u32)]
pub enum LimitableResource {
    /// How much memory can a process map
    Memory = 0,
    /// How many threads can a process spawn
    Threads = 1,
    /// How many events can a process have
    Events = 2,
    /// How many transfer memories can a process make
    TransferMemories = 3,
    /// How many sessions can a process own
    Sessions = 4,
}

/// Memory information structure
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct MemoryInfo {
    /// Base address
    pub addr: usize,
    /// Size
    pub size: usize,
    /// Memory state and type.
    ///
    /// The upper 24 bits hold the [`MemoryState`] flags.
    /// The lower 8 bits indicate the [`MemoryType`].
    pub typ: u32,
    /// Memory attributes (see [`MemoryAttribute`])
    pub attr: u32,
    /// Memory permissions
    pub perm: u32,
    /// IPC reference count
    pub ipc_refcount: u32,
    /// Device reference count
    pub device_refcount: u32,
    /// Padding (for alignment)
    _pad: u32,
}

/// Bitmask for the `typ` field in [`MemoryInfo`]
pub const MEMORY_TYPE_MASK: u32 = 0xFF;

bitflags! {
    /// Memory state flags (upper bits of type field)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct MemoryState: u32 {
        /// Permission change allowed
        const PERM_CHANGE_ALLOWED = 1 << 8;
        /// Force read/writable by debug syscalls
        const FORCE_RW_BY_DEBUG_SYSCALLS = 1 << 9;
        /// IPC type 0 send allowed
        const IPC_SEND_ALLOWED_TYPE0 = 1 << 10;
        /// IPC type 3 send allowed
        const IPC_SEND_ALLOWED_TYPE3 = 1 << 11;
        /// IPC type 1 send allowed
        const IPC_SEND_ALLOWED_TYPE1 = 1 << 12;
        /// Process permission change allowed
        const PROCESS_PERM_CHANGE_ALLOWED = 1 << 14;
        /// Map allowed
        const MAP_ALLOWED = 1 << 15;
        /// Unmap process code memory allowed
        const UNMAP_PROCESS_CODE_MEM_ALLOWED = 1 << 16;
        /// Transfer memory allowed
        const TRANSFER_MEM_ALLOWED = 1 << 17;
        /// Query physical address allowed
        const QUERY_PADDR_ALLOWED = 1 << 18;
        /// Map device allowed ([`map_device_address_space`] and [`map_device_address_space_by_force`])
        const MAP_DEVICE_ALLOWED = 1 << 19;
        /// Map device aligned allowed
        const MAP_DEVICE_ALIGNED_ALLOWED = 1 << 20;
        /// IPC buffer allowed
        const IPC_BUFFER_ALLOWED = 1 << 21;
        /// Is pool allocated
        const IS_POOL_ALLOCATED = 1 << 22;
        /// Alias for [`IS_POOL_ALLOCATED`]
        const IS_REF_COUNTED = 1 << 22;
        /// Map process allowed
        const MAP_PROCESS_ALLOWED = 1 << 23;
        /// Attribute change allowed
        const ATTR_CHANGE_ALLOWED = 1 << 24;
        /// Code memory allowed
        const CODE_MEM_ALLOWED = 1 << 25;
    }
}

/// Memory type enum (lower 8 bits of [`MemoryState])
#[repr(u8)]
pub enum MemoryType {
    /// Unmapped memory
    Unmapped = 0x00,
    /// Mapped by kernel capability parsing in [`create_process`]
    Io = 0x01,
    /// Mapped by kernel capability parsing in [`create_process`]
    Normal = 0x02,
    /// Mapped during [`create_process`]
    CodeStatic = 0x03,
    /// Transition from CodeStatic performed by [`set_process_memory_permission`]
    CodeMutable = 0x04,
    /// Mapped using [`set_heap_size`]
    Heap = 0x05,
    /// Mapped using [`map_shared_memory`]
    SharedMem = 0x06,
    /// Mapped using [`map_memory`]
    WeirdMappedMem = 0x07,
    /// Mapped using [`map_process_code_memory`]
    ModuleCodeStatic = 0x08,
    /// Transition from ModuleCodeStatic performed by [`set_process_memory_permission`]
    ModuleCodeMutable = 0x09,
    /// IPC buffers with descriptor `flags=0`
    IpcBuffer0 = 0x0A,
    /// Mapped using [`map_memory`]
    MappedMemory = 0x0B,
    /// Mapped during [`create_thread`]
    ThreadLocal = 0x0C,
    /// Mapped using [`map_transfer_memory`] when the owning process has `perm=0`
    TransferMemIsolated = 0x0D,
    /// Mapped using [`map_transfer_memory`] when the owning process has `perm!=0`
    TransferMem = 0x0E,
    /// Mapped using [`map_process_memory`]
    ProcessMem = 0x0F,
    /// Reserved
    Reserved = 0x10,
    /// IPC buffers with descriptor `flags=1`
    IpcBuffer1 = 0x11,
    /// IPC buffers with descriptor `flags=3`
    IpcBuffer3 = 0x12,
    /// Mapped in kernel during [`create_thread`]
    KernelStack = 0x13,
    /// Mapped in kernel during [`control_code_memory`]
    CodeReadOnly = 0x14,
    /// Mapped in kernel during [`control_code_memory`]
    CodeWritable = 0x15,
    /// Not available
    Coverage = 0x16,
    /// Mapped in kernel during [`map_insecure_physical_memory`]
    Insecure = 0x17,
}

bitflags! {
    /// Memory attributes
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    #[repr(transparent)]
    pub struct MemoryAttribute: u32 {
        /// Is borrowed memory
        const IS_BORROWED = 1 << 0;
        /// Is IPC mapped (when IpcRefCount > 0)
        const IS_IPC_MAPPED = 1 << 1;
        /// Is device mapped (when DeviceRefCount > 0)
        const IS_DEVICE_MAPPED = 1 << 2;
        /// Is uncached
        const IS_UNCACHED = 1 << 3;
        /// Is permission locked
        const IS_PERMISSION_LOCKED = 1 << 4;
    }
}

/// Physical memory information structure
#[repr(C)]
pub struct PhysicalMemoryInfo {
    /// Physical address.
    pub physical_address: u64,
    /// Virtual address.
    pub virtual_address: u64,
    /// Size.
    pub size: u64,
}

/// Thread context structure (register dump)
#[repr(C)]
pub struct ThreadContext {
    /// GPRs 0..28. Note: also contains AArch32 SPRs.
    pub cpu_gprs: [CpuRegister; 29],
    /// Frame pointer (x29) (AArch64). For AArch32, check r11
    pub fp: u64,
    /// Link register (x30) (AArch64). For AArch32, check r14
    pub lr: u64,
    /// Stack pointer (AArch64). For AArch32, check r13
    pub sp: u64,
    /// Program counter
    pub pc: CpuRegister,
    /// PSTATE or cpsr
    pub psr: u32,
    /// 32 general-purpose NEON registers
    pub fpu_gprs: [FpuRegister; 32],
    /// Floating-point control register
    pub fpcr: u32,
    /// Floating-point status register
    pub fpsr: u32,
    /// EL0 Read/Write Software Thread ID Register
    pub tpidr: u64,
}

impl ThreadContext {
    /// Create a new thread context with all registers set to zero.
    pub fn zeroed() -> Self {
        // SAFETY: This is safe because the thread context is a POD type.
        unsafe { core::mem::zeroed() }
    }

    /// Determines whether a thread context belong to an AArch64 process based on the PSR.
    ///
    /// Returns true if and only if the thread context belongs to an AArch64 process.
    pub fn is_aarch64(&self) -> bool {
        (self.psr & 0x10) == 0
    }
}

/// Armv8 CPU register
#[repr(C)]
pub union CpuRegister {
    /// 64-bit AArch64 register view
    pub x: u64,
    /// 32-bit AArch64 register view
    pub w: u32,
    /// AArch32 register view
    pub r: u32,
}

/// Armv8 NEON register
#[repr(C)]
pub union FpuRegister {
    /// 128-bit vector view
    pub v: u128,
    /// 64-bit double-precision view
    pub d: f64,
    /// 32-bit single-precision view
    pub s: f32,
}

/// Context of a scheduled thread
#[repr(C)]
pub struct LastThreadContext {
    /// Frame Pointer for the thread
    pub fp: u64,
    /// Stack Pointer for the thread
    pub sp: u64,
    /// Link Register for the thread
    pub lr: u64,
    /// Program Counter for the thread
    pub pc: u64,
}

/// Secure monitor arguments
#[repr(C, packed)]
pub struct SecmonArgs {
    /// Values of X0 through X7
    pub x: [u64; 8],
}

//</editor-fold>

//<editor-fold desc="Memory management">

/// Set the process heap to a given size. It can both extend and shrink the heap.
///
/// `Result svcSetHeapSize(void** out_addr, size_t size);`
///
/// Syscall code: [SET_HEAP_SIZE](crate::code::SET_HEAP_SIZE) (`0x1`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_addr_ | Variable to which write the address of the heap (which is randomized and fixed by the kernel) |
/// | IN | _size_ | Size of the heap, must be a multiple of 0x200000 and [2.0.0+] less than 0x18000000. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetHeapSize>
///
/// # Safety
///
/// The caller must ensure that `out_addr` is a valid, aligned pointer to writable memory
/// where the heap address can be written.
#[unsafe(naked)]
pub unsafe extern "C" fn set_heap_size(out_addr: *mut *mut c_void, size: usize) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out_addr) on the stack
        "svc {code}",          // Issue the SVC call with immediate value 0x1
        "ldr x2, [sp], #16",   // Load x2 (out_addr pointer) from the stack
        "str x1, [x2]",        // Store x1 in the memory pointed to by x2
        "ret",
        code = const SET_HEAP_SIZE,
    );
}

/// Set the memory permissions of a (page-aligned) range of memory.
///
/// `Result svcSetMemoryPermission(void* addr, uint64_t size, uint32_t perm);`
///
/// Syscall code: [SET_MEMORY_PERMISSION](crate::code::SET_MEMORY_PERMISSION) (`0x2`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _addr_ | Start address of the range. |
/// | IN | _size_ | Size of the range, in bytes. |
/// | IN | _perm_ | Memory permissions (as u32 bitflags: R=1, W=2, X=4, DONT_CARE=1<<28). |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetMemoryPermission>
///
/// # Safety
///
/// The caller must ensure that `addr` points to a valid, page-aligned memory range
/// that the current process owns.
#[unsafe(naked)]
pub unsafe extern "C" fn set_memory_permission(
    addr: *mut c_void,
    size: usize,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}",          // Issue the SVC call with immediate value 0x2
        "ret",
        code = const SET_MEMORY_PERMISSION,
    );
}

/// Set the memory attributes of a (page-aligned) range of memory.
///
/// `Result svcSetMemoryAttribute(void* addr, uint64_t size, uint32_t mask, uint32_t attr);`
///
/// Syscall code: [SET_MEMORY_ATTRIBUTE](crate::code::SET_MEMORY_ATTRIBUTE) (`0x3`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _addr_ | Start address of the range. |
/// | IN | _size_ | Size of the range, in bytes. |
/// | IN | _mask_ | Mask of the attributes to change. |
/// | IN | _attr_ | New attributes. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetMemoryAttribute>
///
/// # Safety
///
/// The caller must ensure that `addr` points to a valid, page-aligned memory range
/// that the current process owns.
#[unsafe(naked)]
pub unsafe extern "C" fn set_memory_attribute(
    addr: *mut c_void,
    size: usize,
    mask: u32,
    attr: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x3
        "ret",
        code = const SET_MEMORY_ATTRIBUTE,
    );
}

/// Maps a memory range into a different range. Mainly used for adding guard pages around stack.
///
/// Source range gets reprotected to [Perm_None] (it can no longer be accessed), and
/// [MemAttr_IsBorrowed] is set in the source [MemoryAttribute].
///
/// `Result svcMapMemory(void* dst_addr, void* src_addr, uint64_t size);`
///
/// Syscall code: [MAP_MEMORY](crate::code::MAP_MEMORY) (`0x4`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _dst_addr_ | Destination address. |
/// | IN | _src_addr_ | Source address. |
/// | IN | _size_ | Size of the range. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapMemory>
///
/// # Safety
///
/// The caller must ensure that both `dst_addr` and `src_addr` point to valid,
/// page-aligned memory ranges owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn map_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}",  // Issue the SVC call with immediate value 0x4
        "ret",
        code = const MAP_MEMORY,
    );
}

/// Unmaps a region that was previously mapped with [`map_memory`].
///
/// `Result svcUnmapMemory(void* dst_addr, void* src_addr, uint64_t size);`
///
/// Syscall code: [UNMAP_MEMORY](crate::code::UNMAP_MEMORY) (`0x5`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _dst_addr_ | Destination address. |
/// | IN | _src_addr_ | Source address. |
/// | IN | _size_ | Size of the range. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapMemory>
///
/// # Safety
///
/// The caller must ensure that both `dst_addr` and `src_addr` point to valid,
/// page-aligned memory ranges that were previously mapped with [`map_memory`].
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}",  // Issue the SVC call with immediate value 0x5
        "ret",
        code = const UNMAP_MEMORY,
    );
}

/// Query information about an address. Will always fetch the lowest page-aligned mapping that
/// contains the provided address.
///
/// `Result svcQueryMemory(arch::MemoryInfo *out_memory_info, PageInfo *out_page_info, uint64_t address);`
///
/// Syscall code: [QUERY_MEMORY](crate::code::QUERY_MEMORY) (`0x6`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _meminfo_ptr_ | [MemoryInfo] structure which will be filled in.
/// | OUT | _pageinfo_ | Page information which will be filled in.
/// | IN  | _addr_ | Address to query.
///
/// Ref: <https://switchbrew.org/wiki/SVC#QueryMemory>
///
/// # Safety
///
/// The caller must ensure that both `meminfo` and `pageinfo` are valid, aligned pointers
/// to writable memory where the kernel can write the query results.
#[unsafe(naked)]
pub unsafe extern "C" fn query_memory(
    meminfo: *mut MemoryInfo,
    pageinfo: *mut u32,
    addr: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x1, [sp, #-16]!", // Store x1 (pageinfo pointer) on stack with pre-decrement
        "svc {code}",          // Issue the SVC call with immediate value 0x6
        "ldr x2, [sp], #16",   // Load x2 from stack with post-increment
        "str w1, [x2]",        // Store w1 (page info) to address in x2
        "ret",
        code = const QUERY_MEMORY,
    );
}

//</editor-fold>

//<editor-fold desc="Process and thread management">

/// Exists the current process.
///
/// `void NX_NORETURN svcExitProcess(void);`
///
/// Syscall code: [EXIT_PROCESS](crate::code::EXIT_PROCESS) (`0x7`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#ExitProcess>
///
/// # Safety
///
/// This function never returns and terminates the entire process. No further cleanup occurs
/// after the call.
#[unsafe(naked)]
pub unsafe extern "C" fn exit_process() -> ! {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x7 (ExitProcess) - never returns
        "ret",
        code = const EXIT_PROCESS,
    );
}

/// Creates a thread.
///
/// `Result svcCreateThread(Handle* out, void* entry, void* arg, void* stack_top, int prio, int cpuid);`
///
/// Syscall code: [CREATE_THREAD](crate::code::CREATE_THREAD) (`0x8`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for the created thread. |
/// | IN | _entry_ | Entry function. |
/// | IN | _arg_ | Argument to pass to the entry function. |
/// | IN | _stack_top_ | Top of the stack. |
/// | IN | _prio_ | Priority. |
/// | IN | _cpuid_ | CPU core ID. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateThread>
///
/// # Safety
///
/// The caller must ensure:
/// - `handle` is a valid, aligned pointer to writable memory for the output handle
/// - `entry` points to a valid function with the correct signature
/// - `arg` is valid to pass to the entry function (or null)
/// - `stack_top` points to a valid, page-aligned stack that remains valid for the thread's lifetime
#[unsafe(naked)]
pub unsafe extern "C" fn create_thread(
    handle: *mut Handle,
    entry: *mut c_void,
    arg: *mut c_void,
    stack_top: *mut c_void,
    prio: c_int,
    cpuid: c_int,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack with pre-decrement
        "svc {code}",          // Issue the SVC call with immediate value 0x8
        "ldr x2, [sp], #16",   // Load x2 from stack with post-increment
        "str w1, [x2]",        // Store w1 (thread handle) to address in x2
        "ret",
        code = const CREATE_THREAD,
    );
}

/// Starts a freshly created thread.
///
/// `Result svcStartThread(Handle handle);`
///
/// Syscall code: [START_THREAD](crate::code::START_THREAD) (`0x9`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the thread to start. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#StartThread>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel thread handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn start_thread(handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x9
        "ret",
        code = const START_THREAD,
    );
}

/// Exits the current thread.
///
/// `void NX_NORETURN svcExitThread(void);`
///
/// Syscall code: [EXIT_THREAD](crate::code::EXIT_THREAD) (`0xA`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#ExitThread>
///
/// # Safety
///
/// This function never returns and terminates the current thread. Stack and TLS cleanup
/// is performed by the kernel.
#[unsafe(naked)]
pub unsafe extern "C" fn exit_thread() -> ! {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0xA - never returns
        "ret",
        code = const EXIT_THREAD,
    );
}

/// Sleeps the current thread for the specified amount of time.
///
/// Setting nanoseconds to 0, -1, or -2 indicates a [YieldType].
///
/// `void svcSleepThread(int64_t nano);`
///
/// Syscall code: [SLEEP_THREAD](crate::code::SLEEP_THREAD) (`0xB`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _nano_ | Number of nanoseconds to sleep, or [YieldType] for yield. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SleepThread>
///
/// # Safety
///
/// This function is safe to call from any context. The value passed is used directly
/// by the kernel.
#[unsafe(naked)]
pub unsafe extern "C" fn sleep_thread(nano: i64) {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0xB
        "ret",
        code = const SLEEP_THREAD,
    );
}

/// Gets a thread's priority.
///
/// `Result svcGetThreadPriority(int32_t* priority, Handle handle);`
///
/// Syscall code: [GET_THREAD_PRIORITY](crate::code::GET_THREAD_PRIORITY) (`0xC`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _priority_ | Output priority. |
/// | IN | _handle_ | Handle of the thread to query. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetThreadPriority>
///
/// # Safety
///
/// The caller must ensure:
/// - `priority` is a valid, aligned pointer to writable memory where the priority will be stored
/// - `handle` is a valid kernel thread handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_thread_priority(priority: *mut i32, handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (priority pointer) on stack with pre-decrement
        "svc {code}",          // Issue the SVC call with immediate value 0xC
        "ldr x2, [sp], #16",   // Load x2 from stack with post-increment
        "str w1, [x2]",        // Store w1 (priority) to address in x2
        "ret",
        code = const GET_THREAD_PRIORITY,
    );
}

/// Sets the priority of provided thread handle.
///
/// Priority is a number `0-0x3F`. Lower value means higher priority.
///
/// `Result svcSetThreadPriority(Handle handle, uint32_t priority);`
///
/// Syscall code: [SET_THREAD_PRIORITY](crate::code::SET_THREAD_PRIORITY) (`0xD`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the thread to set the priority of. |
/// | IN | _priority_ | New priority. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetThreadPriority>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel thread handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn set_thread_priority(handle: Handle, priority: u32) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0xD
        "ret",
        code = const SET_THREAD_PRIORITY,
    );
}

/// Gets the affinity mask of provided thread handle.
///
/// `Result svcGetThreadCoreMask(int32_t* core_id, uint64_t* affinity_mask, Handle handle);`
///
/// Syscall code: [GET_THREAD_CORE_MASK](crate::code::GET_THREAD_CORE_MASK) (`0xE`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _core_id_ | Output preferred core. |
/// | OUT | _affinity_mask_ | Output affinity mask. |
/// | IN | _handle_ | Handle of the thread to query. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetThreadCoreMask>
///
/// # Safety
///
/// The caller must ensure:
/// - Both `core_id` and `affinity_mask` are valid, aligned pointers to writable memory
/// - `handle` is a valid kernel thread handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_thread_core_mask(
    core_id: *mut i32,
    affinity_mask: *mut u64,
    handle: Handle,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x0, x1, [sp, #-16]!", // Store x0 (core_id ptr) and x1 (affinity_mask ptr) on stack
        "svc {code}",              // Issue the SVC call with immediate value 0xE
        "ldp x3, x4, [sp], #16",   // Load x3 and x4 from the stack
        "str w1, [x3]",            // Store w1 (core_id value) to address in x3
        "str x2, [x4]",            // Store x2 (affinity_mask value) to address in x4
        "ret",
        code = const GET_THREAD_CORE_MASK,
    );
}

/// Sets the affinity mask of provided thread handle.
///
/// `Result svcSetThreadCoreMask(Handle handle, int32_t core_id, uint32_t affinity_mask);`
///
/// Syscall code: [SET_THREAD_CORE_MASK](crate::code::SET_THREAD_CORE_MASK) (`0xF`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the thread to set the core mask of. |
/// | IN | _core_id_ | New preferred core. |
/// | IN | _affinity_mask_ | New affinity mask. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetThreadCoreMask>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel thread handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn set_thread_core_mask(
    handle: Handle,
    core_id: i32,
    affinity_mask: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0xF
        "ret",
        code = const SET_THREAD_CORE_MASK,
    );
}

/// Gets which CPU is executing the current thread.
///
/// CPU ID is an integer in the range `0-3`.
///
/// `uint32_t svcGetCurrentProcessorNumber(void);`
///
/// Syscall code: [GET_CURRENT_PROCESSOR_NUMBER](crate::code::GET_CURRENT_PROCESSOR_NUMBER) (`0x10`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetCurrentProcessorNumber>
///
/// # Safety
///
/// This function is safe to call from any context.
#[unsafe(naked)]
pub unsafe extern "C" fn get_current_processor_number() -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x10
        "ret",
        code = const GET_CURRENT_PROCESSOR_NUMBER,
    );
}

//</editor-fold>

//<editor-fold desc="Synchronization">

/// Puts the given event in the signaled state.
///
/// Will wake up any thread currently waiting on this event. Can potentially trigger a re-schedule.
///
/// Any calls to [wait_synchronization] on this handle will return immediately, until the
/// event's signaled state is reset.
///
/// `Result svcSignalEvent(Handle handle);`
///
/// Syscall code: [SIGNAL_EVENT](crate::code::SIGNAL_EVENT) (`0x11`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the event to signal. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SignalEvent>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel event handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn signal_event(handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x11
        "ret",
        code = const SIGNAL_EVENT,
    );
}

/// Takes the given event out of the signaled state, if it is signaled.
///
/// `Result svcClearEvent(Handle handle);`
///
/// Syscall code: [CLEAR_EVENT](crate::code::CLEAR_EVENT) (`0x12`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the event to clear. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ClearEvent>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel event handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn clear_event(handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x12
        "ret",
        code = const CLEAR_EVENT,
    );
}

//</editor-fold>

//<editor-fold desc="Inter-process memory sharing">

/// Maps a block of shared memory.
///
/// `Result svcMapSharedMemory(Handle handle, void* addr, size_t size, MemoryPermission perm);`
///
/// Syscall code: [MAP_SHARED_MEMORY](crate::code::MAP_SHARED_MEMORY) (`0x13`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the shared memory block. |
/// | IN | _addr_ | Address to map the block to. |
/// | IN | _size_ | Size of the block. |
/// | IN | _perm_ | Memory permissions (as u32 bitflags: R=1, W=2, X=4, DONT_CARE=1<<28). |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapSharedMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `handle` is a valid kernel shared memory handle owned by the current process
/// - `addr` points to a valid, page-aligned memory range owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn map_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x13
        "ret",
        code = const MAP_SHARED_MEMORY,
    );
}

/// Unmaps a block of shared memory.
///
/// `Result svcUnmapSharedMemory(Handle handle, void* addr, size_t size);`
///
/// Syscall code: [UNMAP_SHARED_MEMORY](crate::code::UNMAP_SHARED_MEMORY) (`0x14`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the shared memory block. |
/// | IN | _addr_ | Address of the block. |
/// | IN | _size_ | Size of the block. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapSharedMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `handle` is a valid kernel shared memory handle owned by the current process
/// - `addr` points to a valid, page-aligned memory range that was previously mapped
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x14
        "ret",
        code = const UNMAP_SHARED_MEMORY,
    );
}

/// Creates a block of transfer memory.
///
/// `Result svcCreateTransferMemory(Handle* handle, void* addr, size_t size, MemoryPermission perm);`
///
/// Syscall code: [CREATE_TRANSFER_MEMORY](crate::code::CREATE_TRANSFER_MEMORY) (`0x15`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for the created transfer memory block. |
/// | IN | _addr_ | Address of the block. |
/// | IN | _size_ | Size of the block. |
/// | IN | _perm_ | Memory permissions (as u32 bitflags: R=1, W=2, X=4, DONT_CARE=1<<28). |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateTransferMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `handle` is a valid, aligned pointer to writable memory for the output handle
/// - `addr` points to a valid, page-aligned memory range owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn create_transfer_memory(
    handle: *mut Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack with pre-decrement
        "svc {code}",          // Issue the SVC call with immediate value 0x15
        "ldr x2, [sp], #16",   // Load x2 from stack with post-increment
        "str w1, [x2]",        // Store w1 (handle value) to address in x2
        "ret",
        code = const CREATE_TRANSFER_MEMORY,
    );
}

//</editor-fold>

//<editor-fold desc="Miscellaneous">

/// Closes a handle, decrementing the reference count of the corresponding kernel object.
///
/// `Result svcCloseHandle(Handle handle);`
///
/// Syscall code: [CLOSE_HANDLE](crate::code::CLOSE_HANDLE) (`0x16`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle to close. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CloseHandle>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn close_handle(handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x16
        "ret",
        code = const CLOSE_HANDLE,
    );
}

//</editor-fold>

//<editor-fold desc="Synchronization">

/// Resets a signal.
///
/// `Result svcResetSignal(Handle handle);`
///
/// Syscall code: [RESET_SIGNAL](crate::code::RESET_SIGNAL) (`0x17`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle of the signal to reset. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ResetSignal>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel synchronization handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn reset_signal(handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x17
        "ret",
        code = const RESET_SIGNAL,
    );
}

/// Waits on one or more synchronization objects, optionally with a timeout.
///
/// Works with HandlesNum <= 0x40.
///
/// When zero handles are passed, this will wait forever until either timeout or cancellation
/// occurs.
///
/// Does not accept [CUR_THREAD_HANDLE] (`0xFFFF8000`) or [CUR_PROCESS_HANDLE] (`0xFFFF8001`) as
/// handles.
///
/// `Result svcWaitSynchronization(int32_t* index, const Handle* handles, int32_t handleCount, uint64_t timeout);`
///
/// Syscall code: [WAIT_SYNCHRONIZATION](crate::code::WAIT_SYNCHRONIZATION) (`0x18`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _index_ | Pointer to store the index of the object that was signaled. |
/// | IN | _handles_ | Pointer to an array of handles to wait on. |
/// | IN | _handle_count_ | Number of handles to wait on. |
/// | IN | _timeout_ | Timeout in nanoseconds. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#WaitSynchronization>
///
/// # Safety
///
/// The caller must ensure:
/// - `index` is a valid, aligned pointer to writable memory for the result index
/// - `handles` points to a valid array of `handle_count` kernel handles owned by the current process
/// - All handles in the array are valid and not pseudo-handles
#[unsafe(naked)]
pub unsafe extern "C" fn wait_synchronization(
    index: *mut i32,
    handles: *const u32,
    handle_count: i32,
    timeout: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (index pointer) on stack
        "svc {code}",          // Issue the SVC call with immediate value 0x18
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (index result) to address in x2
        "ret",
        code = const WAIT_SYNCHRONIZATION,
    );
}

/// Waits a [wait_synchronization] operation being done on a synchronization object in
/// another thread.
///
/// If the referenced thread is currently in a synchronization call ([wait_synchronization],
/// [reply_and_receive] or [reply_and_receive_light]), that call will be
/// interrupted and return `0xec01`. If that thread is not currently executing such a synchronization
/// call, the next call to a synchronization call will return `0xec01`.
///
/// This doesn't take force-pause (activity/debug pause) into account.
///
/// Syscall code: [CANCEL_SYNCHRONIZATION](crate::code::CANCEL_SYNCHRONIZATION) (`0x19`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle to the thread to wait for. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CancelSynchronization>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel thread handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn cancel_synchronization(handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x19
        "ret",
        code = const CANCEL_SYNCHRONIZATION,
    );
}

/// Arbitrates a mutex lock operation in userspace.
///
/// `Result svcArbitrateLock(u32 wait_tag, uint32_t* mutex, uint32_t self_tag);`
///
/// Syscall code: [ARBITRATE_LOCK](crate::code::ARBITRATE_LOCK) (`0x1A`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _owner_thread_handle_ | The owner thread's kernel handle. |
/// | IN | _mutex_ | The mutex raw tag value. |
/// | IN | _curr_thread_handle_ | The current thread's kernel handle. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ArbitrateLock>
///
/// # Safety
///
/// The caller must ensure:
/// - Both `owner_thread_handle` and `curr_thread_handle` are valid kernel thread handles
/// - `mutex` points to a valid u32 value
#[unsafe(naked)]
pub unsafe extern "C" fn arbitrate_lock(
    owner_thread_handle: Handle,
    mutex: *mut u32,
    curr_thread_handle: Handle,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x1A
        "ret",
        code = const ARBITRATE_LOCK,
    );
}

/// Arbitrates a mutex unlock operation in userspace.
///
/// `Result svcArbitrateUnlock(uint32_t* mutex);`
///
/// Syscall code: [ARBITRATE_UNLOCK](crate::code::ARBITRATE_UNLOCK) (`0x1B`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _mutex_ | The mutex raw tag value. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ArbitrateUnlock>
///
/// # Safety
///
/// The caller must ensure that `mutex` points to a valid u32 value.
#[unsafe(naked)]
pub unsafe extern "C" fn arbitrate_unlock(mutex: *mut u32) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x1B
        "ret",
        code = const ARBITRATE_UNLOCK,
    );
}

/// Performs a condition variable wait operation in userspace.
///
/// `Result svcWaitProcessWideKeyAtomic(u32* address, uint32_t* cv_key, uint32_t tag, uint64_t timeout_ns);`
///
/// Syscall code: [WAIT_PROCESS_WIDE_KEY_ATOMIC](crate::code::WAIT_PROCESS_WIDE_KEY_ATOMIC) (`0x1C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Pointer to the address of the condition variable. |
/// | IN | _cv_key_ | Pointer to the key of the condition variable. |
/// | IN | _tag_ | Tag to use for the condition variable. |
/// | IN | _timeout_ns_ | Timeout in nanoseconds. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#WaitProcessWideKeyAtomic>
///
/// # Safety
///
/// The caller must ensure that both `address` and `cv_key` point to valid u32 values.
#[unsafe(naked)]
pub unsafe extern "C" fn wait_process_wide_key_atomic(
    address: *mut u32,
    cv_key: *mut u32,
    tag: u32,
    timeout_ns: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x1C
        "ret",
        code = const WAIT_PROCESS_WIDE_KEY_ATOMIC,
    );
}

/// Performs a condition variable wake-up operation in userspace.
///
/// `Result svcSignalProcessWideKey(uint32_t* cv_key, int32_t count);`
///
/// Syscall code: [SIGNAL_PROCESS_WIDE_KEY](crate::code::SIGNAL_PROCESS_WIDE_KEY) (`0x1D`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _cv_key_ | Pointer to the key of the condition variable. |
/// | IN | _count_ | Number of threads to wake up. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SignalProcessWideKey>
///
/// # Safety
///
/// The caller must ensure that `cv_key` points to a valid u32 value.
#[unsafe(naked)]
pub unsafe extern "C" fn signal_process_wide_key(cv_key: *mut u32, count: i32) {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x1D
        "ret",
        code = const SIGNAL_PROCESS_WIDE_KEY,
    );
}

//</editor-fold>

//<editor-fold desc="Miscellaneous">

/// Gets the current system tick.
///
/// `Result svcGetSystemTick();`
///
/// Syscall code: [GET_SYSTEM_TICK](crate::code::GET_SYSTEM_TICK) (`0x1E`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetSystemTick>
///
/// # Safety
///
/// This function is safe to call from any context.
#[unsafe(naked)]
pub unsafe extern "C" fn get_system_tick() -> u64 {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x1E
        "ret",
        code = const GET_SYSTEM_TICK,
    );
}

//</editor-fold>

//<editor-fold desc="Inter-process communication (IPC)">

/// Connects to a registered named port.
///
/// `Result svcConnectToNamedPort(Handle* session, const char* name);`
///
/// Syscall code: [CONNECT_TO_NAMED_PORT](crate::code::CONNECT_TO_NAMED_PORT) (`0x1F`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _session_ | Pointer to store the session handle. |
/// | IN | _name_ | Pointer to the name of the port. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ConnectToNamedPort>
///
/// # Safety
///
/// The caller must ensure:
/// - `session` is a valid, aligned pointer to writable memory for the output handle
/// - `name` points to a null-terminated C string that is valid and readable
#[unsafe(naked)]
pub unsafe extern "C" fn connect_to_named_port(
    session: *mut Handle,
    name: *const c_char,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (session pointer) on stack
        "svc {code}",          // Issue the SVC call with immediate value 0x1F
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (session handle) to address in x2
        "ret",
        code = const CONNECT_TO_NAMED_PORT,
    );
}

/// Sends a light IPC synchronization request to a session.
///
/// `Result svcSendSyncRequestLight(Handle session);`
///
/// Syscall code: [SEND_SYNC_REQUEST_LIGHT](crate::code::SEND_SYNC_REQUEST_LIGHT) (`0x20`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _session_ | Session handle. |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SendSyncRequestLight>
///
/// # Safety
///
/// The caller must ensure that `session` is a valid kernel session handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn send_sync_request_light(session: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x20
        "ret",
        code = const SEND_SYNC_REQUEST_LIGHT,
    );
}

/// Sends an IPC synchronization request to a session.
///
/// `Result svcSendSyncRequest(Handle session);`
///
/// Syscall code: [SEND_SYNC_REQUEST](crate::code::SEND_SYNC_REQUEST) (`0x21`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _session_ | Session handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SendSyncRequest>
///
/// # Safety
///
/// The caller must ensure that `session` is a valid kernel session handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn send_sync_request(session: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x21
        "ret",
        code = const SEND_SYNC_REQUEST,
    );
}

/// Sends an IPC synchronization request to a session from a user allocated buffer.
///
/// `Result svcSendSyncRequestWithUserBuffer(void* usrBuffer, uint64_t size, Handle session);`
///
/// Syscall code: [SEND_SYNC_REQUEST_WITH_USER_BUFFER](crate::code::SEND_SYNC_REQUEST_WITH_USER_BUFFER) (`0x22`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _usr_buffer_ | User buffer address |
/// | IN | _size_ | User buffer size |
/// | IN | _session_ | Session handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SendSyncRequestWithUserBuffer>
///
/// # Safety
///
/// The caller must ensure:
/// - `usr_buffer` points to valid memory owned by the current process
/// - `session` is a valid kernel session handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn send_sync_request_with_user_buffer(
    usr_buffer: *mut c_void,
    size: u64,
    session: Handle,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x22
        "ret",
        code = const SEND_SYNC_REQUEST_WITH_USER_BUFFER,
    );
}

/// Sends an IPC synchronization request to a session from a user allocated buffer (asynchronous
/// version).
///
/// `Result svcSendAsyncRequestWithUserBuffer(Handle* handle, void* usrBuffer, uint64_t size, Handle session);`
///
/// Syscall code: [SEND_ASYNC_REQUEST_WITH_USER_BUFFER](crate::code::SEND_ASYNC_REQUEST_WITH_USER_BUFFER) (`0x23`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for request |
/// | IN | _usr_buffer_ | User buffer address |
/// | IN | _size_ | User buffer size |
/// | IN | _session_ | Session handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SendAsyncRequestWithUserBuffer>
///
/// # Safety
///
/// The caller must ensure:
/// - `handle` is a valid, aligned pointer to writable memory for the output handle
/// - `usr_buffer` points to valid memory owned by the current process
/// - `session` is a valid kernel session handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn send_async_request_with_user_buffer(
    handle: *mut Handle,
    usr_buffer: *mut c_void,
    size: u64,
    session: Handle,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack
        "svc {code}",          // Issue the SVC call with immediate value 0x23
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret",
        code = const SEND_ASYNC_REQUEST_WITH_USER_BUFFER,
    );
}

//</editor-fold>

//<editor-fold desc="Process and thread management">

/// Gets the PID associated with a process.
///
/// `Result svcGetProcessId(uint64_t *processID, Handle handle);`
///
/// Syscall code: [GET_PROCESS_ID](crate::code::GET_PROCESS_ID) (`0x24`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _process_id_ | Variable to which write the process ID |
/// | IN | _handle_ | Handle of the process to get the PID from |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetProcessId>
///
/// # Safety
///
/// The caller must ensure:
/// - `process_id` is a valid, aligned pointer to writable memory where the ID will be stored
/// - `handle` is a valid kernel process handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_process_id(process_id: *mut u64, handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (process_id pointer) on stack
        "svc {code}",          // Issue the SVC call with immediate value 0x24
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (process ID result) to address in x2
        "ret",
        code = const GET_PROCESS_ID,
    );
}

/// Gets the TID associated with a process.
///
/// `Result svcGetThreadId(uint64_t *threadID, Handle handle);`
///
/// Syscall code: [GET_THREAD_ID](crate::code::GET_THREAD_ID) (`0x25`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _thread_id_ | Variable to which write the thread ID |
/// | IN | _handle_ | Handle of the thread to get the TID from |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetThreadId>
///
/// # Safety
///
/// The caller must ensure:
/// - `thread_id` is a valid, aligned pointer to writable memory where the ID will be stored
/// - `handle` is a valid kernel thread handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_thread_id(thread_id: *mut u64, handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (thread_id pointer) on stack
        "svc {code}",          // Issue the SVC call with immediate value 0x25
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (thread ID result) to address in x2
        "ret",
        code = const GET_THREAD_ID,
    );
}

//</editor-fold>

//<editor-fold desc="Miscellaneous">

/// Breaks execution.
///
/// `Result svcBreak(BreakReason reason, uintptr_t address, uintptr_t size);`
///
/// Syscall code: [BREAK](crate::code::BREAK) (`0x26`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _reason_ | Break reason (see [BreakReason]) |
/// | IN | _address_ | Address of the buffer to pass to the debugger |
/// | IN | _size_ | Size of the buffer to pass to the debugger |
///
/// Ref: <https://switchbrew.org/wiki/SVC#Break>
///
/// # Safety
///
/// If debugging is active, `address` should point to valid readable memory for the debugger.
#[unsafe(naked)]
pub unsafe extern "C" fn r#break(reason: BreakReason, address: usize, size: usize) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x26
        "ret",
        code = const BREAK,
    );
}

//</editor-fold>

//<editor-fold desc="Debugging">

/// Outputs debug text, if used during debugging.
///
/// `Result svcOutputDebugString(const char *str, uint64_t size);`
///
/// Syscall code: [OUTPUT_DEBUG_STRING](crate::code::OUTPUT_DEBUG_STRING) (`0x27`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _dbg_str_ | Text to output |
/// | IN | _size_ | Size of the text in bytes |
///
/// Ref: <https://switchbrew.org/wiki/SVC#OutputDebugString>
///
/// # Safety
///
/// The caller must ensure that `dbg_str` points to readable memory of at least `size` bytes.
#[unsafe(naked)]
pub unsafe extern "C" fn output_debug_string(dbg_str: *const c_char, size: u64) -> ResultCode {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x27
        "ret",
        code = const OUTPUT_DEBUG_STRING,
    );
}

//</editor-fold>

//<editor-fold desc="Miscellaneous">

/// Returns from an exception.
///
/// `void svcReturnFromException(Result res);`
///
/// Syscall code: [RETURN_FROM_EXCEPTION](crate::code::RETURN_FROM_EXCEPTION) (`0x28`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _res_ | Result code |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ReturnFromException>
///
/// # Safety
///
/// This function never returns. It must only be called from an exception handler.
#[unsafe(naked)]
pub unsafe extern "C" fn return_from_exception(res: ResultCode) -> ! {
    core::arch::naked_asm!(
        "svc {code}", // Issue the SVC call with immediate value 0x28 - never returns
        "ret",
        code = const RETURN_FROM_EXCEPTION,
    );
}

/// Information types for [`get_info`]
#[repr(u32)]
pub enum InfoTypeId0 {
    CoreMask = 0,
    PriorityMask = 1,
    AliasRegionAddress = 2,
    AliasRegionSize = 3,
    HeapRegionAddress = 4,
    HeapRegionSize = 5,
    TotalMemorySize = 6,
    UsedMemorySize = 7,
    DebuggerAttached = 8,
    ResourceLimit = 9,
    IdleTickCount = 10,
    RandomEntropy = 11,
    AslrRegionAddress = 12,
    AslrRegionSize = 13,
    StackRegionAddress = 14,
    StackRegionSize = 15,
    SystemResourceSizeTotal = 16,
    SystemResourceSizeUsed = 17,
    ProgramId = 18,
    InitialProcessIdRange = 19,
    UserExceptionContextAddress = 20,
    TotalNonSystemMemorySize = 21,
    UsedNonSystemMemorySize = 22,
    IsApplication = 23,
    FreeThreadCount = 24,
    ThreadTickCount = 25,
    IsSvcPermitted = 26,
    IoRegionHint = 27,
    AliasRegionExtraSize = 28,
    TransferMemoryHint = 34,
    ThreadTickCountDeprecated = 0xF0000002,
}
/// Retrieves information about the system, or a certain kernel object.
///
/// `Result svcGetInfo(uint64_t* out, uint32_t id0, Handle handle, uint64_t id1);`
///
/// Syscall code: [GET_INFO](crate::code::GET_INFO) (`0x29`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Variable to which store the information |
/// | IN | _id0_ | First ID of the property to retrieve |
/// | IN | _handle_ | Handle of the object to retrieve information from, or [INVALID_HANDLE] to retrieve information about the system |
/// | IN | _id1_ | Second ID of the property to retrieve |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetInfo>
///
/// # Safety
///
/// The caller must ensure:
/// - `out` is a valid, aligned pointer to writable memory where the result will be stored
/// - `handle` is a valid kernel handle (if not INVALID_HANDLE)
#[unsafe(naked)]
pub unsafe extern "C" fn get_info(out: *mut u64, id0: u32, handle: Handle, id1: u64) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc {code}",          // Issue the SVC call with immediate value 0x29
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (info result) to address in x2
        "ret",
        code = const GET_INFO,
    );
}

//</editor-fold>

//<editor-fold desc="Cache Management">

/// Flushes the entire data cache (by set/way).
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// This syscall is dangerous, and should not be used.
/// </div>
///
/// Syscall code: [FLUSH_ENTIRE_DATA_CACHE](crate::code::FLUSH_ENTIRE_DATA_CACHE) (`0x2A`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#FlushEntireDataCache>
///
/// # Safety
///
/// This is a privileged syscall that flushes the entire data cache. It may not be available
/// on all processes. This is a dangerous operation that can affect system performance.
#[unsafe(naked)]
pub unsafe extern "C" fn flush_entire_data_cache() {
    core::arch::naked_asm!(
        "svc 0x2A", // Issue the SVC call with immediate value 0x2A
        "ret"
    );
}

/// Flushes data cache for a virtual address range.
///
/// `Result svcFlushDataCache(void *address, size_t size);`
///
/// Syscall code: [FLUSH_DATA_CACHE](crate::code::FLUSH_DATA_CACHE) (`0x2B`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address of region to flush |
/// | IN | _size_ | Size of region to flush |
///
/// Ref: <https://switchbrew.org/wiki/SVC#FlushDataCache>
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid memory range owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn flush_data_cache(address: *mut c_void, size: usize) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x2B", // Issue the SVC call with immediate value 0x2B
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Memory management">

/// Maps new heap memory at the desired address. [3.0.0+]
///
/// `Result svcMapPhysicalMemory(void *address, uint64_t size);`
///
/// Syscall code: [MAP_PHYSICAL_MEMORY](crate::code::MAP_PHYSICAL_MEMORY) (`0x2C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address to map the memory to |
/// | IN | _size_ | Size of the memory |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapPhysicalMemory>
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid, page-aligned memory range
/// owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn map_physical_memory(address: *mut c_void, size: u64) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x2C", // Issue the SVC call with immediate value 0x2C
        "ret"
    );
}

/// Undoes the effects of [map_physical_memory]. [3.0.0+]
///
/// `Result svcUnmapPhysicalMemory(void *address, uint64_t size);`
///
/// Syscall code: [UNMAP_PHYSICAL_MEMORY](crate::code::UNMAP_PHYSICAL_MEMORY) (`0x2D`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address of the mapped memory |
/// | IN | _size_ | Size of the memory |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapPhysicalMemory>
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid, page-aligned memory range
/// that was previously mapped with [`map_physical_memory`].
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_physical_memory(address: *mut c_void, size: u64) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x2D", // Issue the SVC call with immediate value 0x2D
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Process and thread management">

/// Gets information about a thread that will be scheduled in the future. [5.0.0+]
///
/// <div class="warning">
/// This is a privileged syscall. Check if it is available first (e.g., `envIsSyscallHinted`).
/// </div>
///
/// `Result svcGetDebugFutureThreadInfo(arch::LastThreadContext *context, uint64_t *thread_id, Handle debug_handle, int64_t ns);`
///
/// Syscall code: [GET_DEBUG_FUTURE_THREAD_INFO](crate::code::GET_DEBUG_FUTURE_THREAD_INFO) (`0x2E`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _context_ | Output [LastThreadContext] for the thread that will be scheduled.
/// | OUT | _thread_id_ | Output thread id for the thread that will be scheduled.
/// | IN | _debug_ | Debug handle.
/// | IN | _ns_ | Nanoseconds in the future to get scheduled thread at.
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetDebugFutureThreadInfo>
///
/// # Safety
///
/// The caller must ensure:
/// - Both `context` and `thread_id` are valid, aligned pointers to writable memory
/// - `debug` is a valid debug handle
#[unsafe(naked)]
pub unsafe extern "C" fn get_debug_future_thread_info(
    context: *mut LastThreadContext,
    thread_id: *mut u64,
    debug: Handle,
    ns: i64,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
        "svc 0x2E",                // Issue the SVC call with immediate value 0x2E
        "ldp x6, x7, [sp], #16",   // Load x6 and x7 from the stack
        "stp x1, x2, [x6]",        // Store x1 and x2 in the memory pointed to by x6
        "stp x3, x4, [x6, #16]", // Store x3 and x4 in the memory pointed to by x6, offset by 16 bytes
        "str x5, [x7]",          // Store x5 in the memory pointed to by x7
        "ret"
    );
}

/// Gets information about the previously-scheduled thread.
///
/// `Result svcGetLastThreadInfo(arch::LastThreadContext *context, uint64_t *tls_address, uint32_t *flags);`
///
/// Syscall code: [GET_LAST_THREAD_INFO](crate::code::GET_LAST_THREAD_INFO) (`0x2F`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _context_ | Output [LastThreadContext] for the previously scheduled thread |
/// | OUT | _tls_address_ | Output TLS address for the previously scheduled thread |
/// | OUT | _flags_ | Output flags for the previously scheduled thread |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetLastThreadInfo>
///
/// # Safety
///
/// The caller must ensure that `context`, `tls_address`, and `flags` are all valid,
/// aligned pointers to writable memory.
#[unsafe(naked)]
pub unsafe extern "C" fn get_last_thread_info(
    context: *mut LastThreadContext,
    tls_address: *mut u64,
    flags: *mut u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x1, x2, [sp, #-16]!", // Store x1 and x2 on the stack
        "str x0, [sp, #-16]!",     // Store x0 on the stack
        "svc 0x2F",                // Issue the SVC call with immediate value 0x2F
        "ldr x7, [sp], #16",       // Load x7 from the stack
        "stp x1, x2, [x7]",        // Store x1 and x2 in the memory pointed to by x7
        "stp x3, x4, [x7, #16]", // Store x3 and x4 in the memory pointed to by x7, offset by 16 bytes
        "ldp x1, x2, [sp], #16", // Load x1 and x2 from the stack
        "str x5, [x1]",          // Store x5 in the memory pointed to by x1
        "str w6, [x2]",          // Store w6 in the memory pointed to by x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Resource Limit Management">

/// Gets the maximum value a [LimitableResource] can have, for a Resource Limit handle.
///
/// `Result svcGetResourceLimitLimitValue(int64_t *out, Handle reslimit, LimitableResource which);`
///
/// Syscall code: [GET_RESOURCE_LIMIT_LIMIT_VALUE](crate::code::GET_RESOURCE_LIMIT_LIMIT_VALUE) (`0x30`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _value_ | Output maximum value |
/// | IN | _handle_ | Resource limit handle |
/// | IN | _which_ | Resource to query |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetResourceLimitLimitValue>
///
/// # Safety
///
/// The caller must ensure:
/// - `value` is a valid, aligned pointer to writable memory
/// - `handle` is a valid resource limit handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_resource_limit_limit_value(
    value: *mut i64,
    handle: Handle,
    which: LimitableResource,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (value pointer) on stack
        "svc 0x30",            // Issue the SVC call with immediate value 0x30
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (limit value result) to address in x2
        "ret"
    );
}

/// Gets the current value a [LimitableResource] has, for a Resource Limit handle.
///
/// `Result svcGetResourceLimitCurrentValue(int64_t *out, Handle reslimit, LimitableResource which);`
///
/// Syscall code: [GET_RESOURCE_LIMIT_CURRENT_VALUE](crate::code::GET_RESOURCE_LIMIT_CURRENT_VALUE) (`0x31`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Output current value |
/// | IN | _reslimit_ | Resource limit handle |
/// | IN | _which_ | Resource to query |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetResourceLimitCurrentValue>
///
/// # Safety
///
/// The caller must ensure:
/// - `out` is a valid, aligned pointer to writable memory
/// - `reslimit` is a valid resource limit handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_resource_limit_current_value(
    out: *mut i64,
    reslimit: Handle,
    which: LimitableResource,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc 0x31",            // Issue the SVC call with immediate value 0x31
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (current value result) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Process and thread management">

/// Configures the pause/unpause status of a thread.
///
/// `Result svcSetThreadActivity(Handle thread, ThreadActivity paused);`
///
/// Syscall code: [SET_THREAD_ACTIVITY](crate::code::SET_THREAD_ACTIVITY) (`0x32`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _thread_ | Thread handle |
/// | IN | _paused_ | Whether to pause or unpause the thread |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetThreadActivity>
///
/// # Safety
///
/// The caller must ensure that `thread` is a valid kernel thread handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn set_thread_activity(thread: Handle, paused: ThreadActivity) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x32", // Issue the SVC call with immediate value 0x32
        "ret"
    );
}

/// Dumps the registers of a thread paused by [set_thread_activity] (register groups: all).
///
/// `Result svcGetThreadContext3(ThreadContext* ctx, Handle thread);`
///
/// Syscall code: [GET_THREAD_CONTEXT3](crate::code::GET_THREAD_CONTEXT3) (`0x33`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _ctx_ | Output thread context (register dump) |
/// | IN | _thread_ | Thread handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetThreadContext3>
///
/// # Safety
///
/// The caller must ensure:
/// - `ctx` is a valid, aligned pointer to writable memory for the thread context
/// - `thread` is a valid kernel thread handle in a paused state
#[unsafe(naked)]
pub unsafe extern "C" fn get_thread_context3(
    ctx: *mut ThreadContext,
    thread: Handle,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x33", // Issue the SVC call with immediate value 0x33
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Synchronization">

/// Arbitrates an address depending on type and value. [4.0.0+]
///
/// `Result svcWaitForAddress(void *address, ArbitrationType arb_type, int64_t value, int64_t timeout);`
///
/// Syscall code: [WAIT_FOR_ADDRESS](crate::code::WAIT_FOR_ADDRESS) (`0x34`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address to arbitrate |
/// | IN | _arb_type_ | [ArbitrationType] to use |
/// | IN | _value_ | Value to arbitrate on |
/// | IN | _timeout_ | Maximum time in nanoseconds to wait |
///
/// Ref: <https://switchbrew.org/wiki/SVC#WaitForAddress>
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid value within the process's
/// address space.
#[unsafe(naked)]
pub unsafe extern "C" fn wait_for_address(
    address: *mut c_void,
    arb_type: ArbitrationType,
    value: i64,
    timeout: i64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x34", // Issue the SVC call with immediate value 0x34
        "ret"
    );
}

/// Signals (and updates) an address depending on type and value. [4.0.0+]
///
/// `Result svcSignalToAddress(void *address, SignalType signal_type, int32_t value, int32_t count);`
///
/// Syscall code: [SIGNAL_TO_ADDRESS](crate::code::SIGNAL_TO_ADDRESS) (`0x35`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address to arbitrate |
/// | IN | _signal_type_ | [SignalType] to use |
/// | IN | _value_ | Value to arbitrate on |
/// | IN | _count_ | Number of waiting threads to signal |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SignalToAddress>
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid value within the process's
/// address space.
#[unsafe(naked)]
pub unsafe extern "C" fn signal_to_address(
    address: *mut c_void,
    signal_type: SignalType,
    value: i32,
    count: i32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x35", // Issue the SVC call with immediate value 0x35
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Miscellaneous">

/// Sets thread preemption state (used during abort/panic). [8.0.0+]
///
/// `void svcSynchronizePreemptionState(void);`
///
/// Syscall code: [SYNCHRONIZE_PREEMPTION_STATE](crate::code::SYNCHRONIZE_PREEMPTION_STATE) (`0x36`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#SynchronizePreemptionState>
///
/// # Safety
///
/// This function is safe to call from any context.
#[unsafe(naked)]
pub unsafe extern "C" fn synchronize_preemption_state() {
    core::arch::naked_asm!(
        "svc 0x36", // Issue the SVC call with immediate value 0x36
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Resource Limit Management">

/// Gets the peak value a [LimitableResource] has had, for a Resource Limit handle. [11.0.0+]
///
/// `Result svcGetResourceLimitPeakValue(int64_t *out, Handle reslimit, LimitableResource which);`
///
/// Syscall code: [GET_RESOURCE_LIMIT_PEAK_VALUE](crate::code::GET_RESOURCE_LIMIT_PEAK_VALUE) (`0x37`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Output peak value |
/// | IN | _reslimit_ | Resource limit handle |
/// | IN | _which_ | Resource to query |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetResourceLimitPeakValue>
///
/// # Safety
///
/// The caller must ensure:
/// - `out` is a valid, aligned pointer to writable memory
/// - `reslimit` is a valid resource limit handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_resource_limit_peak_value(
    out: *mut i64,
    reslimit: Handle,
    which: LimitableResource,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc 0x37",            // Issue the SVC call with immediate value 0x37
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (peak value result) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Memory management">

/// Creates an IO Pool. [13.0.0+]
///
/// `Result svcCreateIoPool(Handle* handle, uint32_t pool_type);`
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// Syscall code: [CREATE_IO_POOL](crate::code::CREATE_IO_POOL) (`0x39`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for the created IO pool |
/// | IN | _which_ | [IoPoolType] to create |
///
/// Ref: <https://switchbrew.org/wiki/SVC#:~:text=0x39,CreateIoPool>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid, aligned pointer to writable memory
/// for the output handle.
#[unsafe(naked)]
pub unsafe extern "C" fn create_io_pool(handle: *mut Handle, which: IoPoolType) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack
        "svc 0x39",            // Issue the SVC call with immediate value 0x39
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

/// Creates an IO Region. [13.0.0+]
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcCreateIoRegion(Handle* handle, Handle io_pool_h, uint64_t physical_address, uint64_t size, MemoryMapping mapping, MemoryPermission perm);`
///
/// Syscall code: [CREATE_IO_REGION](crate::code::CREATE_IO_REGION) (`0x3A`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for the created IO region |
/// | IN | _io_pool_h_ | Handle to the IO pool |
/// | IN | _physical_address_ | Physical address to map |
/// | IN | _size_ | Size of the region |
/// | IN | _mapping_ | [MemoryMapping\ configuration |
/// | IN | _perm_ | [MemoryPermission] configuration |
///
/// Ref: <https://switchbrew.org/wiki/SVC#:~:text=0x3A,CreateIoRegion>
///
/// # Safety
///
/// The caller must ensure:
/// - `handle` is a valid, aligned pointer to writable memory for the output handle
/// - `io_pool_h` is a valid IO pool handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn create_io_region(
    handle: *mut Handle,
    io_pool_h: Handle,
    physical_address: u64,
    size: u64,
    mapping: MemoryMapping,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack
        "svc 0x3A",            // Issue the SVC call with immediate value 0x3A
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Debugging">

/// Causes the kernel to dump debug information. [1.0.0-3.0.2]
///
/// `void svcDumpInfo(uint32_t dump_info_type, uint64_t arg0);`
///
/// Syscall code: [DUMP_INFO](crate::code::DUMP_INFO) (`0x3C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _dump_info_type_ | Type of debug information to dump |
/// | IN | _arg0_ | Additional argument |
///
/// Ref: <https://switchbrew.org/wiki/SVC#DumpInfo>
///
/// # Safety
///
/// This function is safe to call, though it may not be available on all processes.
#[unsafe(naked)]
pub unsafe extern "C" fn dump_info(dump_info_type: u32, arg0: u64) {
    core::arch::naked_asm!(
        "svc 0x3C", // Issue the SVC call with immediate value 0x3C
        "ret"
    );
}

/// Performs a debugging operation on the kernel. [4.0.0+]
///
/// `void svcKernelDebug(uint32_t kern_debug_type, u64 arg0, uint64_t arg1, uint64_t arg2);`
///
/// Syscall code: [KERNEL_DEBUG](crate::code::KERNEL_DEBUG) (`0x3C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _kern_debug_type_ | Type of debug operation |
/// | IN | _arg0_ | First additional argument |
/// | IN | _arg1_ | Second additional argument |
/// | IN | _arg2_ | Third additional argument |
///
/// Ref: <https://switchbrew.org/wiki/SVC#KernelDebug>
///
/// # Safety
///
/// This function is safe to call, though it may not be available on all processes.
#[unsafe(naked)]
pub unsafe extern "C" fn kernel_debug(kern_debug_type: u32, arg0: u64, arg1: u64, arg2: u64) {
    core::arch::naked_asm!(
        "svc 0x3C", // Issue the SVC call with immediate value 0x3C
        "ret"
    );
}

/// Changes the kernel's trace state. [4.0.0+]
///
/// `void svcChangeKernelTraceState(uint32_t kern_trace_state);`
///
/// Syscall code: [CHANGE_KERNEL_TRACE_STATE](crate::code::CHANGE_KERNEL_TRACE_STATE) (`0x3D`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _kern_trace_state_ | New kernel trace state |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ChangeKernelTraceState>
///
/// # Safety
///
/// This function is safe to call, though it may not be available on all processes.
#[unsafe(naked)]
pub unsafe extern "C" fn change_kernel_trace_state(kern_trace_state: u32) {
    core::arch::naked_asm!(
        "svc 0x3D", // Issue the SVC call with immediate value 0x3D
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Inter-process communication (IPC)">

/// Creates an IPC session.
///
/// `Result svcCreateSession(Handle* server_handle, Handle* client_handle, bool is_light, uintptr_t name);`
///
/// Syscall code: [CREATE_SESSION](crate::code::CREATE_SESSION) (`0x40`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _server_handle_ | Output handle for the server endpoint |
/// | OUT | _client_handle_ | Output handle for the client endpoint |
/// | IN | _unk0_ | Unknown parameter |
/// | IN | _unk1_ | Unknown parameter |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateSession>
///
/// # Safety
///
/// The caller must ensure that both `server_handle` and `client_handle` are valid,
/// aligned pointers to writable memory for the output handles.
#[unsafe(naked)]
pub unsafe extern "C" fn create_session(
    server_handle: *mut Handle,
    client_handle: *mut Handle,
    is_light: bool,
    unk1: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x0, x1, [sp, #-16]!", // Store x0 and x1 (handle pointers) on stack
        "svc 0x40",                // Issue the SVC call with immediate value 0x40
        "ldp x3, x4, [sp], #16",   // Load x3 and x4 from stack
        "str w1, [x3]",            // Store w1 (server handle result) to address in x3
        "str w2, [x4]",            // Store w2 (client handle result) to address in x4
        "ret"
    );
}

/// Accepts an IPC session.
///
/// `Result svcAcceptSession(Handle* session_handle, Handle port_handle);`
///
/// Syscall code: [ACCEPT_SESSION](crate::code::ACCEPT_SESSION) (`0x41`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _session_ | Output handle for the accepted session |
/// | IN | _port_handle_ | Handle to the port to accept from |
///
/// Ref: <https://switchbrew.org/wiki/SVC#AcceptSession>
///
/// # Safety
///
/// The caller must ensure:
/// - `session` is a valid, aligned pointer to writable memory for the output handle
/// - `port_handle` is a valid port handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn accept_session(session: *mut Handle, port_handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (session pointer) on stack
        "svc 0x41",            // Issue the SVC call with immediate value 0x41
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (session handle result) to address in x2
        "ret"
    );
}

/// Performs light IPC input/output.
///
/// `Result svcReplyAndReceiveLight(Handle handle);`
///
/// Syscall code: [REPLY_AND_RECEIVE_LIGHT](crate::code::REPLY_AND_RECEIVE_LIGHT) (`0x42`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle to perform IPC on |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ReplyAndReceiveLight>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid kernel session handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn reply_and_receive_light(handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x42", // Issue the SVC call with immediate value 0x42
        "ret"
    );
}

/// Performs IPC input/output.
///
/// If ReplyTargetSessionHandle is not zero, a reply from the TLS will be sent to that session. Then
/// it will wait until either of the passed sessions has an incoming message, is closed, a passed
/// port has an incoming connection, or the timeout expires. If there is an incoming message, it is
/// copied to the TLS.
///
/// If ReplyTargetSessionHandle is zero, the TLS should contain a blank message. If this message has
/// a C descriptor, the buffer it points to will be used as the pointer buffer. See
/// [IPC_Marshalling#IPC_buffers](https://switchbrew.org/wiki/IPC_Marshalling#IPC_buffers). Note
/// that a pointer buffer cannot be specified if `reply_target` is not zero.
///
/// After being validated, passed handles will be enumerated in order; even if a session has been
/// closed, if one that appears earlier in the list has an incoming message, it will take priority
/// and a result code of 0x0 will be returned.
///
/// `Result svcReplyAndReceive(int32_t *index, const Handle *handles, int32_t handle_count, Handle reply_target, uint64_t timeout);`
///
/// Syscall code: [REPLY_AND_RECEIVE](crate::code::REPLY_AND_RECEIVE) (`0x43`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _index_ | Output index of signaled handle |
/// | IN | _handles_ | Array of handles to wait on |
/// | IN | _handle_count_ | Number of handles |
/// | IN | _reply_target_ | Handle to reply to |
/// | IN | _timeout_ | Timeout in nanoseconds |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ReplyAndReceive>
///
/// # Safety
///
/// The caller must ensure:
/// - `index` is a valid, aligned pointer to writable memory for the result index
/// - `handles` points to a valid array of `handle_count` kernel handles
/// - All handles are valid session handles owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn reply_and_receive(
    index: *mut i32,
    handles: *const u32,
    handle_count: i32,
    reply_target: u32,
    timeout: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (index pointer) on stack
        "svc 0x43",            // Issue the SVC call with immediate value 0x43
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (index result) to address in x2
        "ret"
    );
}

/// Performs IPC input/output from a user allocated buffer.
///
/// `Result svcReplyAndReceiveWithUserBuffer(int32_t *index, void *usr_buffer, uint64_t size, const Handle *handles, int32_t handle_count, Handle reply_target, uint64_t timeout);`
///
/// Syscall code: [REPLY_AND_RECEIVE_WITH_USER_BUFFER](crate::code::REPLY_AND_RECEIVE_WITH_USER_BUFFER) (`0x44`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _index_ | Output index of signaled handle |
/// | IN | _usr_buffer_ | User buffer for IPC |
/// | IN | _size_ | Size of user buffer |
/// | IN | _handles_ | Array of handles to wait on |
/// | IN | _handle_count_ | Number of handles |
/// | IN | _reply_target_ | Handle to reply to |
/// | IN | _timeout_ | Timeout in nanoseconds |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ReplyAndReceiveWithUserBuffer>
///
/// # Safety
///
/// The caller must ensure:
/// - `index` is a valid, aligned pointer to writable memory for the result index
/// - `usr_buffer` points to valid memory owned by the current process
/// - `handles` points to a valid array of `handle_count` kernel handles
/// - All handles are valid session handles owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn reply_and_receive_with_user_buffer(
    index: *mut i32,
    usr_buffer: *mut c_void,
    size: u64,
    handles: *const Handle,
    handle_count: i32,
    reply_target: Handle,
    timeout: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (index pointer) on stack
        "svc 0x44",            // Issue the SVC call with immediate value 0x44
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (index result) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Synchronization">

/// Creates a system event.
///
/// `Result svcCreateEvent(Handle* server_handle, Handle* client_handle);`
///
/// Syscall code: [CREATE_EVENT](crate::code::CREATE_EVENT) (`0x45`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _server_handle_ | Output handle for server |
/// | OUT | _client_handle_ | Output handle for client |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateEvent>
///
/// # Safety
///
/// The caller must ensure that both `server_handle` and `client_handle` are valid,
/// aligned pointers to writable memory for the output handles.
#[unsafe(naked)]
pub unsafe extern "C" fn create_event(
    server_handle: *mut Handle,
    client_handle: *mut Handle,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x0, x1, [sp, #-16]!", // Store x0 and x1 (handle pointers) on stack
        "svc 0x45",                // Issue the SVC call with immediate value 0x45
        "ldp x3, x4, [sp], #16",   // Load x3 and x4 from stack
        "str w1, [x3]",            // Store w1 (server handle result) to address in x3
        "str w2, [x4]",            // Store w2 (client handle result) to address in x4
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Memory management">

/// Maps an IO Region. [13.0.0+]
///
/// `Result svcMapIoRegion(Handle io_region_h, void* address, uint64_t size, uint32_t perm);`
///
/// Syscall code: [MAP_IO_REGION](crate::code::MAP_IO_REGION) (`0x46`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _io_region_h_ | IO region handle |
/// | IN | _address_ | Address to map to |
/// | IN | _size_ | Size of the mapping |
/// | IN | _perm_ | Memory permissions |
///
/// Ref: <https://switchbrew.org/wiki/SVC#:~:text=0x46,MapIoRegion>
///
/// # Safety
///
/// The caller must ensure:
/// - `io_region_h` is a valid IO region handle owned by the current process
/// - `address` points to a valid memory range owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn map_io_region(
    io_region_h: Handle,
    address: *mut c_void,
    size: u64,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x46", // Issue the SVC call with immediate value 0x46
        "ret"
    );
}

/// Undoes the effects of [map_io_region]. [13.0.0+]
///
/// `Result svcUnmapIoRegion(Handle io_region_h, void* address, uint64_t size);`
///
/// Syscall code: [UNMAP_IO_REGION](crate::code::UNMAP_IO_REGION) (`0x47`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _io_region_h_ | IO region handle |
/// | IN | _address_ | Address to unmap |
/// | IN | _size_ | Size of the mapping |
///
/// Ref: <https://switchbrew.org/wiki/SVC#:~:text=0x47,UnmapIoRegion>
///
/// # Safety
///
/// The caller must ensure:
/// - `io_region_h` is a valid IO region handle owned by the current process
/// - `address` points to a memory range that was previously mapped
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_io_region(
    io_region_h: Handle,
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x47", // Issue the SVC call with immediate value 0x47
        "ret"
    );
}

/// Maps unsafe memory (usable for GPU DMA) for a system module at the desired address. [5.0.0+]
///
/// `Result svcMapPhysicalMemoryUnsafe(void* address, uint64_t size);`
///
/// Syscall code: [MAP_PHYSICAL_MEMORY_UNSAFE](crate::code::MAP_PHYSICAL_MEMORY_UNSAFE) (`0x48`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address to map to |
/// | IN | _size_ | Size of the mapping |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapPhysicalMemoryUnsafe>
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid, page-aligned memory range
/// owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn map_physical_memory_unsafe(address: *mut c_void, size: u64) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x48", // Issue the SVC call with immediate value 0x48
        "ret"
    );
}

/// Undoes the effects of [map_physical_memory_unsafe]. [5.0.0+]
///
/// `Result svcUnmapPhysicalMemoryUnsafe(void* address, uint64_t size);`
///
/// Syscall code: [UNMAP_PHYSICAL_MEMORY_UNSAFE](crate::code::UNMAP_PHYSICAL_MEMORY_UNSAFE) (`0x49`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address to unmap |
/// | IN | _size_ | Size of the mapping |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapPhysicalMemoryUnsafe>
///
/// # Safety
///
/// The caller must ensure that `address` points to a memory range that was previously
/// mapped with [`map_physical_memory_unsafe`].
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_physical_memory_unsafe(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x49", // Issue the SVC call with immediate value 0x49
        "ret"
    );
}

/// Sets the system-wide limit for unsafe memory mappable using [map_physical_memory_unsafe]. [5.0.0+]
///
/// `Result svcSetUnsafeLimit(uint64_t size);`
///
/// Syscall code: [SET_UNSAFE_LIMIT](crate::code::SET_UNSAFE_LIMIT) (`0x4A`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _size_ | Size limit |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetUnsafeLimit>
///
/// # Safety
///
/// This function is safe to call from any context.
#[unsafe(naked)]
pub unsafe extern "C" fn set_unsafe_limit(size: u64) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x4A", // Issue the SVC call with immediate value 0x4A
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Code memory / Just-in-time (JIT) compilation support">

/// Creates code memory in the caller's address space [4.0.0+].
///
/// `Result svcCreateCodeMemory(Handle* handle, void* src_addr, uint64_t size);`
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// Syscall code: [CREATE_CODE_MEMORY](crate::code::CREATE_CODE_MEMORY) (`0x4B`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for the created code memory |
/// | IN | _src_addr_ | Source address |
/// | IN | _size_ | Size of the memory |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateCodeMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `handle` is a valid, aligned pointer to writable memory for the output handle
/// - `src_addr` points to a valid memory range owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn create_code_memory(
    handle: *mut Handle,
    src_addr: *mut c_void,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 on the stack
        "svc 0x4B",            // Issue the SVC call with immediate value 0x4B
        "ldr x2, [sp], #16",   // Load x2 from the stack
        "str w1, [x2]",        // Store w1 in the memory pointed to by x2
        "ret"
    );
}

/// Maps code memory in the caller's address space [4.0.0+].
///
/// `Result svcControlCodeMemory(Handle code_handle, CodeMapOperation op, void* dst_addr, uint64_t size, uint64_t perm);`
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// Syscall code: [CONTROL_CODE_MEMORY](crate::code::CONTROL_CODE_MEMORY) (`0x4C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _code_handle_ | Handle of the code memory |
/// | IN | _op_ | Operation to perform (see [CodeMapOperation]) |
/// | IN | _dst_addr_ | Destination address |
/// | IN | _size_ | Size of the memory |
/// | IN | _perm_ | Memory permissions |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ControlCodeMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `code_handle` is a valid code memory handle owned by the current process
/// - `dst_addr` points to a valid memory range owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn control_code_memory(
    code_handle: Handle,
    op: CodeMapOperation,
    dst_addr: *mut c_void,
    size: u64,
    perm: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x4C", // Issue the SVC call with immediate value 0x4C
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Power Management">

/// Causes the system to enter deep sleep.
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcSleepSystem(void);`
///
/// Syscall code: [SLEEP_SYSTEM](crate::code::SLEEP_SYSTEM) (`0x4D`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#SleepSystem>
///
/// # Safety
///
/// This is a privileged syscall. The function is safe to call if it's available, but it may
/// terminate execution immediately.
#[unsafe(naked)]
pub unsafe extern "C" fn sleep_system() {
    core::arch::naked_asm!(
        "svc 0x4D", // Issue the SVC call with immediate value 0x4D
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Device memory-mapped I/O (MMIO)">

/// Reads/writes a protected MMIO register.
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcReadWriteRegister(uint32_t* outVal, uint64_t regAddr, uint32_t rwMask, uint32_t inVal);`
///
/// Syscall code: [READ_WRITE_REGISTER](crate::code::READ_WRITE_REGISTER) (`0x4E`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_val_ | Output value read from register |
/// | IN | _reg_addr_ | Register address |
/// | IN | _mask_ | Read/write mask |
/// | IN | _value_ | Input value to write |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ReadWriteRegister>
///
/// # Safety
///
/// The caller must ensure that `out_val` is a valid, aligned pointer to writable memory
/// for the register value result.
#[unsafe(naked)]
pub unsafe extern "C" fn read_write_register(
    out_val: *mut u32,
    reg_addr: u64,
    mask: u32,
    value: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out_val pointer) on stack
        "svc 0x4E",            // Issue the SVC call with immediate value 0x4E
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (register value result) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Process and thread management">

/// Configures the pause/unpause status of a process.
///
/// `Result svcSetProcessActivity(Handle process, ProcessActivity paused);`
///
/// Syscall code: [SET_PROCESS_ACTIVITY](crate::code::SET_PROCESS_ACTIVITY) (`0x4F`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _process_ | Process handle |
/// | IN | _paused_ | Whether to pause (1) or unpause (0) the process |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetProcessActivity>
///
/// # Safety
///
/// The caller must ensure that `process` is a valid kernel process handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn set_process_activity(
    process: Handle,
    paused: ProcessActivity,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x4F", // Issue the SVC call with immediate value 0x4F
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Inter-process memory sharing">

/// Creates a block of shared memory.
///
/// `Result svcCreateSharedMemory(Handle* out, size_t size, MemoryPermission local_perm, MemoryPermission other_perm);`
///
/// Syscall code: [CREATE_SHARED_MEMORY](crate::code::CREATE_SHARED_MEMORY) (`0x50`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle of the created shared memory block |
/// | IN | _size_ | Size of the shared memory block in bytes |
/// | IN | _local_perm_ | [MemoryPermission] for the current process |
/// | IN | _other_perm_ | [MemoryPermission] for other processes |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateSharedMemory>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid, aligned pointer to writable memory
/// for the output handle.
#[unsafe(naked)]
pub unsafe extern "C" fn create_shared_memory(
    handle: *mut Handle,
    size: usize,
    local_perm: u32, // TODO: MemoryPermission bitfield
    other_perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack
        "svc 0x50",            // Issue the SVC call with immediate value 0x50
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

/// Maps a block of transfer memory.
///
/// `Result svcMapTransferMemory(Handle tmem_handle, void* addr, size_t size, uint32_t perm);`
///
/// Syscall code: [MAP_TRANSFER_MEMORY](crate::code::MAP_TRANSFER_MEMORY) (`0x51`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _tmem_handle_ | Handle to the transfer memory |
/// | IN | _addr_ | Address to map to |
/// | IN | _size_ | Size of the transfer memory |
/// | IN | _perm_ | Memory permissions |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapTransferMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `tmem_handle` is a valid transfer memory handle owned by the current process
/// - `addr` points to a valid memory range owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn map_transfer_memory(
    tmem_handle: Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x51", // Issue the SVC call with immediate value 0x51
        "ret"
    );
}

/// Unmaps a block of transfer memory.
///
/// `Result svcUnmapTransferMemory(Handle tmem_handle, void* addr, size_t size);`
///
/// Syscall code: [UNMAP_TRANSFER_MEMORY](crate::code::UNMAP_TRANSFER_MEMORY) (`0x52`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _tmem_handle_ | Handle to the transfer memory |
/// | IN | _addr_ | Mapped address |
/// | IN | _size_ | Size of the transfer memory |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapTransferMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `tmem_handle` is a valid transfer memory handle owned by the current process
/// - `addr` points to a memory range that was previously mapped
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_transfer_memory(
    tmem_handle: Handle,
    addr: *mut c_void,
    size: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x52", // Issue the SVC call with immediate value 0x52
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Device memory-mapped I/O (MMIO)">

/// Creates an event and binds it to a specific hardware interrupt.
///
/// `Result svcCreateInterruptEvent(Handle* handle, uint64_t irq_num, uint32_t flag);`
///
/// Syscall code: [CREATE_INTERRUPT_EVENT](crate::code::CREATE_INTERRUPT_EVENT) (`0x53`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for the created event |
/// | IN | _irq_num_ | IRQ number to bind to |
/// | IN | _flag_ | Flags for the event |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateInterruptEvent>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid, aligned pointer to writable memory
/// for the output handle.
#[unsafe(naked)]
pub unsafe extern "C" fn create_interrupt_event(
    handle: *mut Handle,
    irq_num: u64,
    flag: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack
        "svc 0x53",            // Issue the SVC call with immediate value 0x53
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

/// Queries information about a certain virtual address, including its physical address.
///
/// `Result svcQueryPhysicalAddress(PhysicalMemoryInfo* out, uint64_t virtaddr);`
///
/// Syscall code: [QUERY_PHYSICAL_ADDRESS](crate::code::QUERY_PHYSICAL_ADDRESS) (`0x54`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Output physical memory information |
/// | IN | _virtaddr_ | Virtual address to query |
///
/// Ref: <https://switchbrew.org/wiki/SVC#QueryPhysicalAddress>
///
/// # Safety
///
/// The caller must ensure that `out` is a valid, aligned pointer to writable memory
/// for the physical memory information result.
#[unsafe(naked)]
pub unsafe extern "C" fn query_physical_address(
    out: *mut PhysicalMemoryInfo,
    virtaddr: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc 0x54",            // Issue the SVC call with immediate value 0x54
        "ldr x4, [sp], #16",   // Load x4 from stack
        "stp x1, x2, [x4]",    // Store x1 and x2 to PhysicalMemoryInfo struct
        "str x3, [x4, #16]",   // Store x3 at offset 16 bytes in struct
        "ret"
    );
}

/// Returns a virtual address mapped to a given IO range. [10.0.0+]
///
/// `Result svcQueryMemoryMapping(u64* virtaddr, u64* out_size, uint64_t physaddr, uint64_t size);`
///
/// Syscall code: [QUERY_MEMORY_MAPPING](crate::code::QUERY_MEMORY_MAPPING) (`0x55`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _virtaddr_ | Output virtual address |
/// | OUT | _out_size_ | Output size of the mapping |
/// | IN | _physaddr_ | Physical address to query |
/// | IN | _size_ | Size of the region |
///
/// Ref: <https://switchbrew.org/wiki/SVC#:~:text=%5B10.0.0%2B%5D-,0x55,QueryMemoryMapping,-uintptr_t%20*out_address%2C%20size_t>
///
/// # Safety
///
/// The caller must ensure that both `virtaddr` and `out_size` are valid, aligned pointers
/// to writable memory for the results.
#[unsafe(naked)]
pub unsafe extern "C" fn query_memory_mapping(
    virtaddr: *mut u64,
    out_size: *mut u64,
    physaddr: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x0, x1, [sp, #-16]!", // Store x0 and x1 (pointers) on stack
        "svc 0x55",                // Issue the SVC call with immediate value 0x55
        "ldp x3, x4, [sp], #16",   // Load x3 and x4 from stack
        "str x1, [x3]",            // Store x1 (virtaddr result) to address in x3
        "str x2, [x4]",            // Store x2 (size result) to address in x4
        "ret"
    );
}

/// Returns a virtual address mapped to a given IO range. [1.0.0-9.2.0]
///
/// `Result svcLegacyQueryIoMapping(u64* virtaddr, uint64_t physaddr, uint64_t size);`
///
/// Syscall code: [LEGACY_QUERY_IO_MAPPING](crate::code::LEGACY_QUERY_IO_MAPPING) (`0x55`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _virtaddr_ | Output virtual address |
/// | IN | _physaddr_ | Physical address to query |
/// | IN | _size_ | Size of the region |
///
/// Ref: <https://switchbrew.org/wiki/SVC#QueryIoMapping>
///
/// # Safety
///
/// The caller must ensure that `virtaddr` is a valid, aligned pointer to writable memory
/// for the virtual address result.
#[unsafe(naked)]
pub unsafe extern "C" fn legacy_query_io_mapping(
    virtaddr: *mut u64,
    physaddr: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (virtaddr pointer) on stack
        "svc 0x55",            // Issue the SVC call with immediate value 0x55
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (virtual address result) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="I/O memory management" unit (IOMMU)">

/// Creates a virtual address space for binding device address spaces.
///
/// `Result svcCreateDeviceAddressSpace(Handle* handle, uint64_t dev_addr, uint64_t dev_size);`
///
/// Syscall code: [CREATE_DEVICE_ADDRESS_SPACE](crate::code::CREATE_DEVICE_ADDRESS_SPACE) (`0x56`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _handle_ | Output handle for the created address space |
/// | IN | _dev_addr_ | Device address |
/// | IN | _dev_size_ | Size of the device address space |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateDeviceAddressSpace>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid, aligned pointer to writable memory
/// for the output handle.
#[unsafe(naked)]
pub unsafe extern "C" fn create_device_address_space(
    handle: *mut Handle,
    dev_addr: u64,
    dev_size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (handle pointer) on stack
        "svc 0x56",            // Issue the SVC call with immediate value 0x56
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

/// Attaches a device address space to a device.
///
/// `Result svcAttachDeviceAddressSpace(uint64_t device, Handle handle);`
///
/// Syscall code: [ATTACH_DEVICE_ADDRESS_SPACE](crate::code::ATTACH_DEVICE_ADDRESS_SPACE) (`0x57`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _device_ | Device identifier |
/// | IN | _handle_ | Handle to the device address space |
///
/// Ref: <https://switchbrew.org/wiki/SVC#AttachDeviceAddressSpace>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid device address space handle
/// owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn attach_device_address_space(device: u64, handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x57", // Issue the SVC call with immediate value 0x57
        "ret"
    );
}

/// Detaches a device address space from a device.
///
/// `Result svcDetachDeviceAddressSpace(uint64_t device, Handle handle);`
///
/// Syscall code: [DETACH_DEVICE_ADDRESS_SPACE](crate::code::DETACH_DEVICE_ADDRESS_SPACE) (`0x58`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _device_ | Device identifier |
/// | IN | _handle_ | Handle to the device address space |
///
/// Ref: <https://switchbrew.org/wiki/SVC#DetachDeviceAddressSpace>
///
/// # Safety
///
/// The caller must ensure that `handle` is a valid device address space handle
/// owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn detach_device_address_space(device: u64, handle: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x58", // Issue the SVC call with immediate value 0x58
        "ret"
    );
}

/// Maps an attached device address space to an userspace address.
///
/// `Result svcMapDeviceAddressSpaceByForce(Handle handle, Handle proc_handle, u64 map_addr, uint64_t dev_size, uint64_t dev_addr, uint32_t option);`
///
/// Syscall code: [MAP_DEVICE_ADDRESS_SPACE_BY_FORCE](crate::code::MAP_DEVICE_ADDRESS_SPACE_BY_FORCE) (`0x59`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle to the device address space |
/// | IN | _proc_handle_ | Process handle |
/// | IN | _map_addr_ | Address to map to |
/// | IN | _dev_size_ | Size of the device address space |
/// | IN | _dev_addr_ | Device address |
/// | IN | _option_ | Mapping options |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapDeviceAddressSpaceByForce>
///
/// # Safety
///
/// The caller must ensure that both `handle` and `proc_handle` are valid kernel handles
/// owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn map_device_address_space_by_force(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    option: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x59", // Issue the SVC call with immediate value 0x59
        "ret"
    );
}

/// Maps an attached device address space to an userspace address.
///
/// `Result svcMapDeviceAddressSpaceAligned(Handle handle, Handle proc_handle, uint64_t map_addr, uint64_t dev_size, uint64_t dev_addr, uint32_t option);`
///
/// Syscall code: [MAP_DEVICE_ADDRESS_SPACE_ALIGNED](crate::code::MAP_DEVICE_ADDRESS_SPACE_ALIGNED) (`0x5A`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle to the device address space |
/// | IN | _proc_handle_ | Process handle |
/// | IN | _map_addr_ | Address to map to |
/// | IN | _dev_size_ | Size of the device address space |
/// | IN | _dev_addr_ | Device address |
/// | IN | _option_ | Mapping options |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapDeviceAddressSpaceAligned>
///
/// # Safety
///
/// The caller must ensure that both `handle` and `proc_handle` are valid kernel handles
/// owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn map_device_address_space_aligned(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    option: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x5A", // Issue the SVC call with immediate value 0x5A
        "ret"
    );
}

/// Maps an attached device address space to an userspace address. [1.0.0-12.1.0]
///
/// `Result svcMapDeviceAddressSpace(u64 *out_mapped_size, Handle handle, Handle proc_handle, u64 map_addr, uint64_t dev_size, uint64_t dev_addr, uint32_t perm);`
///
/// Syscall code: [MAP_DEVICE_ADDRESS_SPACE](crate::code::MAP_DEVICE_ADDRESS_SPACE) (`0x5B`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_mapped_size_ | Size of the mapped region |
/// | IN | _handle_ | Handle to the device address space |
/// | IN | _proc_handle_ | Process handle |
/// | IN | _map_addr_ | Address to map to |
/// | IN | _dev_size_ | Size of the device address space |
/// | IN | _dev_addr_ | Device address |
/// | IN | _perm_ | Memory permissions |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapDeviceAddressSpace>
///
/// # Safety
///
/// The caller must ensure:
/// - `out_mapped_size` is a valid, aligned pointer to writable memory
/// - Both `handle` and `proc_handle` are valid kernel handles owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn map_device_address_space(
    out_mapped_size: *mut u64,
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 on the stack
        "svc 0x5B",            // Issue the SVC call with immediate value 0x5B
        "ldr x2, [sp], #16",   // Load x2 from the stack
        "str x1, [x2]",        // Store x1 in the memory pointed to by x2
        "ret"
    );
}

/// Unmaps an attached device address space from an userspace address.
///
/// `Result svcUnmapDeviceAddressSpace(Handle handle, Handle proc_handle, u64 map_addr, uint64_t map_size, uint64_t dev_addr);`
///
/// Syscall code: [UNMAP_DEVICE_ADDRESS_SPACE](crate::code::UNMAP_DEVICE_ADDRESS_SPACE) (`0x5C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handle_ | Handle to the device address space |
/// | IN | _proc_handle_ | Process handle |
/// | IN | _map_addr_ | Mapped address |
/// | IN | _map_size_ | Size of the mapped region |
/// | IN | _dev_addr_ | Device address |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapDeviceAddressSpace>
///
/// # Safety
///
/// The caller must ensure that both `handle` and `proc_handle` are valid kernel handles
/// owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_device_address_space(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    map_size: u64,
    dev_addr: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x5C", // Issue the SVC call with immediate value 0x5C
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Cache Management">

/// Invalidates data cache for a virtual address range within a process.
///
/// `Result svcInvalidateProcessDataCache(Handle process, void* addr, uint64_t size);`
///
/// Syscall code: [INVALIDATE_PROCESS_DATA_CACHE](crate::code::INVALIDATE_PROCESS_DATA_CACHE) (`0x5D`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _process_ | Process handle |
/// | IN | _address_ | Virtual address |
/// | IN | _size_ | Size of the region |
///
/// Ref: <https://switchbrew.org/wiki/SVC#InvalidateProcessDataCache>
///
/// # Safety
///
/// The caller must ensure:
/// - `process` is a valid kernel process handle owned by the current process
/// - `address` points to a valid memory range in the target process
#[unsafe(naked)]
pub unsafe extern "C" fn invalidate_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x5D", // Issue the SVC call with immediate value 0x5D
        "ret"
    );
}

/// Stores data cache for a virtual address range within a process.
///
/// `Result svcStoreProcessDataCache(Handle process, void* addr, uint64_t size);`
///
/// Syscall code: [STORE_PROCESS_DATA_CACHE](crate::code::STORE_PROCESS_DATA_CACHE) (`0x5E`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _process_ | Process handle |
/// | IN | _address_ | Virtual address |
/// | IN | _size_ | Size of the region |
///
/// Ref: <https://switchbrew.org/wiki/SVC#StoreProcessDataCache>
///
/// # Safety
///
/// The caller must ensure:
/// - `process` is a valid kernel process handle owned by the current process
/// - `address` points to a valid memory range in the target process
#[unsafe(naked)]
pub unsafe extern "C" fn store_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x5E", // Issue the SVC call with immediate value 0x5E
        "ret"
    );
}

/// Flushes data cache for a virtual address range within a process.
///
/// `Result svcFlushProcessDataCache(Handle process, void* addr, uint64_t size);`
///
/// Syscall code: [FLUSH_PROCESS_DATA_CACHE](crate::code::FLUSH_PROCESS_DATA_CACHE) (`0x5F`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _process_ | Process handle |
/// | IN | _address_ | Virtual address |
/// | IN | _size_ | Size of the region |
///
/// Ref: <https://switchbrew.org/wiki/SVC#FlushProcessDataCache>
///
/// # Safety
///
/// The caller must ensure:
/// - `process` is a valid kernel process handle owned by the current process
/// - `address` points to a valid memory range in the target process
#[unsafe(naked)]
pub unsafe extern "C" fn flush_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x5F", // Issue the SVC call with immediate value 0x5F
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Debugging">

/// Debugs an active process.
///
/// `Result svcDebugActiveProcess(Handle* debug_handle, uint64_t process_id);`
///
/// Syscall code: [DEBUG_ACTIVE_PROCESS](crate::code::DEBUG_ACTIVE_PROCESS) (`0x60`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _debug_ | Output debug handle |
/// | IN | _process_id_ | Process ID to debug |
///
/// Ref: <https://switchbrew.org/wiki/SVC#DebugActiveProcess>
///
/// # Safety
///
/// The caller must ensure that `debug` is a valid, aligned pointer to writable memory
/// for the output debug handle.
#[unsafe(naked)]
pub unsafe extern "C" fn debug_active_process(debug: *mut Handle, process_id: u64) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (debug pointer) on stack
        "svc 0x60",            // Issue the SVC call with immediate value 0x60
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

/// Breaks an active debugging session.
///
/// `Result svcBreakDebugProcess(Handle debug);`
///
/// Syscall code: [BREAK_DEBUG_PROCESS](crate::code::BREAK_DEBUG_PROCESS) (`0x61`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _debug_ | Debug handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#BreakDebugProcess>
///
/// # Safety
///
/// The caller must ensure that `debug` is a valid debug handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn break_debug_process(debug: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x61", // Issue the SVC call with immediate value 0x61
        "ret"
    );
}

/// Terminates the process of an active debugging session.
///
/// `Result svcTerminateDebugProcess(Handle debug);`
///
/// Syscall code: [TERMINATE_DEBUG_PROCESS](crate::code::TERMINATE_DEBUG_PROCESS) (`0x62`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _debug_ | Debug handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#TerminateDebugProcess>
///
/// # Safety
///
/// The caller must ensure that `debug` is a valid debug handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn terminate_debug_process(debug: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x62", // Issue the SVC call with immediate value 0x62
        "ret"
    );
}

/// Gets an incoming debug event from a debugging session.
///
/// `Result svcGetDebugEvent(void* event_out, Handle debug);`
///
/// Syscall code: [GET_DEBUG_EVENT](crate::code::GET_DEBUG_EVENT) (`0x63`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _event_ | Debug event |
/// | IN | _debug_ | Debug handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetDebugEvent>
///
/// # Safety
///
/// The caller must ensure:
/// - `event` is a valid, aligned pointer to writable memory for the event result
/// - `debug` is a valid debug handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_debug_event(event: *mut c_void, debug: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (event pointer) on stack
        "svc 0x63",            // Issue the SVC call with immediate value 0x63
        "ldr x2, [sp], #16",   // Load x2 (saved event pointer) from stack
        "str w1, [x2]",        // Store w1 (event value) to *event
        "ret"
    );
}

/// Continues a debugging session. [3.0.0+]
///
/// `Result svcContinueDebugEvent(Handle debug, uint32_t flags, uint64_t* thread_ids, uint32_t num_thread_ids);`
///
/// Syscall code: [CONTINUE_DEBUG_EVENT](crate::code::CONTINUE_DEBUG_EVENT) (`0x64`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _debug_ | Debug handle |
/// | IN | _flags_ | Flags |
/// | IN | _tid_list_ | Thread IDs |
/// | IN | _num_tids_ | Number of thread IDs |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ContinueDebugEvent>
///
/// # Safety
///
/// The caller must ensure:
/// - `debug` is a valid debug handle owned by the current process
/// - `tid_list` points to a valid array of `num_tids` thread IDs
#[unsafe(naked)]
pub unsafe extern "C" fn continue_debug_event(
    debug: Handle,
    flags: u32,
    tid_list: *mut u64,
    num_tids: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x64", // Issue the SVC call with immediate value 0x64
        "ret"
    );
}

/// Continues a debugging session. [1.0.0-2.3.0]
///
/// `Result svcLegacyContinueDebugEvent(Handle debug, uint32_t flags, uint64_t thread_id);`
///
/// Syscall code: [CONTINUE_DEBUG_EVENT](crate::code::CONTINUE_DEBUG_EVENT) (`0x64`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _debug_ | Debug handle |
/// | IN | _flags_ | Flags |
/// | IN | _thread_id_ | Thread ID |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ContinueDebugEvent>
///
/// # Safety
///
/// The caller must ensure that `debug` is a valid debug handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn legacy_continue_debug_event(
    debug: Handle,
    flags: u32,
    thread_id: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x64", // Issue the SVC call with immediate value 0x64
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Process and thread management">

/// Retrieves a list of all running processes.
///
/// Fills the provided array with the pids of currently living processes. A process "lives" so long
/// as it is currently running or a handle to it still exists.
///
/// It returns the total number of processes currently alive. If this number is bigger than the size
/// of the provided buffer, the user won't have all the pids.
///
/// `Result svcGetProcessList(int32_t* pids_count, uint64_t* pids_list, uint32_t max_pids_count);`
///
/// Syscall code: [GET_PROCESS_LIST](crate::code::GET_PROCESS_LIST) (`0x65`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _pids_count_ | Number of processes written to the output array |
/// | OUT | _pids_list_ | Output array of process IDs |
/// | IN | _max_pids_count_ | Maximum number of process IDs to write |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetProcessList>
///
/// # Safety
///
/// The caller must ensure:
/// - `pids_count` is a valid, aligned pointer to writable memory for the count result
/// - `pids_list` points to a valid array of at least `max_pids_count` u64 values
#[unsafe(naked)]
pub unsafe extern "C" fn get_process_list(
    pids_count: *mut i32,
    pids_list: *mut u64,
    max_pids_count: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (pids_count pointer) on stack
        "svc 0x65",            // Issue the SVC call with immediate value 0x65
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (count result) to address in x2
        "ret"
    );
}

/// Retrieves a list of all threads for a debug handle (or zero).
///
/// `Result svcGetThreadList(int32_t* out_num_threads, uint64_t* out_thread_ids, uint32_t max_out_count, Handle debug);`
///
/// Syscall code: [GET_THREAD_LIST](crate::code::GET_THREAD_LIST) (`0x66`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _num_out_ | Number of threads written to the output array |
/// | OUT | _tids_out_ | Output array of thread IDs |
/// | IN | _max_tids_ | Maximum number of thread IDs to write. Typically, the buffer capacity |
/// | IN | _debug_ | Debug handle, or 0 to use the current process |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetThreadList>
///
/// # Safety
///
/// The caller must ensure:
/// - `num_out` is a valid, aligned pointer to writable memory for the count result
/// - `tids_out` points to a valid array of at least `max_tids` u64 values
/// - If `debug` is non-zero, it must be a valid debug handle
#[unsafe(naked)]
pub unsafe extern "C" fn get_thread_list(
    num_out: *mut i32,
    tids_out: *mut u64,
    max_tids: u32,
    debug: Handle,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (num_out pointer) on stack
        "svc 0x66",            // Issue the SVC call with immediate value 0x66
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (count result) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Debugging">

/// Gets the context (dump the registers) of a thread in a debugging session.
///
/// `Result svcGetDebugThreadContext(ThreadContext* out, Handle debug, uint64_t threadID, uint32_t flags);`
///
/// Syscall code: [GET_DEBUG_THREAD_CONTEXT](crate::code::GET_DEBUG_THREAD_CONTEXT) (`0x67`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _ctx_ | Output thread context |
/// | IN | _debug_ | Debug handle |
/// | IN | _thread_id_ | ID of the thread |
/// | IN | _flags_ | Context flags |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetDebugThreadContext>
///
/// # Safety
///
/// The caller must ensure:
/// - `ctx` is a valid, aligned pointer to writable memory for the thread context
/// - `debug` is a valid debug handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_debug_thread_context(
    ctx: *mut ThreadContext,
    debug: Handle,
    thread_id: u64,
    flags: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x67", // Issue the SVC call with immediate value 0x67
        "ret"
    );
}

/// Gets the context (dump the registers) of a thread in a debugging session.
///
/// `Result svcSetDebugThreadContext(Handle debug, uint64_t threadID, const ThreadContext* ctx, uint32_t flags);`
///
/// Syscall code: [SET_DEBUG_THREAD_CONTEXT](crate::code::SET_DEBUG_THREAD_CONTEXT) (`0x68`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _debug_ | Debug handle |
/// | IN | _thread_id_ | ID of the thread |
/// | IN | _ctx_ | Thread context to set |
/// | IN | _flags_ | Context flags |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetDebugThreadContext>
///
/// # Safety
///
/// The caller must ensure:
/// - `ctx` is a valid, aligned pointer to readable memory containing the thread context
/// - `debug` is a valid debug handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn set_debug_thread_context(
    debug: Handle,
    thread_id: u64,
    ctx: *mut ThreadContext,
    flags: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x68", // Issue the SVC call with immediate value 0x68
        "ret"
    );
}

/// Queries memory information from a process that is being debugged.
///
/// `Result svcQueryDebugProcessMemory(MemoryInfo* out, uint32_t* out_page_info, Handle debug, uint64_t addr);`
///
/// Syscall code: [QUERY_DEBUG_PROCESS_MEMORY](crate::code::QUERY_DEBUG_PROCESS_MEMORY) (`0x69`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _meminfo_ptr_ | Output memory info |
/// | OUT | _pageinfo_ | Output page info |
/// | IN | _debug_ | Debug handle |
/// | IN | _addr_ | Address to query |
///
/// Ref: <https://switchbrew.org/wiki/SVC#QueryDebugProcessMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - Both `meminfo_ptr` and `pageinfo` are valid, aligned pointers to writable memory
/// - `debug` is a valid debug handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn query_debug_process_memory(
    meminfo_ptr: *mut MemoryInfo,
    pageinfo: *mut u32,
    debug: Handle,
    addr: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x1, [sp, #-16]!", // Store x1 (pageinfo pointer) on stack
        "svc 0x69",            // Issue the SVC call with immediate value 0x69
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (page info result) to address in x2
        "ret"
    );
}

///  Reads memory from a process that is being debugged.
///
/// `Result svcReadDebugProcessMemory(void* buffer, Handle debug, uint64_t addr, uint64_t size);`
///
/// Syscall code: [READ_DEBUG_PROCESS_MEMORY](crate::code::READ_DEBUG_PROCESS_MEMORY) (`0x6A`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _buffer_ | Output buffer |
/// | IN | _debug_ | Debug handle |
/// | IN | _addr_ | Address to read from |
/// | IN | _size_ | Size to read |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ReadDebugProcessMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `buffer` points to writable memory of at least `size` bytes
/// - `debug` is a valid debug handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn read_debug_process_memory(
    buffer: *mut c_void,
    debug: Handle,
    addr: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x6A", // Issue the SVC call with immediate value 0x6A
        "ret"
    );
}

/// Writes to memory in a process that is being debugged.
///
/// `Result svcWriteDebugProcessMemory(Handle debug, const void* buffer, uint64_t addr, uint64_t size);`
///
/// Syscall code: [WRITE_DEBUG_PROCESS_MEMORY](crate::code::WRITE_DEBUG_PROCESS_MEMORY) (`0x6B`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _debug_ | Debug handle |
/// | IN | _buffer_ | Input buffer |
/// | IN | _addr_ | Address to write to |
/// | IN | _size_ | Size to write |
///
/// Ref: <https://switchbrew.org/wiki/SVC#WriteDebugProcessMemory>
///
/// # Safety
///
/// The caller must ensure:
/// - `buffer` points to readable memory of at least `size` bytes
/// - `debug` is a valid debug handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn write_debug_process_memory(
    debug: Handle,
    buffer: *const c_void,
    addr: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x6B", // Issue the SVC call with immediate value 0x6B
        "ret"
    );
}

/// Sets one of the hardware breakpoints.
///
/// `Result svcSetHardwareBreakpoint(uint32_t which, uint64_t flags, uint64_t value);`
///
/// Syscall code: [SET_HARDWARE_BREAKPOINT](crate::code::SET_HARDWARE_BREAKPOINT) (`0x6C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _which_ | Hardware breakpoint slot to use |
/// | IN | _flags_ | Breakpoint flags |
/// | IN | _value_ | Breakpoint value |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetHardwareBreakPoint>
///
/// # Safety
///
/// This function is safe to call from a debugging context.
#[unsafe(naked)]
pub unsafe extern "C" fn set_hardware_breakpoint(which: u32, flags: u64, value: u64) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x6C", // Issue the SVC call with immediate value 0x6C
        "ret"
    );
}

///  Gets parameters from a thread in a debugging session.
///
/// `Result svcGetDebugThreadParam(uint64_t* out_64, uint32_t* out_32, Handle debug, uint64_t threadID, DebugThreadParam param);`
///
/// Syscall code: [GET_DEBUG_THREAD_PARAM](crate::code::GET_DEBUG_THREAD_PARAM) (`0x6D`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_64_ | Output 64-bit value |
/// | OUT | _out_32_ | Output 32-bit value |
/// | IN | _debug_ | Debug handle |
/// | IN | _thread_id_ | Thread ID |
/// | IN | _param_ | Parameter to get |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetDebugThreadParam>
///
/// # Safety
///
/// The caller must ensure:
/// - Both `out_64` and `out_32` are valid, aligned pointers to writable memory
/// - `debug` is a valid debug handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_debug_thread_param(
    out_64: *mut u64,
    out_32: *mut u32,
    debug: Handle,
    thread_id: u64,
    param: DebugThreadParam,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x0, x1, [sp, #-16]!", // Store out_64 and out_32 on stack
        "svc 0x6D",                // Issue the SVC call with immediate value 0x6D
        "ldp x3, x4, [sp], #16",   // Load out_64 and out_32 from stack
        "str x1, [x3]",            // Store x1 (out_64) to address in x3
        "str w2, [x4]",            // Store w2 (out_32) to address in x4
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Miscellaneous">

/// Retrieves privileged information about the system, or a certain kernel object.
///
/// `Result svcGetSystemInfo(u64* out, uint64_t id0, Handle handle, uint64_t id1);`
///
/// Syscall code: [GET_SYSTEM_INFO](crate::code::GET_SYSTEM_INFO) (`0x6F`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Output value |
/// | IN | _id0_ | First ID |
/// | IN | _handle_ | Handle |
/// | IN | _id1_ | Second ID |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetSystemInfo>
///
/// # Safety
///
/// The caller must ensure:
/// - `out` is a valid, aligned pointer to writable memory for the result
/// - If `handle` is non-zero, it must be a valid kernel handle
#[unsafe(naked)]
pub unsafe extern "C" fn get_system_info(
    out: *mut u64,
    id0: u64,
    handle: Handle,
    id1: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc 0x6F",            // Issue the SVC call with immediate value 0x6F
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (result value) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Inter-process communication (IPC)">

/// Creates a port.
///
/// `Result svcCreatePort(Handle* server_handle, Handle* client_handle, int32_t max_sessions, bool is_light, const char* name);`
///
/// Syscall code: [CREATE_PORT](crate::code::CREATE_PORT) (`0x70`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _port_server_ | Output server handle |
/// | OUT | _port_client_ | Output client handle |
/// | IN | _max_sessions_ | Maximum number of sessions |
/// | IN | _is_light_ | Whether to create a light port |
/// | IN | _name_ | Name of the port |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreatePort>
///
/// # Safety
///
/// The caller must ensure:
/// - Both `port_server` and `port_client` are valid, aligned pointers to writable memory
/// - `name` points to a null-terminated C string that is valid and readable
#[unsafe(naked)]
pub unsafe extern "C" fn create_port(
    port_server: *mut Handle,
    port_client: *mut Handle,
    max_sessions: i32,
    is_light: bool,
    name: *const c_char,
) -> ResultCode {
    core::arch::naked_asm!(
        "stp x0, x1, [sp, #-16]!", // Store port_server and port_client on stack
        "svc 0x70",                // Issue the SVC call with immediate value 0x70
        "ldp x3, x4, [sp], #16",   // Load port_server and port_client from stack
        "str w1, [x3]",            // Store w1 (port_server) to address in x3
        "str w2, [x4]",            // Store w2 (port_client) to address in x4
        "ret"
    );
}

/// Manages a named port.
///
/// `Result svcManageNamedPort(Handle* server_handle, const char* name, int32_t maxSessions);`
///
/// Syscall code: [MANAGE_NAMED_PORT](crate::code::MANAGE_NAMED_PORT) (`0x71`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _port_server_ | Output server handle |
/// | IN | _name_ | Name of the port |
/// | IN | _max_sessions_ | Maximum number of sessions |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ManageNamedPort>
///
/// # Safety
///
/// The caller must ensure:
/// - `port_server` is a valid, aligned pointer to writable memory for the output handle
/// - `name` points to a null-terminated C string that is valid and readable
#[unsafe(naked)]
pub unsafe extern "C" fn manage_named_port(
    port_server: *mut Handle,
    name: *const c_char,
    max_sessions: i32,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store port_server on stack
        "svc 0x71",            // Issue the SVC call with immediate value 0x71
        "ldr x2, [sp], #16",   // Load port_server from stack
        "str w1, [x2]",        // Store w1 (port_server) to address in x2
        "ret"
    );
}

/// Connects to a port.
///
/// `Result svcConnectToPort(Handle* session, Handle port);`
///
/// Syscall code: [CONNECT_TO_PORT](crate::code::CONNECT_TO_PORT) (`0x72`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _session_ | Output session handle |
/// | IN | _port_ | Port handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#ConnectToPort>
///
/// # Safety
///
/// The caller must ensure that `session` is a valid, aligned pointer to writable memory
/// where a Handle can be stored.
#[unsafe(naked)]
pub unsafe extern "C" fn connect_to_port(session: *mut Handle, port: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (session pointer) on stack
        "svc 0x72",            // Issue the SVC call with immediate value 0x72
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (session handle) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Memory Management">

/// Sets the memory permissions for the specified memory with the supplied process handle.
///
/// `Result svcSetProcessMemoryPermission(Handle proc, uint64_t addr, uint64_t size, uint32_t perm);`
///
/// Syscall code: [SET_PROCESS_MEMORY_PERMISSION](crate::code::SET_PROCESS_MEMORY_PERMISSION) (`0x73`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _proc_ | Process handle |
/// | IN | _addr_ | Target memory address |
/// | IN | _size_ | Size of the target memory |
/// | IN | _perm_ | New memory permissions |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetProcessMemoryPermission>
///
/// # Safety
///
/// The caller must ensure that `proc` is a valid kernel process handle owned by the
/// current process, and that the memory region defined by `addr` and `size` is valid
/// within the target process's address space.
#[unsafe(naked)]
pub unsafe extern "C" fn set_process_memory_permission(
    proc: Handle,
    addr: u64,
    size: u64,
    perm: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x73", // Issue the SVC call with immediate value 0x73
        "ret"
    );
}

/// Maps the src address from the supplied process handle into the current process.
///
/// `Result svcMapProcessMemory(void* dst, Handle proc, uint64_t src, uint64_t size);`
///
/// Syscall code: [MAP_PROCESS_MEMORY](crate::code::MAP_PROCESS_MEMORY) (`0x74`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _dst_ | Destination address |
/// | IN | _proc_ | Process handle |
/// | IN | _src_ | Source address |
/// | IN | _size_ | Size of the memory to map |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapProcessMemory>
///
/// # Safety
///
/// The caller must ensure that:
/// - `dst` is a valid, aligned pointer to writable memory that can hold a mapping of the
///   specified size
/// - `proc` is a valid kernel process handle owned by the current process
/// - The memory region defined by `src` and `size` is valid in the target process's address space
#[unsafe(naked)]
pub unsafe extern "C" fn map_process_memory(
    dst: *mut c_void,
    proc: Handle,
    src: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x74", // Issue the SVC call with immediate value 0x74
        "ret"
    );
}

/// Undoes the effects of [map_process_memory].
///
/// `Result svcUnmapProcessMemory(void* dst, Handle proc, uint64_t src, uint64_t size);`
///
/// Syscall code: [UNMAP_PROCESS_MEMORY](crate::code::UNMAP_PROCESS_MEMORY) (`0x75`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _dst_ | Destination mapping address |
/// | IN | _proc_ | Process handle |
/// | IN | _src_ | Address of the memory in the process |
/// | IN | _size_ | Size of the memory |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapProcessMemory>
///
/// # Safety
///
/// The caller must ensure that:
/// - `dst` is a valid, aligned pointer to the memory region being unmapped
/// - `proc` is a valid kernel process handle owned by the current process
/// - The mapping was previously established by a call to [map_process_memory] with the same
///   `dst` and `size` parameters
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_process_memory(
    dst: *mut c_void,
    proc: Handle,
    src: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x75", // Issue the SVC call with immediate value 0x75
        "ret"
    );
}

/// Equivalent to [query_memory], for another process.
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcQueryProcessMemory(MemoryInfo* meminfo_ptr, uint32_t *pageinfo, Handle proc, uint64_t addr);`
///
/// Syscall code: [QUERY_PROCESS_MEMORY](crate::code::QUERY_PROCESS_MEMORY) (`0x76`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _meminfo_ptr_ | [MemoryInfo] structure which will be filled in |
/// | OUT | _pageinfo_ | Page information which will be filled in |
/// | IN | _proc_ | Process handle |
/// | IN | _addr_ | Address to query |
///
/// Ref: <https://switchbrew.org/wiki/SVC#QueryProcessMemory>
///
/// # Safety
///
/// The caller must ensure that:
/// - `meminfo_ptr` is a valid, aligned pointer to writable memory for a MemoryInfo structure
/// - `pageinfo` is a valid, aligned pointer to writable memory for a u32
/// - `proc` is a valid kernel process handle owned by the current process
/// - The address `addr` is valid to query within the target process's address space
#[unsafe(naked)]
pub unsafe extern "C" fn query_process_memory(
    meminfo_ptr: *mut MemoryInfo,
    pageinfo: *mut u32,
    proc: Handle,
    addr: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x1, [sp, #-16]!", // Store pageinfo on stack
        "svc 0x76",            // Issue the SVC call with immediate value 0x76
        "ldr x2, [sp], #16",   // Load pageinfo from stack
        "str w1, [x2]",        // Store w1 (pageinfo) to address in x2
        "ret"
    );
}

/// Maps normal heap in a certain process as executable code (used when loading NROs).
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcMapProcessCodeMemory(Handle proc, u64 dst, uint64_t src, uint64_t size);`
///
/// Syscall code: [MAP_PROCESS_CODE_MEMORY](crate::code::MAP_PROCESS_CODE_MEMORY) (`0x77`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _proc_ | Process handle (cannot be [CUR_PROCESS_HANDLE]) |
/// | IN | _dst_ | Destination mapping address |
/// | IN | _src_ | Source mapping address |
/// | IN | _size_ | Size of the mapping |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapProcessCodeMemory>
///
/// # Safety
///
/// The caller must ensure that:
/// - `proc` is a valid kernel process handle owned by the current process (cannot be CUR_PROCESS_HANDLE)
/// - `dst` and `src` are valid addresses within the target process's address space
/// - The memory region defined by `src` and `size` is valid and readable in the target process
/// - The destination region is valid and writable in the target process
#[unsafe(naked)]
pub unsafe extern "C" fn map_process_code_memory(
    proc: Handle,
    dst: u64,
    src: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x77", // Issue the SVC call with immediate value 0x77
        "ret"
    );
}

/// Undoes the effects of [map_process_code_memory].
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcUnmapProcessCodeMemory(Handle proc, u64 dst, uint64_t src, uint64_t size);`
///
/// Syscall code: [UNMAP_PROCESS_CODE_MEMORY](crate::code::UNMAP_PROCESS_CODE_MEMORY) (`0x78`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _proc_ | Process handle (cannot be [CUR_PROCESS_HANDLE]) |
/// | IN | _dst_ | Destination mapping address |
/// | IN | _src_ | Source mapping address |
/// | IN | _size_ | Size of the mapping |
///
/// Ref: <https://switchbrew.org/wiki/SVC#UnmapProcessCodeMemory>
///
/// # Safety
///
/// The caller must ensure that:
/// - `proc` is a valid kernel process handle owned by the current process (cannot be CUR_PROCESS_HANDLE)
/// - `dst` and `src` are valid addresses within the target process's address space
/// - The mapping was previously established by a call to [map_process_code_memory] with the same
///   process, addresses, and size
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_process_code_memory(
    proc: Handle,
    dst: u64,
    src: u64,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x78", // Issue the SVC call with immediate value 0x78
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Process and thread management">

/// Creates a new process.
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcCreateProcess(Handle* out, const void* proc_info, const uint32_t* caps, uint64_t cap_num);`
///
/// Syscall code: [CREATE_PROCESS](crate::code::CREATE_PROCESS) (`0x79`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Output handle for the created process |
/// | IN | _proc_info_ | Process information |
/// | IN | _caps_ | Array of kernel capabilities |
/// | IN | _cap_num_ | Number of kernel capabilities |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateProcess>
///
/// # Safety
///
/// The caller must ensure:
/// - `out` is a valid, aligned pointer to writable memory for the output handle
/// - `proc_info` points to valid process information structure
/// - `caps` points to a valid array of `cap_num` capability descriptors
#[unsafe(naked)]
pub unsafe extern "C" fn create_process(
    out: *mut Handle,
    proc_info: *const u8,
    caps: *const u32,
    cap_num: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc 0x79",            // Issue the SVC call with immediate value 0x79
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

/// Starts executing a freshly created process.
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// Syscall code: [START_PROCESS](crate::code::START_PROCESS) (`0x7A`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _proc_ | Process handle |
/// | IN | _main_prio_ | Priority of the main thread |
/// | IN | _default_cpu_ | ID of the default CPU core |
/// | IN | _stack_size_ | Stack size for the main thread |
///
/// Ref: <https://switchbrew.org/wiki/SVC#StartProcess>
///
/// # Safety
///
/// The caller must ensure that `proc` is a valid kernel process handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn start_process(
    proc: Handle,
    main_prio: i32,
    default_cpu: i32,
    stack_size: u32,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x7A", // Issue the SVC call with immediate value 0x7A
        "ret"
    );
}

/// Terminates a running process.
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcTerminateProcess(Handle proc);`
///
/// Syscall code: [TERMINATE_PROCESS](crate::code::TERMINATE_PROCESS) (`0x7B`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _proc_ | Handle of the process to terminate |
///
/// Ref: <https://switchbrew.org/wiki/SVC#TerminateProcess>
#[unsafe(naked)]
/// # Safety
///
/// The caller must ensure that `proc` is a valid kernel process handle owned by the current process.
pub unsafe extern "C" fn terminate_process(proc: Handle) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x7B", // Issue the SVC call with immediate value 0x7B
        "ret"
    );
}

/// Gets a [ProcessInfoType] for a process.
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `Result svcGetProcessInfo(int64_t *out, Handle proc, ProcessInfoType which);`
///
/// Syscall code: [GET_PROCESS_INFO](crate::code::GET_PROCESS_INFO) (`0x7C`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Output buffer for the process information |
/// | IN | _proc_ | Handle of the process to get information from |
/// | IN | _which_ | Type of information to retrieve |
///
/// Ref: <https://switchbrew.org/wiki/SVC#GetProcessInfo>
///
/// # Safety
///
/// The caller must ensure:
/// - `out` is a valid, aligned pointer to writable memory for the process info result
/// - `proc` is a valid kernel process handle owned by the current process
#[unsafe(naked)]
pub unsafe extern "C" fn get_process_info(
    out: *mut i64,
    proc: Handle,
    which: ProcessInfoType,
) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc 0x7C",            // Issue the SVC call with immediate value 0x7C
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str x1, [x2]",        // Store x1 (result value) to address in x2
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Resource Limit Management">

/// Creates a new Resource Limit handle.
///
/// `Result svcCreateResourceLimit(Handle* out);`
///
/// Syscall code: [CREATE_RESOURCE_LIMIT](crate::code::CREATE_RESOURCE_LIMIT) (`0x7D`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | OUT | _out_ | Output resource limit handle |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateResourceLimit>
///
/// # Safety
///
/// The caller must ensure that `out` is a valid, aligned pointer to writable memory
/// for the output handle.
#[unsafe(naked)]
pub unsafe extern "C" fn create_resource_limit(out: *mut Handle) -> ResultCode {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Store x0 (out pointer) on stack
        "svc 0x7D",            // Issue the SVC call with immediate value 0x7D
        "ldr x2, [sp], #16",   // Load x2 from stack
        "str w1, [x2]",        // Store w1 (handle result) to address in x2
        "ret"
    );
}

/// Sets the value for a [LimitableResource] for a Resource Limit handle.
///
/// `Result svcSetResourceLimitLimitValue(Handle reslimit, LimitableResource which, uint64_t value);`
///
/// Syscall code: [SET_RESOURCE_LIMIT_LIMIT_VALUE](crate::code::SET_RESOURCE_LIMIT_LIMIT_VALUE) (`0x7E`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _reslimit_ | Resource limit handle |
/// | IN | _which_ | Resource to set the value of |
/// | IN | _value_ | Value to set |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetResourceLimitLimitValue>
///
/// # Safety
///
/// The caller must ensure that `reslimit` is a valid resource limit handle owned by the current process.
#[unsafe(naked)]
pub unsafe extern "C" fn set_resource_limit_limit_value(
    reslimit: Handle,
    which: LimitableResource,
    value: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x7E", // Issue the SVC call with immediate value 0x7E
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Secure Monitor">

/// Calls a secure monitor function (TrustZone, EL3).
///
/// <div class="warning">
/// This is a privileged syscall. Use envIsSyscallHinted to check if it is available.
/// </div>
///
/// `void svcCallSecureMonitor(SecmonArgs* regs);`
///
/// Syscall code: [CALL_SECURE_MONITOR](crate::code::CALL_SECURE_MONITOR) (`0x7F`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _regs_ | Arguments to pass to the secure monitor |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CallSecureMonitor>
///
/// # Safety
///
/// The caller must ensure that `regs` is a valid, aligned pointer to a SecmonArgs structure
/// that will be read from and written to by the secure monitor.
#[unsafe(naked)]
pub unsafe extern "C" fn call_secure_monitor(regs: *mut SecmonArgs) {
    core::arch::naked_asm!(
        "str x0, [sp, #-16]!", // Push regs pointer on stack
        "mov x8, x0",          // Copy regs pointer to x8 for load phase
        // Load arguments into x0..x7
        "ldp x0, x1, [x8]",        // x0,x1 = regs->x[0], x[1]
        "ldp x2, x3, [x8, #0x10]", // x2,x3 = regs->x[2], x[3]
        "ldp x4, x5, [x8, #0x20]", // x4,x5 = regs->x[4], x[5]
        "ldp x6, x7, [x8, #0x30]", // x6,x7 = regs->x[6], x[7]
        "svc 0x7F",                // Secure monitor call
        // Restore original regs pointer from stack
        "ldr x8, [sp], #16", // Pop regs pointer into x8
        // Store results back to struct
        "stp x0, x1, [x8]",        // regs->x[0] = x0, x1
        "stp x2, x3, [x8, #0x10]", // regs->x[2] = x2, x3
        "stp x4, x5, [x8, #0x20]", // regs->x[4] = x4, x5
        "stp x6, x7, [x8, #0x30]", // regs->x[6] = x6, x7
        "ret"
    );
}

//</editor-fold>

//<editor-fold desc="Memory Management">

/// Maps new insecure memory at the desired address. [15.0.0+]
///
/// `Result svcMapInsecurePhysicalMemory(void *address, uint64_t size);`
///
/// Syscall code: [MAP_INSECURE_PHYSICAL_MEMORY](crate::code::MAP_INSECURE_PHYSICAL_MEMORY) (`0x90`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address to map memory at |
/// | IN | _size_ | Size of memory to map |
///
/// Ref: <https://switchbrew.org/wiki/SVC#:~:text=0x90,MapInsecurePhysicalMemory>
///
/// # Safety
///
/// The caller must ensure that `address` is a valid, aligned pointer to writable memory
/// that can hold the mapped physical memory region of the specified size.
#[unsafe(naked)]
pub unsafe extern "C" fn map_insecure_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x90", // Issue the SVC call with immediate value 0x90
        "ret"
    );
}

/// Undoes the effects of [map_insecure_physical_memory]. [15.0.0+]
///
/// `Result svcUnmapInsecurePhysicalMemory(void *address, uint64_t size);`
///
/// Syscall code: [UNMAP_INSECURE_PHYSICAL_MEMORY](crate::code::UNMAP_INSECURE_PHYSICAL_MEMORY) (`0x91`).
///
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _address_ | Address to unmap memory from |
/// | IN | _size_ | Size of memory to unmap |
///
/// Ref: <https://switchbrew.org/wiki/SVC#:~:text=0x91,UnmapInsecurePhysicalMemory>
///
/// # Safety
///
/// The caller must ensure that `address` is a valid, aligned pointer that was previously
/// mapped by a call to [map_insecure_physical_memory] with the same size parameter.
#[unsafe(naked)]
pub unsafe extern "C" fn unmap_insecure_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    core::arch::naked_asm!(
        "svc 0x91", // Issue the SVC call with immediate value 0x91
        "ret"
    );
}

//</editor-fold>
