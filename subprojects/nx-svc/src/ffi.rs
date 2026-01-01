//! Raw _Supervisor Call (SVC)_ API.

use core::ffi::{c_char, c_int, c_void};

use crate::{
    raw::{
        self, ArbitrationType, BreakReason, CodeMapOperation, DebugThreadParam, Handle, IoPoolType,
        LastThreadContext, LimitableResource, MemoryInfo, MemoryMapping, PhysicalMemoryInfo,
        ProcessActivity, ProcessInfoType, SecmonArgs, SignalType, ThreadActivity, ThreadContext,
    },
    result::ResultCode,
};

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
unsafe extern "C" fn __nx_svc__svc_set_heap_size(
    out_addr: *mut *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::set_heap_size(out_addr, size) }
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
unsafe extern "C" fn __nx_svc__svc_set_memory_permission(
    addr: *mut c_void,
    size: usize,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    unsafe { raw::set_memory_permission(addr, size, perm) }
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
unsafe extern "C" fn __nx_svc__svc_set_memory_attribute(
    addr: *mut c_void,
    size: usize,
    mask: u32,
    attr: u32,
) -> ResultCode {
    unsafe { raw::set_memory_attribute(addr, size, mask, attr) }
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
unsafe extern "C" fn __nx_svc__svc_map_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::map_memory(dst_addr, src_addr, size) }
}

