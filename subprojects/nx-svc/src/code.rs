//! _Supervisor Call (SVC)_ codes.
//!
//! References:
//! - <https://switchbrew.org/wiki/SVC#system_calls>
//! - <https://developer.arm.com/documentation/ddi0602/2024-12/Base-Instructions/SVC--Supervisor-call->

/// Set the process heap to a given size. It can both extend and shrink the heap.
pub const SET_HEAP_SIZE: u16 = 0x1;

/// Set the memory permissions of a (page-aligned) range of memory.
pub const SET_MEMORY_PERMISSION: u16 = 0x2;

/// Set the memory attributes of a (page-aligned) range of memory.
pub const SET_MEMORY_ATTRIBUTE: u16 = 0x3;

/// Maps a memory range into a different range. Mainly used for adding guard pages around stack.
pub const MAP_MEMORY: u16 = 0x4;

/// Unmaps a region that was previously mapped with [MAP_MEMORY].
pub const UNMAP_MEMORY: u16 = 0x5;

/// Query information about an address. Will always fetch the lowest page-aligned mapping that contains the provided address.
pub const QUERY_MEMORY: u16 = 0x6;

/// Exits the current process.
pub const EXIT_PROCESS: u16 = 0x7;

/// Creates a thread.
pub const CREATE_THREAD: u16 = 0x8;

/// Starts a freshly created thread.
pub const START_THREAD: u16 = 0x9;

/// Exits the current thread.
pub const EXIT_THREAD: u16 = 0xA;

/// Sleeps the current thread for the specified amount of time.
pub const SLEEP_THREAD: u16 = 0xB;

/// Gets a thread's priority.
pub const GET_THREAD_PRIORITY: u16 = 0xC;

/// Sets a thread's priority.
pub const SET_THREAD_PRIORITY: u16 = 0xD;

/// Gets a thread's core mask.
pub const GET_THREAD_CORE_MASK: u16 = 0xE;

/// Sets a thread's core mask.
pub const SET_THREAD_CORE_MASK: u16 = 0xF;

/// Gets the current processor's number.
pub const GET_CURRENT_PROCESSOR_NUMBER: u16 = 0x10;

/// Sets an event's signalled status.
pub const SIGNAL_EVENT: u16 = 0x11;

/// Clears an event's signalled status.
pub const CLEAR_EVENT: u16 = 0x12;

/// Maps a block of shared memory.
pub const MAP_SHARED_MEMORY: u16 = 0x13;

/// Unmaps a block of shared memory.
pub const UNMAP_SHARED_MEMORY: u16 = 0x14;

/// Creates a block of transfer memory.
pub const CREATE_TRANSFER_MEMORY: u16 = 0x15;

/// Closes a handle, decrementing the reference count of the corresponding kernel object.
pub const CLOSE_HANDLE: u16 = 0x16;

/// Resets a signal.
pub const RESET_SIGNAL: u16 = 0x17;

/// Waits on one or more synchronization objects, optionally with a timeout.
pub const WAIT_SYNCHRONIZATION: u16 = 0x18;

/// Waits a [WAIT_SYNCHRONIZATION] operation being done on a synchronization object in another thread.
pub const CANCEL_SYNCHRONIZATION: u16 = 0x19;

/// Arbitrates a mutex lock operation in userspace.
pub const ARBITRATE_LOCK: u16 = 0x1A;

/// Arbitrates a mutex unlock operation in userspace.
pub const ARBITRATE_UNLOCK: u16 = 0x1B;

/// Performs a condition variable wait operation in userspace.
pub const WAIT_PROCESS_WIDE_KEY_ATOMIC: u16 = 0x1C;

/// Performs a condition variable wake-up operation in userspace.
pub const SIGNAL_PROCESS_WIDE_KEY: u16 = 0x1D;

/// Gets the current system tick.
pub const GET_SYSTEM_TICK: u16 = 0x1E;

/// Connects to a registered named port.
pub const CONNECT_TO_NAMED_PORT: u16 = 0x1F;

/// Sends a light IPC synchronization request to a session.
pub const SEND_SYNC_REQUEST_LIGHT: u16 = 0x20;

/// Sends an IPC synchronization request to a session.
pub const SEND_SYNC_REQUEST: u16 = 0x21;

