//! Shared memory layout and access for Time service (6.0.0+).

use core::{
    ptr,
    sync::atomic::{AtomicU32, Ordering, compiler_fence},
};

use crate::types::{TimeStandardSteadyClockTimePointType, TimeSystemClockContext};

/// Offsets in shared memory for time data structures.
pub mod offsets {
    /// Offset for standard steady clock time point (0x00).
    pub const STEADY_CLOCK: usize = 0x00;

    /// Offset for user/local system clock context (0x38).
    pub const USER_SYSTEM_CLOCK: usize = 0x38;

    /// Offset for network system clock context (0x80).
    pub const NETWORK_SYSTEM_CLOCK: usize = 0x80;
}

/// Double-buffered shared memory entry.
///
/// Layout (at each offset):
/// - `[0..4]`: Counter (u32) - indicates which buffer is current
/// - `[4..8]`: Padding
/// - `[8..8+size]`: Buffer 0
/// - `[8+size..8+2*size]`: Buffer 1
///
/// The counter's LSB determines which buffer is active (counter & 1).
#[repr(C)]
struct DoubleBufferedEntry<T> {
    counter: AtomicU32,
    _padding: u32,
    buffers: [T; 2],
}

/// Read a value from shared memory using lock-free double-buffering.
///
/// # Safety
///
/// - `base_ptr` must be a valid pointer to shared memory mapping
/// - `offset` must be valid within the shared memory region
/// - The shared memory must contain a properly initialized double-buffered entry of type `T`
unsafe fn read_shared_mem_obj<T: Copy>(base_ptr: *const u8, offset: usize) -> T {
    // SAFETY: Caller guarantees base_ptr is valid and offset is within bounds
    let entry_ptr = unsafe { base_ptr.add(offset) as *const DoubleBufferedEntry<T> };
    // SAFETY: entry_ptr was just computed from valid base_ptr + offset
    let entry = unsafe { &*entry_ptr };

    loop {
        // Read the counter to determine which buffer is current
        let cur_counter = entry.counter.load(Ordering::Acquire);

        // Read from the active buffer (determined by LSB of counter)
        let buffer_index = (cur_counter & 1) as usize;
        // SAFETY: Accessing shared memory buffer that's guaranteed initialized
        let value = unsafe { ptr::read_volatile(&entry.buffers[buffer_index]) };

        // Memory fence to ensure read completes before counter check
        compiler_fence(Ordering::Acquire);

        // Verify counter hasn't changed during our read
        let new_counter = entry.counter.load(Ordering::Acquire);
        if cur_counter == new_counter {
            return value;
        }
        // Counter changed, retry
    }
}

/// Read the standard steady clock time point from shared memory.
///
/// # Safety
///
/// `base_ptr` must be a valid pointer to the time service shared memory mapping.
pub unsafe fn read_steady_clock(base_ptr: *const u8) -> TimeStandardSteadyClockTimePointType {
    // SAFETY: Caller guarantees base_ptr is valid
    unsafe { read_shared_mem_obj(base_ptr, offsets::STEADY_CLOCK) }
}

/// Read the user/local system clock context from shared memory.
///
/// # Safety
///
/// `base_ptr` must be a valid pointer to the time service shared memory mapping.
pub unsafe fn read_user_system_clock(base_ptr: *const u8) -> TimeSystemClockContext {
    // SAFETY: Caller guarantees base_ptr is valid
    unsafe { read_shared_mem_obj(base_ptr, offsets::USER_SYSTEM_CLOCK) }
}

/// Read the network system clock context from shared memory.
///
/// # Safety
///
/// `base_ptr` must be a valid pointer to the time service shared memory mapping.
pub unsafe fn read_network_system_clock(base_ptr: *const u8) -> TimeSystemClockContext {
    // SAFETY: Caller guarantees base_ptr is valid
    unsafe { read_shared_mem_obj(base_ptr, offsets::NETWORK_SYSTEM_CLOCK) }
}