/// Unmaps a region that was previously mapped with [`__nx_svc__svc_map_memory`].
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
unsafe extern "C" fn __nx_svc__svc_unmap_memory(
    dst_addr: *mut c_void,
    src_addr: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::unmap_memory(dst_addr, src_addr, size) }
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
unsafe extern "C" fn __nx_svc__svc_query_memory(
    meminfo: *mut MemoryInfo,
    pageinfo: *mut u32,
    addr: usize,
) -> ResultCode {
    unsafe { raw::query_memory(meminfo, pageinfo, addr) }
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
unsafe extern "C" fn __nx_svc__svc_exit_process() -> ! {
    unsafe { raw::exit_process() }
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
unsafe extern "C" fn __nx_svc__svc_create_thread(
    handle: *mut Handle,
    entry: *mut c_void,
    arg: *mut c_void,
    stack_top: *mut c_void,
    prio: c_int,
    cpuid: c_int,
) -> ResultCode {
    unsafe { raw::create_thread(handle, entry, arg, stack_top, prio, cpuid) }
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
unsafe extern "C" fn __nx_svc__svc_start_thread(handle: Handle) -> ResultCode {
    unsafe { raw::start_thread(handle) }
}

/// Exits the current thread.
///
/// `void NX_NORETURN svcExitThread(void);`
///
/// Syscall code: [EXIT_THREAD](crate::code::EXIT_THREAD) (`0xA`).
///
/// Ref: <https://switchbrew.org/wiki/SVC#ExitThread>
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_svc__svc_exit_thread() -> ! {
    unsafe { raw::exit_thread() }
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
unsafe extern "C" fn __nx_svc__svc_sleep_thread(nano: i64) {
    unsafe { raw::sleep_thread(nano) }
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
unsafe extern "C" fn __nx_svc__svc_get_thread_priority(
    priority: *mut i32,
    handle: Handle,
) -> ResultCode {
    unsafe { raw::get_thread_priority(priority, handle) }
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
unsafe extern "C" fn __nx_svc__svc_set_thread_priority(
    handle: Handle,
    priority: u32,
) -> ResultCode {
    unsafe { raw::set_thread_priority(handle, priority) }
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
unsafe extern "C" fn __nx_svc__svc_get_thread_core_mask(
    core_id: *mut i32,
    affinity_mask: *mut u64,
    handle: Handle,
) -> ResultCode {
    unsafe { raw::get_thread_core_mask(core_id, affinity_mask, handle) }
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
unsafe extern "C" fn __nx_svc__svc_set_thread_core_mask(
    handle: Handle,
    core_id: i32,
    affinity_mask: u32,
) -> ResultCode {
    unsafe { raw::set_thread_core_mask(handle, core_id, affinity_mask) }
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
unsafe extern "C" fn __nx_svc__svc_get_current_processor_number() -> ResultCode {
    unsafe { raw::get_current_processor_number() }
}

//</editor-fold>

//<editor-fold desc="Synchronization">

/// Puts the given event in the signaled state.
///
/// Will wake up any thread currently waiting on this event. Can potentially trigger a re-schedule.
///
/// Any calls to [__nx_svc__svc_wait_synchronization] on this handle will return immediately, until the
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
unsafe extern "C" fn __nx_svc__svc_signal_event(handle: Handle) -> ResultCode {
    unsafe { raw::signal_event(handle) }
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
unsafe extern "C" fn __nx_svc__svc_clear_event(handle: Handle) -> ResultCode {
    unsafe { raw::clear_event(handle) }
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
unsafe extern "C" fn __nx_svc__svc_map_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    unsafe { raw::map_shared_memory(handle, addr, size, perm) }
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
unsafe extern "C" fn __nx_svc__svc_unmap_shared_memory(
    handle: Handle,
    addr: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::unmap_shared_memory(handle, addr, size) }
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
unsafe extern "C" fn __nx_svc__svc_create_transfer_memory(
    handle: *mut Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    unsafe { raw::create_transfer_memory(handle, addr, size, perm) }
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
unsafe extern "C" fn __nx_svc__svc_close_handle(handle: Handle) -> ResultCode {
    unsafe { raw::close_handle(handle) }
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
unsafe extern "C" fn __nx_svc__svc_reset_signal(handle: Handle) -> ResultCode {
    unsafe { raw::reset_signal(handle) }
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
unsafe extern "C" fn __nx_svc__svc_wait_synchronization(
    index: *mut i32,
    handles: *const u32,
    handle_count: i32,
    timeout: u64,
) -> ResultCode {
    unsafe { raw::wait_synchronization(index, handles, handle_count, timeout) }
}

/// Waits a [__nx_svc__svc_wait_synchronization] operation being done on a synchronization object in
/// another thread.
///
/// If the referenced thread is currently in a synchronization call ([__nx_svc__svc_wait_synchronization],
/// [__nx_svc__svc_reply_and_receive] or [__nx_svc__svc_reply_and_receive_light]), that call will be
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
unsafe extern "C" fn __nx_svc__svc_cancel_synchronization(handle: Handle) -> ResultCode {
    unsafe { raw::cancel_synchronization(handle) }
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
unsafe extern "C" fn __nx_svc__svc_arbitrate_lock(
    owner_thread_handle: Handle,
    mutex: *mut u32,
    curr_thread_handle: Handle,
) -> ResultCode {
    unsafe { raw::arbitrate_lock(owner_thread_handle, mutex, curr_thread_handle) }
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
unsafe extern "C" fn __nx_svc__svc_arbitrate_unlock(mutex: *mut u32) -> ResultCode {
    unsafe { raw::arbitrate_unlock(mutex) }
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
unsafe extern "C" fn __nx_svc__svc_wait_process_wide_key_atomic(
    address: *mut u32,
    cv_key: *mut u32,
    tag: u32,
    timeout_ns: u64,
) -> ResultCode {
    unsafe { raw::wait_process_wide_key_atomic(address, cv_key, tag, timeout_ns) }
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
unsafe extern "C" fn __nx_svc__svc_signal_process_wide_key(cv_key: *mut u32, count: i32) {
    unsafe { raw::signal_process_wide_key(cv_key, count) }
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
unsafe extern "C" fn __nx_svc__svc_get_system_tick() -> u64 {
    unsafe { raw::get_system_tick() }
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
unsafe extern "C" fn __nx_svc__svc_connect_to_named_port(
    session: *mut Handle,
    name: *const c_char,
) -> ResultCode {
    unsafe { raw::connect_to_named_port(session, name) }
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
unsafe extern "C" fn __nx_svc__svc_send_sync_request_light(session: Handle) -> ResultCode {
    unsafe { raw::send_sync_request_light(session) }
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
unsafe extern "C" fn __nx_svc__svc_send_sync_request(session: Handle) -> ResultCode {
    unsafe { raw::send_sync_request(session) }
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
unsafe extern "C" fn __nx_svc__svc_send_sync_request_with_user_buffer(
    usr_buffer: *mut c_void,
    size: u64,
    session: Handle,
) -> ResultCode {
    unsafe { raw::send_sync_request_with_user_buffer(usr_buffer, size, session) }
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
unsafe extern "C" fn __nx_svc__svc_send_async_request_with_user_buffer(
    handle: *mut Handle,
    usr_buffer: *mut c_void,
    size: u64,
    session: Handle,
) -> ResultCode {
    unsafe { raw::send_async_request_with_user_buffer(handle, usr_buffer, size, session) }
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
unsafe extern "C" fn __nx_svc__svc_get_process_id(
    process_id: *mut u64,
    handle: Handle,
) -> ResultCode {
    unsafe { raw::get_process_id(process_id, handle) }
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
unsafe extern "C" fn __nx_svc__svc_get_thread_id(
    thread_id: *mut u64,
    handle: Handle,
) -> ResultCode {
    unsafe { raw::get_thread_id(thread_id, handle) }
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
unsafe extern "C" fn __nx_svc__svc_break(
    reason: BreakReason,
    address: usize,
    size: usize,
) -> ResultCode {
    unsafe { raw::r#break(reason, address, size) }
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
unsafe extern "C" fn __nx_svc__svc_output_debug_string(
    dbg_str: *const c_char,
    size: u64,
) -> ResultCode {
    unsafe { raw::output_debug_string(dbg_str, size) }
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
unsafe extern "C" fn __nx_svc__svc_return_from_exception(res: ResultCode) -> ! {
    unsafe { raw::return_from_exception(res) }
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
unsafe extern "C" fn __nx_svc__svc_get_info(
    out: *mut u64,
    id0: u32,
    handle: Handle,
    id1: u64,
) -> ResultCode {
    unsafe { raw::get_info(out, id0, handle, id1) }
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
unsafe extern "C" fn __nx_svc__svc_flush_entire_data_cache() {
    unsafe { raw::flush_entire_data_cache() }
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
unsafe extern "C" fn __nx_svc__svc_flush_data_cache(
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::flush_data_cache(address, size) }
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
unsafe extern "C" fn __nx_svc__svc_map_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::map_physical_memory(address, size) }
}

/// Undoes the effects of [__nx_svc__svc_map_physical_memory]. [3.0.0+]
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
unsafe extern "C" fn __nx_svc__svc_unmap_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::unmap_physical_memory(address, size) }
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
unsafe extern "C" fn __nx_svc__svc_get_debug_future_thread_info(
    context: *mut LastThreadContext,
    thread_id: *mut u64,
    debug: Handle,
    ns: i64,
) -> ResultCode {
    unsafe { raw::get_debug_future_thread_info(context, thread_id, debug, ns) }
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
unsafe extern "C" fn __nx_svc__svc_get_last_thread_info(
    context: *mut LastThreadContext,
    tls_address: *mut u64,
    flags: *mut u32,
) -> ResultCode {
    unsafe { raw::get_last_thread_info(context, tls_address, flags) }
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
unsafe extern "C" fn __nx_svc__svc_get_resource_limit_limit_value(
    value: *mut i64,
    handle: Handle,
    which: LimitableResource,
) -> ResultCode {
    unsafe { raw::get_resource_limit_limit_value(value, handle, which) }
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
unsafe extern "C" fn __nx_svc__svc_get_resource_limit_current_value(
    out: *mut i64,
    reslimit: Handle,
    which: LimitableResource,
) -> ResultCode {
    unsafe { raw::get_resource_limit_current_value(out, reslimit, which) }
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
unsafe extern "C" fn __nx_svc__svc_set_thread_activity(
    thread: Handle,
    paused: ThreadActivity,
) -> ResultCode {
    unsafe { raw::set_thread_activity(thread, paused) }
}

/// Dumps the registers of a thread paused by [__nx_svc__svc_set_thread_activity] (register groups: all).
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
unsafe extern "C" fn __nx_svc__svc_get_thread_context3(
    ctx: *mut raw::ThreadContext,
    thread: Handle,
) -> ResultCode {
    unsafe { raw::get_thread_context3(ctx, thread) }
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
unsafe extern "C" fn __nx_svc__svc_wait_for_address(
    address: *mut c_void,
    arb_type: ArbitrationType,
    value: i64,
    timeout: i64,
) -> ResultCode {
    unsafe { raw::wait_for_address(address, arb_type, value, timeout) }
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
unsafe extern "C" fn __nx_svc__svc_signal_to_address(
    address: *mut c_void,
    signal_type: SignalType,
    value: i32,
    count: i32,
) -> ResultCode {
    unsafe { raw::signal_to_address(address, signal_type, value, count) }
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
unsafe extern "C" fn __nx_svc__svc_synchronize_preemption_state() {
    unsafe { raw::synchronize_preemption_state() }
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
unsafe extern "C" fn __nx_svc__svc_get_resource_limit_peak_value(
    out: *mut i64,
    reslimit: Handle,
    which: LimitableResource,
) -> ResultCode {
    unsafe { raw::get_resource_limit_peak_value(out, reslimit, which) }
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
unsafe extern "C" fn __nx_svc__svc_create_io_pool(
    handle: *mut Handle,
    which: IoPoolType,
) -> ResultCode {
    unsafe { raw::create_io_pool(handle, which) }
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
unsafe extern "C" fn __nx_svc__svc_create_io_region(
    handle: *mut Handle,
    io_pool_h: Handle,
    physical_address: u64,
    size: u64,
    mapping: MemoryMapping,
    perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    unsafe { raw::create_io_region(handle, io_pool_h, physical_address, size, mapping, perm) }
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
unsafe extern "C" fn __nx_svc__svc_dump_info(dump_info_type: u32, arg0: u64) {
    unsafe { raw::dump_info(dump_info_type, arg0) }
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
unsafe extern "C" fn __nx_svc__svc_kernel_debug(
    kern_debug_type: u32,
    arg0: u64,
    arg1: u64,
    arg2: u64,
) {
    unsafe { raw::kernel_debug(kern_debug_type, arg0, arg1, arg2) }
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
unsafe extern "C" fn __nx_svc__svc_change_kernel_trace_state(kern_trace_state: u32) {
    unsafe { raw::change_kernel_trace_state(kern_trace_state) }
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
unsafe extern "C" fn __nx_svc__svc_create_session(
    server_handle: *mut Handle,
    client_handle: *mut Handle,
    is_light: bool,
    unk1: u64,
) -> ResultCode {
    unsafe { raw::create_session(server_handle, client_handle, is_light, unk1) }
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
unsafe extern "C" fn __nx_svc__svc_accept_session(
    session: *mut Handle,
    port_handle: Handle,
) -> ResultCode {
    unsafe { raw::accept_session(session, port_handle) }
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
unsafe extern "C" fn __nx_svc__svc_reply_and_receive_light(handle: Handle) -> ResultCode {
    unsafe { raw::reply_and_receive_light(handle) }
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
unsafe extern "C" fn __nx_svc__svc_reply_and_receive(
    index: *mut i32,
    handles: *const u32,
    handle_count: i32,
    reply_target: u32,
    timeout: u64,
) -> ResultCode {
    unsafe { raw::reply_and_receive(index, handles, handle_count, reply_target, timeout) }
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
unsafe extern "C" fn __nx_svc__svc_reply_and_receive_with_user_buffer(
    index: *mut i32,
    usr_buffer: *mut c_void,
    size: u64,
    handles: *const Handle,
    handle_count: i32,
    reply_target: Handle,
    timeout: u64,
) -> ResultCode {
    unsafe {
        raw::reply_and_receive_with_user_buffer(
            index,
            usr_buffer,
            size,
            handles,
            handle_count,
            reply_target,
            timeout,
        )
    }
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
unsafe extern "C" fn __nx_svc__svc_create_event(
    server_handle: *mut Handle,
    client_handle: *mut Handle,
) -> ResultCode {
    unsafe { raw::create_event(server_handle, client_handle) }
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
unsafe extern "C" fn __nx_svc__svc_map_io_region(
    io_region_h: Handle,
    address: *mut c_void,
    size: u64,
    perm: u32,
) -> ResultCode {
    unsafe { raw::map_io_region(io_region_h, address, size, perm) }
}

/// Undoes the effects of [__nx_svc__svc_map_io_region]. [13.0.0+]
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
unsafe extern "C" fn __nx_svc__svc_unmap_io_region(
    io_region_h: Handle,
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::unmap_io_region(io_region_h, address, size) }
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
unsafe extern "C" fn __nx_svc__svc_map_physical_memory_unsafe(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::map_physical_memory(address, size) }
}

/// Undoes the effects of [__nx_svc__svc_map_physical_memory_unsafe]. [5.0.0+]
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
unsafe extern "C" fn __nx_svc__svc_unmap_physical_memory_unsafe(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::unmap_physical_memory(address, size) }
}

/// Sets the system-wide limit for unsafe memory mappable using [__nx_svc__svc_map_physical_memory_unsafe]. [5.0.0+]
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
unsafe extern "C" fn __nx_svc__svc_set_unsafe_limit(size: u64) -> ResultCode {
    unsafe { raw::set_unsafe_limit(size) }
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
unsafe extern "C" fn __nx_svc__svc_create_code_memory(
    handle: *mut Handle,
    src_addr: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::create_code_memory(handle, src_addr, size) }
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
unsafe extern "C" fn __nx_svc__svc_control_code_memory(
    code_handle: Handle,
    op: CodeMapOperation,
    dst_addr: *mut c_void,
    size: u64,
    perm: u64,
) -> ResultCode {
    unsafe { raw::control_code_memory(code_handle, op, dst_addr, size, perm) }
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
unsafe extern "C" fn __nx_svc__svc_sleep_system() {
    unsafe { raw::sleep_system() }
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
unsafe extern "C" fn __nx_svc__svc_read_write_register(
    out_val: *mut u32,
    reg_addr: u64,
    mask: u32,
    value: u32,
) -> ResultCode {
    unsafe { raw::read_write_register(out_val, reg_addr, mask, value) }
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
unsafe extern "C" fn __nx_svc__svc_set_process_activity(
    process: Handle,
    paused: ProcessActivity,
) -> ResultCode {
    unsafe { raw::set_process_activity(process, paused) }
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
unsafe extern "C" fn __nx_svc__svc_create_shared_memory(
    handle: *mut Handle,
    size: usize,
    local_perm: u32, // TODO: MemoryPermission bitfield
    other_perm: u32, // TODO: MemoryPermission bitfield
) -> ResultCode {
    unsafe { raw::create_shared_memory(handle, size, local_perm, other_perm) }
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
unsafe extern "C" fn __nx_svc__svc_map_transfer_memory(
    tmem_handle: Handle,
    addr: *mut c_void,
    size: usize,
    perm: u32,
) -> ResultCode {
    unsafe { raw::map_transfer_memory(tmem_handle, addr, size, perm) }
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
unsafe extern "C" fn __nx_svc__svc_unmap_transfer_memory(
    tmem_handle: Handle,
    addr: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::unmap_transfer_memory(tmem_handle, addr, size) }
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
unsafe extern "C" fn __nx_svc__svc_create_interrupt_event(
    handle: *mut Handle,
    irq_num: u64,
    flag: u32,
) -> ResultCode {
    unsafe { raw::create_interrupt_event(handle, irq_num, flag) }
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
unsafe extern "C" fn __nx_svc__svc_query_physical_address(
    out: *mut PhysicalMemoryInfo,
    virtaddr: u64,
) -> ResultCode {
    unsafe { raw::query_physical_address(out, virtaddr) }
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
unsafe extern "C" fn __nx_svc__svc_query_memory_mapping(
    virtaddr: *mut u64,
    out_size: *mut u64,
    physaddr: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::query_memory_mapping(virtaddr, out_size, physaddr, size) }
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
unsafe extern "C" fn __nx_svc__svc_legacy_query_io_mapping(
    virtaddr: *mut u64,
    physaddr: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::legacy_query_io_mapping(virtaddr, physaddr, size) }
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
unsafe extern "C" fn __nx_svc__svc_create_device_address_space(
    handle: *mut Handle,
    dev_addr: u64,
    dev_size: u64,
) -> ResultCode {
    unsafe { raw::create_device_address_space(handle, dev_addr, dev_size) }
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
unsafe extern "C" fn __nx_svc__svc_attach_device_address_space(
    device: u64,
    handle: Handle,
) -> ResultCode {
    unsafe { raw::attach_device_address_space(device, handle) }
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
unsafe extern "C" fn __nx_svc__svc_detach_device_address_space(
    device: u64,
    handle: Handle,
) -> ResultCode {
    unsafe { raw::detach_device_address_space(device, handle) }
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
unsafe extern "C" fn __nx_svc__svc_map_device_address_space_by_force(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    option: u32,
) -> ResultCode {
    unsafe {
        raw::map_device_address_space_by_force(
            handle,
            proc_handle,
            map_addr,
            dev_size,
            dev_addr,
            option,
        )
    }
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
unsafe extern "C" fn __nx_svc__svc_map_device_address_space_aligned(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    option: u32,
) -> ResultCode {
    unsafe {
        raw::map_device_address_space_aligned(
            handle,
            proc_handle,
            map_addr,
            dev_size,
            dev_addr,
            option,
        )
    }
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
unsafe extern "C" fn __nx_svc__svc_map_device_address_space(
    out_mapped_size: *mut u64,
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    dev_size: u64,
    dev_addr: u64,
    perm: u32,
) -> ResultCode {
    unsafe {
        raw::map_device_address_space(
            out_mapped_size,
            handle,
            proc_handle,
            map_addr,
            dev_size,
            dev_addr,
            perm,
        )
    }
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
unsafe extern "C" fn __nx_svc__svc_unmap_device_address_space(
    handle: Handle,
    proc_handle: Handle,
    map_addr: u64,
    map_size: u64,
    dev_addr: u64,
) -> ResultCode {
    unsafe { raw::unmap_device_address_space(handle, proc_handle, map_addr, map_size, dev_addr) }
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
unsafe extern "C" fn __nx_svc__svc_invalidate_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::invalidate_process_data_cache(process, address, size) }
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
unsafe extern "C" fn __nx_svc__svc_store_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::store_process_data_cache(process, address, size) }
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
unsafe extern "C" fn __nx_svc__svc_flush_process_data_cache(
    process: Handle,
    address: *mut c_void,
    size: usize,
) -> ResultCode {
    unsafe { raw::flush_process_data_cache(process, address, size) }
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
unsafe extern "C" fn __nx_svc__svc_debug_active_process(
    debug: *mut Handle,
    process_id: u64,
) -> ResultCode {
    unsafe { raw::debug_active_process(debug, process_id) }
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
unsafe extern "C" fn __nx_svc__svc_break_debug_process(debug: Handle) -> ResultCode {
    unsafe { raw::break_debug_process(debug) }
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
unsafe extern "C" fn __nx_svc__svc_terminate_debug_process(debug: Handle) -> ResultCode {
    unsafe { raw::terminate_debug_process(debug) }
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
unsafe extern "C" fn __nx_svc__svc_get_debug_event(
    event: *mut c_void,
    debug: Handle,
) -> ResultCode {
    unsafe { raw::get_debug_event(event, debug) }
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
unsafe extern "C" fn __nx_svc__svc_continue_debug_event(
    debug: Handle,
    flags: u32,
    tid_list: *mut u64,
    num_tids: u32,
) -> ResultCode {
    unsafe { raw::continue_debug_event(debug, flags, tid_list, num_tids) }
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
unsafe extern "C" fn __nx_svc__svc_legacy_continue_debug_event(
    debug: Handle,
    flags: u32,
    thread_id: u64,
) -> ResultCode {
    unsafe { raw::legacy_continue_debug_event(debug, flags, thread_id) }
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
unsafe extern "C" fn __nx_svc__svc_get_process_list(
    pids_count: *mut i32,
    pids_list: *mut u64,
    max_pids_count: u32,
) -> ResultCode {
    unsafe { raw::get_process_list(pids_count, pids_list, max_pids_count) }
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
unsafe extern "C" fn __nx_svc__svc_get_thread_list(
    num_out: *mut i32,
    tids_out: *mut u64,
    max_tids: u32,
    debug: Handle,
) -> ResultCode {
    unsafe { raw::get_thread_list(num_out, tids_out, max_tids, debug) }
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
unsafe extern "C" fn __nx_svc__svc_get_debug_thread_context(
    ctx: *mut ThreadContext,
    debug: Handle,
    thread_id: u64,
    flags: u32,
) -> ResultCode {
    unsafe { raw::get_debug_thread_context(ctx, debug, thread_id, flags) }
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
unsafe extern "C" fn __nx_svc__svc_set_debug_thread_context(
    debug: Handle,
    thread_id: u64,
    ctx: *mut ThreadContext,
    flags: u32,
) -> ResultCode {
    unsafe { raw::set_debug_thread_context(debug, thread_id, ctx, flags) }
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
unsafe extern "C" fn __nx_svc__svc_query_debug_process_memory(
    meminfo_ptr: *mut MemoryInfo,
    pageinfo: *mut u32,
    debug: Handle,
    addr: u64,
) -> ResultCode {
    unsafe { raw::query_debug_process_memory(meminfo_ptr, pageinfo, debug, addr) }
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
unsafe extern "C" fn __nx_svc__svc_read_debug_process_memory(
    buffer: *mut c_void,
    debug: Handle,
    addr: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::read_debug_process_memory(buffer, debug, addr, size) }
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
unsafe extern "C" fn __nx_svc__svc_write_debug_process_memory(
    debug: Handle,
    buffer: *const c_void,
    addr: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::write_debug_process_memory(debug, buffer, addr, size) }
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
unsafe extern "C" fn __nx_svc__svc_set_hardware_breakpoint(
    which: u32,
    flags: u64,
    value: u64,
) -> ResultCode {
    unsafe { raw::set_hardware_breakpoint(which, flags, value) }
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
unsafe extern "C" fn __nx_svc__svc_get_debug_thread_param(
    out_64: *mut u64,
    out_32: *mut u32,
    debug: Handle,
    thread_id: u64,
    param: DebugThreadParam,
) -> ResultCode {
    unsafe { raw::get_debug_thread_param(out_64, out_32, debug, thread_id, param) }
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
unsafe extern "C" fn __nx_svc__svc_get_system_info(
    out: *mut u64,
    id0: u64,
    handle: Handle,
    id1: u64,
) -> ResultCode {
    unsafe { raw::get_system_info(out, id0, handle, id1) }
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
unsafe extern "C" fn __nx_svc__svc_create_port(
    port_server: *mut Handle,
    port_client: *mut Handle,
    max_sessions: i32,
    is_light: bool,
    name: *const c_char,
) -> ResultCode {
    unsafe { raw::create_port(port_server, port_client, max_sessions, is_light, name) }
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
unsafe extern "C" fn __nx_svc__svc_manage_named_port(
    port_server: *mut Handle,
    name: *const c_char,
    max_sessions: i32,
) -> ResultCode {
    unsafe { raw::manage_named_port(port_server, name, max_sessions) }
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
unsafe extern "C" fn __nx_svc__svc_connect_to_port(
    session: *mut Handle,
    port: Handle,
) -> ResultCode {
    unsafe { raw::connect_to_port(session, port) }
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
unsafe extern "C" fn __nx_svc__svc_set_process_memory_permission(
    proc: Handle,
    addr: u64,
    size: u64,
    perm: u32,
) -> ResultCode {
    unsafe { raw::set_process_memory_permission(proc, addr, size, perm) }
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
unsafe extern "C" fn __nx_svc__svc_map_process_memory(
    dst: *mut c_void,
    proc: Handle,
    src: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::map_process_memory(dst, proc, src, size) }
}

/// Undoes the effects of [__nx_svc__svc_map_process_memory].
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
unsafe extern "C" fn __nx_svc__svc_unmap_process_memory(
    dst: *mut c_void,
    proc: Handle,
    src: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::unmap_process_memory(dst, proc, src, size) }
}

/// Equivalent to [__nx_svc__svc_query_memory], for another process.
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
unsafe extern "C" fn __nx_svc__svc_query_process_memory(
    meminfo_ptr: *mut MemoryInfo,
    pageinfo: *mut u32,
    proc: Handle,
    addr: u64,
) -> ResultCode {
    unsafe { raw::query_process_memory(meminfo_ptr, pageinfo, proc, addr) }
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
unsafe extern "C" fn __nx_svc__svc_map_process_code_memory(
    proc: Handle,
    dst: u64,
    src: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::map_process_code_memory(proc, dst, src, size) }
}

/// Undoes the effects of [__nx_svc__svc_map_process_code_memory].
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
unsafe extern "C" fn __nx_svc__svc_unmap_process_code_memory(
    proc: Handle,
    dst: u64,
    src: u64,
    size: u64,
) -> ResultCode {
    unsafe { raw::unmap_process_code_memory(proc, dst, src, size) }
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
unsafe extern "C" fn __nx_svc__svc_create_process(
    out: *mut Handle,
    proc_info: *const u8,
    caps: *const u32,
    cap_num: u64,
) -> ResultCode {
    unsafe { raw::create_process(out, proc_info, caps, cap_num) }
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
unsafe extern "C" fn __nx_svc__svc_start_process(
    proc: Handle,
    main_prio: i32,
    default_cpu: i32,
    stack_size: u32,
) -> ResultCode {
    unsafe { raw::start_process(proc, main_prio, default_cpu, stack_size) }
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
unsafe extern "C" fn __nx_svc__svc_terminate_process(proc: Handle) -> ResultCode {
    unsafe { raw::terminate_process(proc) }
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
unsafe extern "C" fn __nx_svc__svc_get_process_info(
    out: *mut i64,
    proc: Handle,
    which: ProcessInfoType,
) -> ResultCode {
    unsafe { raw::get_process_info(out, proc, which) }
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
unsafe extern "C" fn __nx_svc__svc_create_resource_limit(out: *mut Handle) -> ResultCode {
    unsafe { raw::create_resource_limit(out) }
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
unsafe extern "C" fn __nx_svc__svc_set_resource_limit_limit_value(
    reslimit: Handle,
    which: LimitableResource,
    value: u64,
) -> ResultCode {
    unsafe { raw::set_resource_limit_limit_value(reslimit, which, value) }
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
unsafe extern "C" fn __nx_svc__svc_call_secure_monitor(regs: *mut SecmonArgs) {
    unsafe { raw::call_secure_monitor(regs) }
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
unsafe extern "C" fn __nx_svc__svc_map_insecure_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::map_insecure_physical_memory(address, size) }
}

/// Undoes the effects of [__nx_svc__svc_map_insecure_physical_memory]. [15.0.0+]
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
unsafe extern "C" fn __nx_svc__svc_unmap_insecure_physical_memory(
    address: *mut c_void,
    size: u64,
) -> ResultCode {
    unsafe { raw::unmap_insecure_physical_memory(address, size) }
}

//</editor-fold>