/// Sends an IPC synchronization request to a session from a user allocated buffer.
pub const SEND_SYNC_REQUEST_WITH_USER_BUFFER: u16 = 0x22;

/// Sends an IPC synchronization request to a session from a user allocated buffer (asynchronous version).
pub const SEND_ASYNC_REQUEST_WITH_USER_BUFFER: u16 = 0x23;

/// Gets the PID associated with a process.
pub const GET_PROCESS_ID: u16 = 0x24;

/// Gets the TID associated with a process.
pub const GET_THREAD_ID: u16 = 0x25;

/// Breaks execution.
pub const BREAK: u16 = 0x26;

/// Outputs debug text, if used during debugging.
pub const OUTPUT_DEBUG_STRING: u16 = 0x27;

/// Returns from an exception.
pub const RETURN_FROM_EXCEPTION: u16 = 0x28;

/// Retrieves information about the system, or a certain kernel object.
pub const GET_INFO: u16 = 0x29;

/// Flushes the entire data cache (by set/way).
pub const FLUSH_ENTIRE_DATA_CACHE: u16 = 0x2A;

/// Flushes data cache for a virtual address range.
pub const FLUSH_DATA_CACHE: u16 = 0x2B;

/// Maps new heap memory at the desired address. [3.0.0+]
pub const MAP_PHYSICAL_MEMORY: u16 = 0x2C;

/// Undoes the effects of [MAP_PHYSICAL_MEMORY]. [3.0.0+]
pub const UNMAP_PHYSICAL_MEMORY: u16 = 0x2D;

/// Gets information about a thread that will be scheduled in the future. [5.0.0+]
pub const GET_DEBUG_FUTURE_THREAD_INFO: u16 = 0x2E;

/// Gets information about the previously-scheduled thread.
pub const GET_LAST_THREAD_INFO: u16 = 0x2F;

/// Gets the maximum value a LimitableResource can have, for a Resource Limit handle.
pub const GET_RESOURCE_LIMIT_LIMIT_VALUE: u16 = 0x30;

/// Gets the maximum value a LimitableResource can have, for a Resource Limit handle.
pub const GET_RESOURCE_LIMIT_CURRENT_VALUE: u16 = 0x31;

/// Configures the pause/unpause status of a thread.
pub const SET_THREAD_ACTIVITY: u16 = 0x32;

/// Dumps the registers of a thread paused by [SET_THREAD_ACTIVITY] (register groups: all).
pub const GET_THREAD_CONTEXT3: u16 = 0x33;

/// Arbitrates an address depending on type and value. [4.0.0+]
pub const WAIT_FOR_ADDRESS: u16 = 0x34;

/// Signals (and updates) an address depending on type and value. [4.0.0+]
pub const SIGNAL_TO_ADDRESS: u16 = 0x35;

/// Sets thread preemption state (used during abort/panic). [8.0.0+]
pub const SYNCHRONIZE_PREEMPTION_STATE: u16 = 0x36;

/// Gets the peak value a LimitableResource has had, for a Resource Limit handle. [11.0.0+]
pub const GET_RESOURCE_LIMIT_PEAK_VALUE: u16 = 0x37;

/// Creates an IO Pool. [13.0.0+]
pub const CREATE_IO_POOL: u16 = 0x39;

/// Creates an IO Region. [13.0.0+]
pub const CREATE_IO_REGION: u16 = 0x3A;

/// Causes the kernel to dump debug information. [1.0.0-3.0.2]
pub const DUMP_INFO: u16 = 0x3C;

/// Performs a debugging operation on the kernel. [4.0.0+]
pub const KERNEL_DEBUG: u16 = 0x3C;

/// Performs a debugging operation on the kernel. [4.0.0+]
pub const CHANGE_KERNEL_TRACE_STATE: u16 = 0x3D;

/// Creates an IPC session.
pub const CREATE_SESSION: u16 = 0x40;

/// Accepts an IPC session.
pub const ACCEPT_SESSION: u16 = 0x41;

/// Performs light IPC input/output.
pub const REPLY_AND_RECEIVE_LIGHT: u16 = 0x42;

/// Performs IPC input/output.
pub const REPLY_AND_RECEIVE: u16 = 0x43;

