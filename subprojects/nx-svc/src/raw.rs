//! Raw _Supervisor Call (SVC)_ API.

use core::{
    arch::asm,
    ffi::{c_char, c_int, c_void},
};

use crate::result::ResultCode;

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
#[repr(C)]
pub struct MemoryInfo {
    /// Base address
    pub addr: u64,
    /// Size
    pub size: u64,
    /// Memory type (see lower 8 bits of [MemoryState])
    // TODO: MemoryState bitfield
    pub type_: u32,
    /// Memory attributes (see [MemoryAttribute])
    // TODO: MemoryAttribute bitfield
    pub attr: u32,
    /// Memory permissions (see [MemoryPermission])
    // TODO: MemoryPermission bitfield
    pub perm: u32,
    /// IPC reference count
    pub ipc_refcount: u32,
    /// Device reference count
    pub device_refcount: u32,
    /// Padding
    pub padding: u32,
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_heap_size(
    out_addr: *mut *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 (out_addr) on the stack
            "svc 0x1",             // Issue the SVC call with immediate value 0x1
            "ldr x2, [sp], #16",   // Load x2 (out_addr pointer) from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") out_addr,     // Input: out_addr in register x0
            in("x1") size,         // Input: size in register x1
            lateout("w0") result,  // Output: Capture result from x0
            options(nostack)
        );
    }
    result
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
/// | IN | _perm_ | Permissions (see [MemoryPermission]). |
///
/// Ref: <https://switchbrew.org/wiki/SVC#SetMemoryPermission>
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_memory_permission(
    addr: *mut c_void,
    size: usize,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x2",            // Issue the SVC call with immediate value 0x2
            in("x0") addr,        // Input: addr in register x0
            in("x1") size,        // Input: size in register x1
            in("x2") perm,        // Input: perm in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_memory_attribute(
    addr: *mut c_void,
    size: usize,
    mask: u32,
    attr: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x3",            // Issue the SVC call with immediate value 0x3
            in("x0") addr,        // Input: addr in register x0
            in("x1") size,        // Input: size in register x1
            in("x2") mask,        // Input: mask in register x2
            in("x3") attr,        // Input: attr in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x4",            // Issue the SVC call with immediate value 0x4
            in("x0") dst_addr,    // Input: addr in register x0
            in("x1") src_addr,    // Input: addr in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Unmaps a region that was previously mapped with [`__nx_svc_map_memory`].
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x5",            // Issue the SVC call with immediate value 0x5
            in("x0") dst_addr,    // Input: addr in register x0
            in("x1") src_addr,    // Input: addr in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_query_memory(
    meminfo: *mut MemoryInfo,
    pageinfo: *mut u32,
    addr: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x1, [sp, #-16]!", // Store x1 on the stack
            "svc 0x6",             // Issue the SVC call with immediate value 0x6
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") meminfo,      // Input: meminfo in register x0
            in("x1") pageinfo,     // Input: pageinfo in register x1
            in("x2") addr,         // Input: addr in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_exit_process() -> ! {
    unsafe {
        asm!(
            "svc 0x7",         // Issue the SVC call with immediate value 0x7
            options(noreturn)  // Never returns, and its return type is defined as ! (never).
        )
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_thread(
    handle: *mut Handle,
    entry: *mut c_void,
    arg: *mut c_void,
    stack_top: *mut c_void,
    prio: c_int,
    cpuid: c_int,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 (handle pointer) on the stack
            "svc 0x8",             // Issue the SVC call with immediate value 0x8
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 (thread handle) in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") entry,        // Input: entry in register x1
            in("x2") arg,          // Input: arg in register x2
            in("x3") stack_top,    // Input: stack_top in register x3
            in("w4") prio,         // Input: prio in register w4
            in("w5") cpuid,        // Input: cpuid in register w5
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_start_thread(handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x9",            // Issue the SVC call with immediate value 0x9
            in("x0") handle,      // Input: handle in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Exits the current thread.
///
/// `void NX_NORETURN svcExitThread(void);`
///
/// Syscall code: [EXIT_THREAD](crate::code::EXIT_THREAD) (`0xA`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#ExitThread>
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_exit_thread() -> ! {
    unsafe {
        asm!(
            "svc 0xA",         // Issue the SVC call with immediate value 0xA
            options(noreturn)  // Never returns, and its return type is defined as ! (never).
        )
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_sleep_thread(nano: i64) {
    unsafe {
        asm!(
            "svc 0xB",     // Issue the SVC call with immediate value 0xB
            in("x0") nano, // Input: nano in register x0
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_thread_priority(
    priority: *mut i32,
    handle: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0xC",             // Issue the SVC call with immediate value 0xC
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") priority,     // Input: priority in register x0
            in("x1") handle,       // Input: handle in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_thread_priority(handle: Handle, priority: u32) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0xD",            // Issue the SVC call with immediate value 0xD
            in("x0") handle,      // Input: handle in register x0
            in("x1") priority,    // Input: priority in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_thread_core_mask(
    core_id: *mut i32,
    affinity_mask: *mut u64,
    handle: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
            "svc 0xE",                 // Issue the SVC call with immediate value 0xE
            "ldp x3, x4, [sp], #16",   // Load x3 and x4 from the stack
            "str w1, [x3]",            // Store w1 in the memory pointed to by x3
            "str x2, [x4]",            // Store x2 in the memory pointed to by x4
            in("x0") core_id,          // Input: core_id in register x0
            in("x1") affinity_mask,    // Input: affinity_mask in register x1
            in("x2") handle,           // Input: handle in register x2
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_thread_core_mask(
    handle: Handle,
    core_id: i32,
    affinity_mask: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0xF",              // Issue the SVC call with immediate value 0xF
            in("x0") handle,        // Input: handle in register x0
            in("x1") core_id,       // Input: core_id in register x1
            in("x2") affinity_mask, // Input: affinity_mask in register x2
            lateout("w0") result,   // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_current_processor_number() -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x10",           // Issue the SVC call with immediate value 0x10
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

//</editor-fold>

//<editor-fold desc="Synchronization">

/// Puts the given event in the signaled state.
///
/// Will wake up any thread currently waiting on this event. Can potentially trigger a re-schedule.
///
/// Any calls to [__nx_svc_wait_synchronization] on this handle will return immediately, until the
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_signal_event(handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x11",           // Issue the SVC call with immediate value 0x11
            in("x0") handle,      // Input: handle in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_clear_event(handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x12",           // Issue the SVC call with immediate value 0x12
            in("x0") handle,      // Input: handle in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
/// | IN | _perm_ | Permissions (see [MemoryPermission]). |
///
/// Ref: <https://switchbrew.org/wiki/SVC#MapSharedMemory>
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x13",           // Issue the SVC call with immediate value 0x13
            in("x0") handle,      // Input: handle in register x0
            in("x1") addr,        // Input: addr in register x1
            in("x2") size,        // Input: size in register x2
            in("x3") perm,        // Input: perm in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x14",           // Issue the SVC call with immediate value 0x14
            in("x0") handle,      // Input: handle in register x0
            in("x1") addr,        // Input: addr in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
/// | IN | _perm_ | Permissions (see [MemoryPermission]). |
///
/// Ref: <https://switchbrew.org/wiki/SVC#CreateTransferMemory>
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_transfer_memory(
    handle: *mut Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x15",            // Issue the SVC call with immediate value 0x15
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") addr,         // Input: addr in register x1
            in("x2") size,         // Input: size in register x2
            in("x3") perm,         // Input: perm in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_close_handle(handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x16",           // Issue the SVC call with immediate value 0x16
            in("x0") handle,      // Input: handle in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_reset_signal(handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x17",           // Issue the SVC call with immediate value 0x17
            in("x0") handle,      // Input: handle in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_wait_synchronization(
    index: *mut i32,
    handles: *const u32,
    handle_count: i32,
    timeout: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x18",            // Issue the SVC call with immediate value 0x18
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") index,        // Input: index in register x0
            in("x1") handles,      // Input: handles in register x1
            in("x2") handle_count, // Input: handle_count in register x2
            in("x3") timeout,      // Input: timeout in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Waits a [__nx_svc_wait_synchronization] operation being done on a synchronization object in
/// another thread.
///
/// If the referenced thread is currently in a synchronization call ([__nx_svc_wait_synchronization],
/// [__nx_svc_reply_and_receive] or [__nx_svc_reply_and_receive_light]), that call will be
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_cancel_synchronization(handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x19",           // Issue the SVC call with immediate value 0x19
            in("x0") handle,      // Input: handle in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_arbitrate_lock(
    owner_thread_handle: Handle,
    mutex: *mut u32,
    curr_thread_handle: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x1A",                   // Issue the SVC call with immediate value 0x1A
            in("x0") owner_thread_handle, // Input: owner_thread_handle in register x0
            in("x1") mutex,               // Input: mutex in register x1
            in("x2") curr_thread_handle,  // Input: curr_thread_handle in register x2
            lateout("w0") result,         // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_arbitrate_unlock(mutex: *mut u32) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x1B",            // Issue the SVC call with immediate value 0x1B
            in("x0") mutex,        // Input: mutex in register x0
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_wait_process_wide_key_atomic(
    address: *mut u32,
    cv_key: *mut u32,
    tag: u32,
    timeout_ns: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x1C",           // Issue the SVC call with immediate value 0x1C
            in("x0") address,     // Input: address in register x0
            in("x1") cv_key,      // Input: cv_key in register x1
            in("x2") tag,         // Input: tag in register x2
            in("x3") timeout_ns,  // Input: timeout_ns in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_signal_process_wide_key(cv_key: *mut u32, count: i32) {
    unsafe {
        asm!(
            "svc 0x1D",      // Issue the SVC call with immediate value 0x1D
            in("x0") cv_key, // Input: cv_key in register x0
            in("x1") count,  // Input: count in register x1
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_system_tick() -> u64 {
    let result: u64;
    unsafe {
        asm!(
            "svc 0x1E",       // Issue the SVC call with immediate value 0x1E
            out("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_connect_to_named_port(
    session: *mut Handle,
    name: *const c_char,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x1F",            // Issue the SVC call with immediate value 0x1F
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") session,      // Input: session in register x0
            in("x1") name,         // Input: name in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_send_sync_request_light(session: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x20",           // Issue the SVC call with immediate value 0x20
            in("x0") session,     // Input: session in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_send_sync_request(session: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x21",           // Issue the SVC call with immediate value 0x21
            in("x0") session,     // Input: session in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_send_sync_request_with_user_buffer(
    usr_buffer: *mut c_void,
    size: u64,
    session: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x22",           // Issue the SVC call with immediate value 0x22
            in("x0") usr_buffer,  // Input: usr_buffer in register x0
            in("x1") size,        // Input: size in register x1
            in("x2") session,     // Input: session in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_send_async_request_with_user_buffer(
    handle: *mut Handle,
    usr_buffer: *mut c_void,
    size: u64,
    session: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x23",            // Issue the SVC call with immediate value 0x23
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") usr_buffer,   // Input: usr_buffer in register x1
            in("x2") size,         // Input: size in register x2
            in("x3") session,      // Input: session in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_process_id(
    process_id: *mut u64,
    handle: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x24",            // Issue the SVC call with immediate value 0x24
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") process_id,   // Input: process_id in register x0
            in("x1") handle,       // Input: handle in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_thread_id(thread_id: *mut u64, handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x25",            // Issue the SVC call with immediate value 0x25
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") thread_id,    // Input: thread_id in register x0
            in("x1") handle,       // Input: handle in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_break(
    reason: BreakReason,
    address: *mut c_void, // TODO: Review uintptr_t
    size: usize,          // TODO: Review uintptr_t
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x26",             // Issue the SVC call with immediate value 0x26
            in("x0") reason as u32, // Input: break_reason in register x0
            in("x1") address,       // Input: address in register x1
            in("x2") size,          // Input: size in register x2
            lateout("w0") result,   // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_output_debug_string(
    dbg_str: *const c_char,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x27",           // Issue the SVC call with immediate value 0x27
            in("x0") dbg_str,     // Input: dbg_str in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_return_from_exception(res: ResultCode) -> ! {
    unsafe {
        asm!(
            "svc 0x28",       // Issue the SVC call with immediate value 0x28
            in("x0") res,     // Input: res in register x0
            options(noreturn) // Never returns, and its return type is defined as ! (never).
        )
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_info(
    out: *mut u64,
    id0: u32,
    handle: Handle,
    id1: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x29",            // Issue the SVC call with immediate value 0x29
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") out,          // Input: out in register x0
            in("x1") id0,          // Input: id0 in register x1
            in("x2") handle,       // Input: handle in register x2
            in("x3") id1,          // Input: id1 in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_flush_entire_data_cache() {
    unsafe {
        asm!(
            "svc 0x2A", // Issue the SVC call with immediate value 0x2A
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_flush_data_cache(
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x2B",           // Issue the SVC call with immediate value 0x2B
            in("x0") address,     // Input: address in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x2C",           // Issue the SVC call with immediate value 0x2C
            in("x0") address,     // Input: address in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Undoes the effects of [__nx_svc_map_physical_memory]. [3.0.0+]
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x2D",           // Issue the SVC call with immediate value 0x2D
            in("x0") address,     // Input: address in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_debug_future_thread_info(
    context: *mut LastThreadContext,
    thread_id: *mut u64,
    debug: Handle,
    ns: i64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
            "svc 0x2E",                // Issue the SVC call with immediate value 0x2E
            "ldp x6, x7, [sp], #16",   // Load x6 and x7 from the stack
            "stp x1, x2, [x6]",        // Store x1 and x2 in the memory pointed to by x6
            "stp x3, x4, [x6, #16]",   // Store x3 and x4 in the memory pointed to by x6, offset by 16 bytes
            "str x5, [x7]",            // Store x5 in the memory pointed to by x7
            in("x0") context,          // Input: context in register x0
            in("x1") thread_id,        // Input: thread_id in register x1
            in("x2") debug,            // Input: debug in register x2
            in("x3") ns,               // Input: ns in register x3
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_last_thread_info(
    context: *mut LastThreadContext,
    tls_address: *mut u64,
    flags: *mut u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x1, x2, [sp, #-16]!", // Store x1 and x2 on the stack
            "str x0, [sp, #-16]!",     // Store x0 on the stack
            "svc 0x2F",                // Issue the SVC call with immediate value 0x2F
            "ldr x7, [sp], #16",       // Load x7 from the stack
            "stp x1, x2, [x7]",        // Store x1 and x2 in the memory pointed to by x7
            "stp x3, x4, [x7, #16]",   // Store x3 and x4 in the memory pointed to by x7, offset by 16 bytes
            "ldp x1, x2, [sp], #16",   // Load x1 and x2 from the stack
            "str x5, [x1]",            // Store x5 in the memory pointed to by x1
            "str w6, [x2]",            // Store w6 in the memory pointed to by x2
            in("x0") context,          // Input: context in register x0
            in("x1") tls_address,      // Input: tls_address in register x1
            in("x2") flags,            // Input: flags in register x2
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_resource_limit_limit_value(
    value: *mut i64,
    handle: Handle,
    which: LimitableResource,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x30",            // Issue the SVC call with immediate value 0x30
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") value,        // Input: value in register x0
            in("x1") handle,       // Input: handle in register x1
            in("x2") which as u32, // Input: which in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_resource_limit_current_value(
    out: *mut i64,
    reslimit: Handle,
    which: LimitableResource,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x31",            // Issue the SVC call with immediate value 0x31
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") out,          // Input: out in register x0
            in("x1") reslimit,     // Input: reslimit in register x1
            in("x2") which as u32, // Input: which in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_thread_activity(
    thread: Handle,
    paused: ThreadActivity,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x32",             // Issue the SVC call with immediate value 0x32
            in("x0") thread,        // Input: thread in register x0
            in("x1") paused as u32, // Input: paused in register x1
            lateout("w0") result,   // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Dumps the registers of a thread paused by [__nx_svc_set_thread_activity] (register groups: all).
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_thread_context3(
    ctx: *mut ThreadContext,
    thread: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x33",           // Issue the SVC call with immediate value 0x33
            in("x0") ctx,         // Input: ctx in register x0
            in("x1") thread,      // Input: thread in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_wait_for_address(
    address: *mut c_void,
    arb_type: ArbitrationType,
    value: i64,
    timeout: i64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x34",               // Issue the SVC call with immediate value 0x34
            in("x0") address,         // Input: address in register x0
            in("x1") arb_type as u32, // Input: arb_type in register x1
            in("x2") value,           // Input: value in register x2
            in("x3") timeout,         // Input: timeout in register x3
            lateout("w0") result,     // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_signal_to_address(
    address: *mut c_void,
    signal_type: SignalType,
    value: i32,
    count: i32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x35",                  // Issue the SVC call with immediate value 0x35
            in("x0") address,            // Input: address in register x0
            in("x1") signal_type as u32, // Input: signal_type in register x1
            in("x2") value,              // Input: value in register x2
            in("x3") count,              // Input: count in register x3
            lateout("w0") result,        // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_synchronize_preemption_state() {
    unsafe {
        asm!(
            "svc 0x36", // Issue the SVC call with immediate value 0x36
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_resource_limit_peak_value(
    out: *mut i64,
    reslimit: Handle,
    which: LimitableResource,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x37",            // Issue the SVC call with immediate value 0x37
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") out,          // Input: out in register x0
            in("x1") reslimit,     // Input: reslimit in register x1
            in("x2") which as u32, // Input: which in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_io_pool(
    handle: *mut Handle,
    which: IoPoolType,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x39",            // Issue the SVC call with immediate value 0x37
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") which as u32, // Input: which in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_io_region(
    handle: *mut Handle,
    io_pool_h: Handle,
    physical_address: u64,
    size: u64,
    mapping: MemoryMapping,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!",     // Store x0 on the stack
            "svc 0x3A",                // Issue the SVC call with immediate value 0x38
            "ldr x2, [sp], #16",       // Load x2 from the stack
            "str w1, [x2]",            // Store w1 in the memory pointed to by x2
            in("x0") handle,           // Input: handle in register x0
            in("x1") io_pool_h,        // Input: io_pool_h in register x1
            in("x2") physical_address, // Input: physical_address in register x2
            in("x3") size,             // Input: size in register x3
            in("x4") mapping as u32,   // Input: mapping in register x4
            in("x5") perm,             // Input: perm in register x5
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_dump_info(dump_info_type: u32, arg0: u64) {
    unsafe {
        asm!(
            "svc 0x3C",              // Issue the SVC call with immediate value 0x3C
            in("x0") dump_info_type, // Input: dump_info_type in register x0
            in("x1") arg0,           // Input: arg0 in register x1
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_kernel_debug(
    kern_debug_type: u32,
    arg0: u64,
    arg1: u64,
    arg2: u64,
) {
    unsafe {
        asm!(
            "svc 0x3C",               // Issue the SVC call with immediate value 0x3C
            in("x0") kern_debug_type, // Input: kern_debug_type in register x0
            in("x1") arg0,            // Input: arg0 in register x1
            in("x2") arg1,            // Input: arg1 in register x2
            in("x3") arg2,            // Input: arg2 in register x3
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_change_kernel_trace_state(kern_trace_state: u32) {
    unsafe {
        asm!(
            "svc 0x3D",                  // Issue the SVC call with immediate value 0x3D
            in("x0") kern_trace_state,   // Input: kern_trace_state in register x0
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_session(
    server_handle: *mut Handle,
    client_handle: *mut Handle,
    is_light: bool,
    unk1: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
            "svc 0x40",                // Issue the SVC call with immediate value 0x40
            "ldp x3, x4, [sp], #16",   // Load x3 and x4 from the stack
            "str w1, [x3]",            // Store w1 in the memory pointed to by x3
            "str w2, [x4]",            // Store w2 in the memory pointed to by x4
            in("x0") server_handle,    // Input: server_handle in register x0
            in("x1") client_handle,    // Input: client_handle in register x1
            in("x2") is_light as u32,  // Input: is_light in register x2
            in("x3") unk1,             // Input: unk1 in register x3
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_accept_session(
    session: *mut Handle,
    port_handle: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!",   // Store x0 on the stack
            "svc 0x41",              // Issue the SVC call with immediate value 0x41
            "ldr x2, [sp], #16",     // Load x2 from the stack
            "str w1, [x2]",          // Store w1 in the memory pointed to by x2
            in("x0") session,        // Input: session in register x0
            in("x1") port_handle,    // Input: port_handle in register x1
            lateout("w0") result,    // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_reply_and_receive_light(handle: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x42",           // Issue the SVC call with immediate value 0x42
            in("x0") handle,      // Input: handle in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_reply_and_receive(
    index: *mut i32,
    handles: *const u32,
    handle_count: i32,
    reply_target: u32,
    timeout: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x43",            // Issue the SVC call with immediate value 0x43
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") index,        // Input: index in register x0
            in("x1") handles,      // Input: handles in register x1
            in("x2") handle_count, // Input: handle_count in register x2
            in("x3") reply_target, // Input: reply_target in register x3
            in("x4") timeout,      // Input: timeout in register x4
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_reply_and_receive_with_user_buffer(
    index: *mut i32,
    usr_buffer: *mut c_void,
    size: u64,
    handles: *const Handle,
    handle_count: i32,
    reply_target: Handle,
    timeout: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x44",            // Issue the SVC call with immediate value 0x44
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") index,        // Input: index in register x0
            in("x1") usr_buffer,   // Input: usr_buffer in register x1
            in("x2") size,         // Input: size in register x2
            in("x3") handles,      // Input: handles in register x3
            in("x4") handle_count, // Input: handle_count in register x4
            in("x5") reply_target, // Input: reply_target in register x5
            in("x6") timeout,      // Input: timeout in register x6
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_event(
    server_handle: *mut Handle,
    client_handle: *mut Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
            "svc 0x45",                // Issue the SVC call with immediate value 0x45
            "ldp x3, x4, [sp], #16",   // Load x3 and x4 from the stack
            "str w1, [x3]",            // Store w1 in the memory pointed to by x3
            "str w2, [x4]",            // Store w2 in the memory pointed to by x4
            in("x0") server_handle,    // Input: server_handle in register x0
            in("x1") client_handle,    // Input: client_handle in register x1
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_io_region(
    io_region_h: Handle,
    address: *mut c_void,
    size: u64,
    perm: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x46",           // Issue the SVC call with immediate value 0x46
            in("x0") io_region_h, // Input: io_region_h in register x0
            in("x1") address,     // Input: address in register x1
            in("x2") size,        // Input: size in register x2
            in("x3") perm,        // Input: perm in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Undoes the effects of [__nx_svc_map_io_region]. [13.0.0+]
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_io_region(
    io_region_h: Handle,
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x47",           // Issue the SVC call with immediate value 0x47
            in("x0") io_region_h, // Input: io_region_h in register x0
            in("x1") address,     // Input: address in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_physical_memory_unsafe(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x48",           // Issue the SVC call with immediate value 0x48
            in("x0") address,     // Input: address in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Undoes the effects of [__nx_svc_map_physical_memory_unsafe]. [5.0.0+]
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_physical_memory_unsafe(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x49",           // Issue the SVC call with immediate value 0x49
            in("x0") address,     // Input: address in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Sets the system-wide limit for unsafe memory mappable using [__nx_svc_map_physical_memory_unsafe]. [5.0.0+]
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_unsafe_limit(size: u64) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x4A",           // Issue the SVC call with immediate value 0x4A
            in("x0") size,        // Input: size in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_code_memory(
    handle: *mut Handle,
    src_addr: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x4B",            // Issue the SVC call with immediate value 0x4B
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") src_addr,     // Input: src_addr in register x1
            in("x2") size,         // Input: size in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_control_code_memory(
    code_handle: Handle,
    op: CodeMapOperation,
    dst_addr: *mut c_void,
    size: u64,
    perm: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x4C",           // Issue the SVC call with immediate value 0x4C
            in("x0") code_handle, // Input: code_handle in register x0
            in("x1") op as u32,   // Input: op in register x1
            in("x2") dst_addr,    // Input: dst_addr in register x2
            in("x3") size,        // Input: size in register x3
            in("x4") perm,        // Input: perm in register x4
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_sleep_system() {
    unsafe {
        asm!(
            "svc 0x4D", // Issue the SVC call with immediate value 0x4D
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_read_write_register(
    out_val: *mut u32,
    reg_addr: u64,
    mask: u32,
    value: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x4E",            // Issue the SVC call with immediate value 0x4E
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") out_val,      // Input: out_val in register x0
            in("x1") reg_addr,     // Input: reg_addr in register x1
            in("x2") mask,         // Input: mask in register x2
            in("x3") value,        // Input: value in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_process_activity(
    process: Handle,
    paused: ProcessActivity,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x4F",             // Issue the SVC call with immediate value 0x4F
            in("x0") process,       // Input: process in register x0
            in("x1") paused as u32, // Input: paused in register x1
            lateout("w0") result,   // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_shared_memory(
    handle: *mut Handle,
    size: usize,
    local_perm: u32, // TODO: MemoryPermission bitfield
    other_perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x50",            // Issue the SVC call with immediate value 0x4F
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") size,         // Input: size in register x1
            in("x2") local_perm,   // Input: local_perm in register x2
            in("x3") other_perm,   // Input: other_perm in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_transfer_memory(
    tmem_handle: Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x51",           // Issue the SVC call with immediate value 0x50
            in("x0") tmem_handle, // Input: tmem_handle in register x0
            in("x1") addr,        // Input: addr in register x1
            in("x2") size,        // Input: size in register x2
            in("x3") perm,        // Input: perm in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_transfer_memory(
    tmem_handle: Handle,
    addr: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x52",           // Issue the SVC call with immediate value 0x51
            in("x0") tmem_handle, // Input: tmem_handle in register x0
            in("x1") addr,        // Input: addr in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_interrupt_event(
    handle: *mut Handle,
    irq_num: u64,
    flag: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x53",            // Issue the SVC call with immediate value 0x52
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") irq_num,      // Input: irq_num in register x1
            in("x2") flag,         // Input: flag in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_query_physical_address(
    out: *mut PhysicalMemoryInfo,
    virtaddr: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x54",            // Issue the SVC call with immediate value 0x54
            "ldr x4, [sp], #16",   // Load x4 from the stack
            "stp x1, x2, [x4]",    // Store x1 and x2 in the memory pointed to by x4
            "str x3, [x4, #16]",   // Store x3 in the memory pointed to by x4, offset by 16 bytes
            in("x0") out,          // Input: out in register x0
            in("x1") virtaddr,     // Input: virtaddr in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_query_memory_mapping(
    virtaddr: *mut u64,
    out_size: *mut u64,
    physaddr: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
            "svc 0x55",                // Issue the SVC call with immediate value 0x54
            "ldp x3, x4, [sp], #16",   // Load x3 and x4 from the stack
            "str x1, [x3]",            // Store x1 in the memory pointed to by x3
            "str x2, [x4]",            // Store x2 in the memory pointed to by x4
            in("x0") virtaddr,         // Input: virtaddr in register x0
            in("x1") out_size,         // Input: out_size in register x1
            in("x2") physaddr,         // Input: physaddr in register x2
            in("x3") size,             // Input: size in register x3
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_legacy_query_io_mapping(
    virtaddr: *mut u64,
    physaddr: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x55",            // Issue the SVC call with immediate value 0x55
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") virtaddr,     // Input: virtaddr in register x0
            in("x1") physaddr,     // Input: physaddr in register x1
            in("x2") size,         // Input: size in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_device_address_space(
    handle: *mut Handle,
    dev_addr: u64,
    dev_size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x56",            // Issue the SVC call with immediate value 0x56
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") handle,       // Input: handle in register x0
            in("x1") dev_addr,     // Input: dev_addr in register x1
            in("x2") dev_size,     // Input: dev_size in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_attach_device_address_space(
    device: u64,
    handle: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x57",           // Issue the SVC call with immediate value 0x57
            in("x0") device,      // Input: device in register x0
            in("x1") handle,      // Input: handle in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_detach_device_address_space(
    device: u64,
    handle: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x58",           // Issue the SVC call with immediate value 0x58
            in("x0") device,      // Input: device in register x0
            in("x1") handle,      // Input: handle in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_device_address_space_by_force(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    option: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x59",           // Issue the SVC call with immediate value 0x59
            in("x0") handle,      // Input: handle in register x0
            in("x1") proc_handle, // Input: proc_handle in register x1
            in("x2") map_addr,    // Input: map_addr in register x2
            in("x3") dev_size,    // Input: dev_size in register x3
            in("x4") dev_addr,    // Input: dev_addr in register x4
            in("x5") option,      // Input: option in register x5
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_device_address_space_aligned(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    option: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x5A",           // Issue the SVC call with immediate value 0x5A
            in("x0") handle,      // Input: handle in register x0
            in("x1") proc_handle, // Input: proc_handle in register x1
            in("x2") map_addr,    // Input: map_addr in register x2
            in("x3") dev_size,    // Input: dev_size in register x3
            in("x4") dev_addr,    // Input: dev_addr in register x4
            in("x5") option,      // Input: option in register x5
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_device_address_space(
    out_mapped_size: *mut u64,
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    perm: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!",    // Store x0 on the stack
            "svc 0x5B",               // Issue the SVC call with immediate value 0x5B
            "ldr x2, [sp], #16",      // Load x2 from the stack
            "str w1, [x2]",           // Store w1 in the memory pointed to by x2
            in("x0") out_mapped_size, // Input: out_mapped_size in register x0
            in("x1") handle,          // Input: handle in register x1
            in("x2") proc_handle,     // Input: proc_handle in register x2
            in("x3") map_addr,        // Input: map_addr in register x3
            in("x4") dev_size,        // Input: dev_size in register x4
            in("x5") dev_addr,        // Input: dev_addr in register x5
            in("x6") perm,            // Input: perm in register x6
            lateout("w0") result,     // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_device_address_space(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    map_size: u64,
    dev_addr: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x5C",           // Issue the SVC call with immediate value 0x5C
            in("x0") handle,      // Input: handle in register x0
            in("x1") proc_handle, // Input: proc_handle in register x1
            in("x2") map_addr,    // Input: map_addr in register x2
            in("x3") map_size,    // Input: map_size in register x3
            in("x4") dev_addr,    // Input: dev_addr in register x4
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_invalidate_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x5D",           // Issue the SVC call with immediate value 0x5D
            in("x0") process,     // Input: process in register x0
            in("x1") address,     // Input: address in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_store_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x5E",           // Issue the SVC call with immediate value 0x5E
            in("x0") process,     // Input: process in register x0
            in("x1") address,     // Input: address in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_flush_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x5F",           // Issue the SVC call with immediate value 0x5F
            in("x0") process,     // Input: process in register x0
            in("x1") address,     // Input: address in register x1
            in("x2") size,        // Input: size in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_debug_active_process(
    debug: *mut Handle,
    process_id: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x60",            // Issue the SVC call with immediate value 0x60
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") debug,        // Input: debug in register x0
            in("x1") process_id,   // Input: process_id in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_break_debug_process(debug: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x61",           // Issue the SVC call with immediate value 0x61
            in("x0") debug,       // Input: debug in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_terminate_debug_process(debug: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x62",           // Issue the SVC call with immediate value 0x62
            in("x0") debug,       // Input: debug in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_debug_event(event: *mut c_void, debug: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x63",           // Issue the SVC call with immediate value 0x63
            in("x0") event,       // Input: event in register x0
            in("x1") debug,       // Input: debug in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_continue_debug_event(
    debug: Handle,
    flags: u32,
    tid_list: *mut u64,
    num_tids: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x64",           // Issue the SVC call with immediate value 0x64
            in("x0") debug,       // Input: debug in register x0
            in("x1") flags,       // Input: flags in register x1
            in("x2") tid_list,    // Input: tid_list in register x2
            in("x3") num_tids,    // Input: num_tids in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_legacy_continue_debug_event(
    debug: Handle,
    flags: u32,
    thread_id: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x64",           // Issue the SVC call with immediate value 0x64
            in("x0") debug,       // Input: debug in register x0
            in("x1") flags,       // Input: flags in register x1
            in("x2") thread_id,   // Input: thread_id in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_process_list(
    pids_count: *mut i32,
    pids_list: *mut u64,
    max_pids_count: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!",   // Store x0 on the stack
            "svc 0x65",              // Issue the SVC call with immediate value 0x65
            "ldr x2, [sp], #16",     // Load x2 from the stack
            "str w1, [x2]",          // Store w1 in the memory pointed to by x2
            in("x0") pids_count,     // Input: pids_count in register x0
            in("x1") pids_list,      // Input: pids_list in register x1
            in("x2") max_pids_count, // Input: max_pids_count in register x2
            lateout("w0") result,    // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_thread_list(
    num_out: *mut i32,
    tids_out: *mut u64,
    max_tids: u32,
    debug: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x66",            // Issue the SVC call with immediate value 0x66
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") num_out,      // Input: num_out in register x0
            in("x1") tids_out,     // Input: tids_out in register x1
            in("x2") max_tids,     // Input: max_tids in register x2
            in("x3") debug,        // Input: debug in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_debug_thread_context(
    ctx: *mut ThreadContext,
    debug: Handle,
    thread_id: u64,
    flags: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x67",           // Issue the SVC call with immediate value 0x67
            in("x0") ctx,         // Input: ctx in register x0
            in("x1") debug,       // Input: debug in register x1
            in("x2") thread_id,   // Input: thread_id in register x2
            in("x3") flags,       // Input: flags in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_debug_thread_context(
    debug: Handle,
    thread_id: u64,
    ctx: *mut ThreadContext,
    flags: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x68",           // Issue the SVC call with immediate value 0x68
            in("x0") debug,       // Input: debug in register x0
            in("x1") thread_id,   // Input: thread_id in register x1
            in("x2") ctx,         // Input: ctx in register x2
            in("x3") flags,       // Input: flags in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_query_debug_process_memory(
    meminfo_ptr: *mut MemoryInfo,
    pageinfo: *mut u32,
    debug: Handle,
    addr: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x1, [sp, #-16]!", // Store x1 on the stack
            "svc 0x69",            // Issue the SVC call with immediate value 0x69
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") meminfo_ptr,  // Input: meminfo_ptr in register x0
            in("x1") pageinfo,     // Input: pageinfo in register x1
            in("x2") debug,        // Input: debug in register x2
            in("x3") addr,         // Input: addr in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_read_debug_process_memory(
    buffer: *mut c_void,
    debug: Handle,
    addr: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x6A",           // Issue the SVC call with immediate value 0x6A
            in("x0") buffer,      // Input: buffer in register x0
            in("x1") debug,       // Input: debug in register x1
            in("x2") addr,        // Input: addr in register x2
            in("x3") size,        // Input: size in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_write_debug_process_memory(
    debug: Handle,
    buffer: *const c_void,
    addr: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x6B",           // Issue the SVC call with immediate value 0x6B
            in("x0") debug,       // Input: debug in register x0
            in("x1") buffer,      // Input: buffer in register x1
            in("x2") addr,        // Input: addr in register x2
            in("x3") size,        // Input: size in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_hardware_breakpoint(
    which: u32,
    flags: u64,
    value: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x6C",           // Issue the SVC call with immediate value 0x6C
            in("x0") which,       // Input: which in register x0
            in("x1") flags,       // Input: flags in register x1
            in("x2") value,       // Input: value in register x2
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_debug_thread_param(
    out_64: *mut u64,
    out_32: *mut u32,
    debug: Handle,
    thread_id: u64,
    param: DebugThreadParam,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
            "svc 0x6D",                // Issue the SVC call with immediate value 0x6D
            "ldp x3, x4, [sp], #16",   // Load x3 and x4 from the stack
            "str x1, [x3]",            // Store x1 in the memory pointed to by x3
            "str w2, [x4]",            // Store w2 in the memory pointed to by x4
            in("x0") out_64,           // Input: out_64 in register x0
            in("x1") out_32,           // Input: out_32 in register x1
            in("x2") debug,            // Input: debug in register x2
            in("x3") thread_id,        // Input: thread_id in register x3
            in("x4") param as u32,     // Input: param in register x4
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_system_info(
    out: *mut u64,
    id0: u64,
    handle: Handle,
    id1: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x6F",            // Issue the SVC call with immediate value 0x6F
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str x1, [x2]",        // Store x1 in the memory pointed to by x2
            in("x0") out,          // Input: out in register x0
            in("x1") id0,          // Input: id0 in register x1
            in("x2") handle,       // Input: handle in register x2
            in("x3") id1,          // Input: id1 in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_port(
    port_server: *mut Handle,
    port_client: *mut Handle,
    max_sessions: i32,
    is_light: bool,
    name: *const c_char,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "stp x0, x1, [sp, #-16]!", // Store x0 and x1 on the stack
            "svc 0x70",                // Issue the SVC call with immediate value 0x70
            "ldp x3, x4, [sp], #16",   // Load x3 and x4 from the stack
            "str w1, [x3]",            // Store w1 in the memory pointed to by x3
            "str w2, [x4]",            // Store w2 in the memory pointed to by x4
            in("x0") port_server,      // Input: port_server in register x0
            in("x1") port_client,      // Input: port_client in register x1
            in("x2") max_sessions,     // Input: max_sessions in register x2
            in("x3") is_light as u32,  // Input: is_light in register x3
            in("x4") name,             // Input: name in register x4
            lateout("w0") result,      // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_manage_named_port(
    port_server: *mut Handle,
    name: *const c_char,
    max_sessions: i32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x71",            // Issue the SVC call with immediate value 0x71
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") port_server,  // Input: port_server in register x0
            in("x1") name,         // Input: name in register x1
            in("x2") max_sessions, // Input: max_sessions in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_connect_to_port(
    session: *mut Handle,
    port: Handle,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x72",            // Issue the SVC call with immediate value 0x72
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") session,      // Input: session in register x0
            in("x1") port,         // Input: port in register x1
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_process_memory_permission(
    proc: Handle,
    addr: u64,
    size: u64,
    perm: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x73",           // Issue the SVC call with immediate value 0x73
            in("x0") proc,        // Input: proc in register x0
            in("x1") addr,        // Input: addr in register x1
            in("x2") size,        // Input: size in register x2
            in("x3") perm,        // Input: perm in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_process_memory(
    dst: *mut c_void,
    proc: Handle,
    src: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x74",           // Issue the SVC call with immediate value 0x74
            in("x0") dst,         // Input: dst in register x0
            in("x1") proc,        // Input: proc in register x1
            in("x2") src,         // Input: src in register x2
            in("x3") size,        // Input: size in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Undoes the effects of [__nx_svc_map_process_memory].
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_process_memory(
    dst: *mut c_void,
    proc: Handle,
    src: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x75",           // Issue the SVC call with immediate value 0x75
            in("x0") dst,         // Input: dst in register x0
            in("x1") proc,        // Input: proc in register x1
            in("x2") src,         // Input: src in register x2
            in("x3") size,        // Input: size in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Equivalent to [__nx_svc_query_memory], for another process.
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_query_process_memory(
    meminfo_ptr: *mut MemoryInfo,
    pageinfo: *mut u32,
    proc: Handle,
    addr: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x1, [sp, #-16]!", // Store x1 on the stack
            "svc 0x76",            // Issue the SVC call with immediate value 0x76
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") meminfo_ptr,  // Input: meminfo_ptr in register x0
            in("x1") pageinfo,     // Input: pageinfo in register x1
            in("x2") proc,         // Input: proc in register x2
            in("x3") addr,         // Input: addr in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_process_code_memory(
    proc: Handle,
    dst: u64,
    src: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x77",            // Issue the SVC call with immediate value 0x77
            in("x0") proc,         // Input: proc in register x0
            in("x1") dst,          // Input: dst in register x1
            in("x2") src,          // Input: src in register x2
            in("x3") size,         // Input: size in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Undoes the effects of [__nx_svc_map_process_code_memory].
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_process_code_memory(
    proc: Handle,
    dst: u64,
    src: u64,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x78",           // Issue the SVC call with immediate value 0x78
            in("x0") proc,        // Input: proc in register x0
            in("x1") dst,         // Input: dst in register x1
            in("x2") src,         // Input: src in register x2
            in("x3") size,        // Input: size in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_process(
    out: *mut Handle,
    proc_info: *const u8,
    caps: *const u32,
    cap_num: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x79",            // Issue the SVC call with immediate value 0x79
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") out,          // Input: out in register x0
            in("x1") proc_info,    // Input: proc_info in register x1
            in("x2") caps,         // Input: caps in register x2
            in("x3") cap_num,      // Input: cap_num in register x3
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_start_process(
    proc: Handle,
    main_prio: i32,
    default_cpu: i32,
    stack_size: u32,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x7A",           // Issue the SVC call with immediate value 0x7A
            in("x0") proc,        // Input: proc in register x0
            in("x1") main_prio,   // Input: main_prio in register x1
            in("x2") default_cpu, // Input: default_cpu in register x2
            in("x3") stack_size,  // Input: stack_size in register x3
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_terminate_process(proc: Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x7B",           // Issue the SVC call with immediate value 0x7B
            in("x0") proc,        // Input: proc in register x0
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_get_process_info(
    out: *mut i64,
    proc: Handle,
    which: ProcessInfoType,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x7C",            // Issue the SVC call with immediate value 0x7C
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") out,          // Input: out in register x0
            in("x1") proc,         // Input: proc in register x1
            in("x2") which as u32, // Input: which in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_create_resource_limit(out: *mut Handle) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "str x0, [sp, #-16]!", // Store x0 on the stack
            "svc 0x7D",            // Issue the SVC call with immediate value 0x7D
            "ldr x2, [sp], #16",   // Load x2 from the stack
            "str w1, [x2]",        // Store w1 in the memory pointed to by x2
            in("x0") out,          // Input: out in register x0
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_set_resource_limit_limit_value(
    reslimit: Handle,
    which: LimitableResource,
    value: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x7E",            // Issue the SVC call with immediate value 0x7E
            in("x0") reslimit,     // Input: reslimit in register x0
            in("x1") which as u32, // Input: which in register x1
            in("x2") value,        // Input: value in register x2
            lateout("w0") result,  // Output: Capture result from w0
            options(nostack)
        );
    }
    result
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_call_secure_monitor(regs: *mut SecmonArgs) {
    unsafe {
        asm!(
            "str x0, [sp, #-16]!",     // Store x0 on the stack
            "mov x8, x0",              // Move regs pointer to x8
            "ldp x0, x1, [x8]",        // Load first pair of args
            "ldp x2, x3, [x8, #0x10]", // Load second pair
            "ldp x4, x5, [x8, #0x20]", // Load third pair
            "ldp x6, x7, [x8, #0x30]", // Load fourth pair
            "svc 0x7F",                // Issue the SVC call
            "ldr x8, [sp], #16",       // Restore regs pointer
            "stp x0, x1, [x8]",        // Store first pair of results
            "stp x2, x3, [x8, #0x10]", // Store second pair
            "stp x4, x5, [x8, #0x20]", // Store third pair
            "stp x6, x7, [x8, #0x30]", // Store fourth pair
            in("x0") regs,             // Input: regs pointer in x0
            options(nostack)
        );
    }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_map_insecure_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x90",           // Issue the SVC call with immediate value 0x90
            in("x0") address,     // Input: address in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

/// Undoes the effects of [__nx_svc_map_insecure_physical_memory]. [15.0.0+]
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_svc_unmap_insecure_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    let result: ResultCode;
    unsafe {
        asm!(
            "svc 0x91",           // Issue the SVC call with immediate value 0x91
            in("x0") address,     // Input: address in register x0
            in("x1") size,        // Input: size in register x1
            lateout("w0") result, // Output: Capture result from w0
            options(nostack)
        );
    }
    result
}

//</editor-fold>
