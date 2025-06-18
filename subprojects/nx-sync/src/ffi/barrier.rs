//! FFI bindings for the `nx-sync` crate - Barrier
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/barrier.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/barrier.h)

use crate::sys::switch::Barrier;

/// Initializes a new barrier with the specified thread count.
///
/// # Arguments
///
/// * `bar` - Pointer to uninitialized barrier memory
/// * `thread_count` - Number of threads that must reach the barrier before any can proceed
///
/// # Safety
///
/// This function is unsafe because:
/// * `bar` must point to valid memory that can hold a [`Barrier`]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_barrier_init(bar: *mut Barrier, thread_count: u64) {
    unsafe { bar.write(Barrier::new(thread_count)) };
}

/// Waits on the barrier until all threads have reached it.
///
/// # Arguments
///
/// * `bar` - Pointer to an initialized barrier
///
/// # Safety
///
/// This function is unsafe because:
/// * `bar` must point to a valid, initialized [`Barrier`]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_barrier_wait(bar: *mut Barrier) {
    unsafe { &*bar }.wait()
} 