/// Performs IPC input/output from a user allocated buffer.
pub const REPLY_AND_RECEIVE_WITH_USER_BUFFER: u16 = 0x44;

/// Creates a system event.
pub const CREATE_EVENT: u16 = 0x45;

/// Maps an IO Region. [13.0.0+]
pub const MAP_IO_REGION: u16 = 0x46;

/// Undoes the effects of [MAP_IO_REGION]. [13.0.0+]
pub const UNMAP_IO_REGION: u16 = 0x47;

/// Maps unsafe memory (usable for GPU DMA) for a system module at the desired address. [5.0.0+]
pub const MAP_PHYSICAL_MEMORY_UNSAFE: u16 = 0x48;

/// Undoes the effects of [MAP_PHYSICAL_MEMORY_UNSAFE]. [5.0.0+]
pub const UNMAP_PHYSICAL_MEMORY_UNSAFE: u16 = 0x49;

/// Sets the system-wide limit for unsafe memory mappable using [MAP_PHYSICAL_MEMORY_UNSAFE]. [5.0.0+]
pub const SET_UNSAFE_LIMIT: u16 = 0x4A;

/// Creates code memory in the caller's address space [4.0.0+].
pub const CREATE_CODE_MEMORY: u16 = 0x4B;

/// Maps code memory in the caller's address space [4.0.0+].
pub const CONTROL_CODE_MEMORY: u16 = 0x4C;

/// Causes the system to enter deep sleep.
pub const SLEEP_SYSTEM: u16 = 0x4D;

/// Reads/writes a protected MMIO register.
pub const READ_WRITE_REGISTER: u16 = 0x4E;

/// Configures the pause/unpause status of a process.
pub const SET_PROCESS_ACTIVITY: u16 = 0x4F;

/// Creates a block of shared memory.
pub const CREATE_SHARED_MEMORY: u16 = 0x50;

/// Maps a block of transfer memory.
pub const MAP_TRANSFER_MEMORY: u16 = 0x51;

/// Unmaps a block of transfer memory.
pub const UNMAP_TRANSFER_MEMORY: u16 = 0x52;

/// Creates an event and binds it to a specific hardware interrupt.
pub const CREATE_INTERRUPT_EVENT: u16 = 0x53;

/// Queries information about a certain virtual address, including its physical address.
pub const QUERY_PHYSICAL_ADDRESS: u16 = 0x54;

/// Returns a virtual address mapped to a given IO range.
pub const QUERY_MEMORY_MAPPING: u16 = 0x55;

/// Returns a virtual address mapped to a given IO range.
pub const LEGACY_QUERY_IO_MAPPING: u16 = 0x55;

/// Creates a virtual address space for binding device address spaces.
pub const CREATE_DEVICE_ADDRESS_SPACE: u16 = 0x56;

/// Attaches a device address space to a device.
pub const ATTACH_DEVICE_ADDRESS_SPACE: u16 = 0x57;

/// Detaches a device address space from a device.
pub const DETACH_DEVICE_ADDRESS_SPACE: u16 = 0x58;

/// Maps an attached device address space to an userspace address.
pub const MAP_DEVICE_ADDRESS_SPACE_BY_FORCE: u16 = 0x59;

/// Maps an attached device address space to an userspace address.
pub const MAP_DEVICE_ADDRESS_SPACE_ALIGNED: u16 = 0x5A;

/// Maps an attached device address space to an userspace address. [1.0.0-12.1.0]
pub const MAP_DEVICE_ADDRESS_SPACE: u16 = 0x5B;

/// Unmaps an attached device address space from an userspace address.
pub const UNMAP_DEVICE_ADDRESS_SPACE: u16 = 0x5C;

/// Invalidates data cache for a virtual address range within a process.
pub const INVALIDATE_PROCESS_DATA_CACHE: u16 = 0x5D;

/// Stores data cache for a virtual address range within a process.
pub const STORE_PROCESS_DATA_CACHE: u16 = 0x5E;

/// Flushes data cache for a virtual address range within a process.
pub const FLUSH_PROCESS_DATA_CACHE: u16 = 0x5F;

/// Debugs an active process.
pub const DEBUG_ACTIVE_PROCESS: u16 = 0x60;

/// Breaks an active debugging session.
pub const BREAK_DEBUG_PROCESS: u16 = 0x61;

/// Terminates the process of an active debugging session.
pub const TERMINATE_DEBUG_PROCESS: u16 = 0x62;

/// Gets an incoming debug event from a debugging session.
pub const GET_DEBUG_EVENT: u16 = 0x63;

/// Continues a debugging session.
pub const CONTINUE_DEBUG_EVENT: u16 = 0x64;

/// Retrieves a list of all running processes.
pub const GET_PROCESS_LIST: u16 = 0x65;

/// Retrieves a list of all threads for a debug handle (or zero).
pub const GET_THREAD_LIST: u16 = 0x66;

/// Gets the context (dump the registers) of a thread in a debugging session.
pub const GET_DEBUG_THREAD_CONTEXT: u16 = 0x67;

/// Gets the context (dump the registers) of a thread in a debugging session.
pub const SET_DEBUG_THREAD_CONTEXT: u16 = 0x68;

/// Queries memory information from a process that is being debugged.
pub const QUERY_DEBUG_PROCESS_MEMORY: u16 = 0x69;

/// Reads memory from a process that is being debugged.
pub const READ_DEBUG_PROCESS_MEMORY: u16 = 0x6A;

/// Writes to memory in a process that is being debugged.
pub const WRITE_DEBUG_PROCESS_MEMORY: u16 = 0x6B;

/// Sets one of the hardware breakpoints.
pub const SET_HARDWARE_BREAKPOINT: u16 = 0x6C;

/// Gets parameters from a thread in a debugging session.
pub const GET_DEBUG_THREAD_PARAM: u16 = 0x6D;

/// Retrieves privileged information about the system, or a certain kernel object.
pub const GET_SYSTEM_INFO: u16 = 0x6F;

/// Creates a port.
pub const CREATE_PORT: u16 = 0x70;

/// Manages a named port.
pub const MANAGE_NAMED_PORT: u16 = 0x71;

/// Connects to a port.
pub const CONNECT_TO_PORT: u16 = 0x72;

/// Sets the memory permissions for the specified memory with the supplied process handle.
pub const SET_PROCESS_MEMORY_PERMISSION: u16 = 0x73;

/// Maps the src address from the supplied process handle into the current process.
pub const MAP_PROCESS_MEMORY: u16 = 0x74;

/// Undoes the effects of [MAP_PROCESS_MEMORY].
pub const UNMAP_PROCESS_MEMORY: u16 = 0x75;

/// Equivalent to [QUERY_MEMORY], for another process.
pub const QUERY_PROCESS_MEMORY: u16 = 0x76;

/// Maps normal heap in a certain process as executable code (used when loading NROs).
pub const MAP_PROCESS_CODE_MEMORY: u16 = 0x77;

/// Undoes the effects of [MAP_PROCESS_CODE_MEMORY].
pub const UNMAP_PROCESS_CODE_MEMORY: u16 = 0x78;

/// Creates a new process.
pub const CREATE_PROCESS: u16 = 0x79;

/// Starts executing a freshly created process.
pub const START_PROCESS: u16 = 0x7A;

/// Terminates a running process.
pub const TERMINATE_PROCESS: u16 = 0x7B;

/// Gets a `ProcessInfoType` for a process.
pub const GET_PROCESS_INFO: u16 = 0x7C;

/// Creates a new Resource Limit handle.
pub const CREATE_RESOURCE_LIMIT: u16 = 0x7D;

/// Sets the value for a `LimitableResource` for a Resource Limit handle.
pub const SET_RESOURCE_LIMIT_LIMIT_VALUE: u16 = 0x7E;

/// Calls a secure monitor function (TrustZone, EL3).
pub const CALL_SECURE_MONITOR: u16 = 0x7F;

/// Maps new insecure memory at the desired address. [15.0.0+]
pub const MAP_INSECURE_PHYSICAL_MEMORY: u16 = 0x90;

/// Undoes the effects of [MAP_INSECURE_PHYSICAL_MEMORY]. [15.0.0+]
pub const UNMAP_INSECURE_PHYSICAL_MEMORY: u16 = 0x91;
