use crate::{
    error::KernelError,
    raw::{__nx_svc_arbitrate_lock, __nx_svc_arbitrate_unlock, Handle},
    result::{Error, Result, raw},
};

/// Bitmask for the _waiters bitflag_ in mutex raw tag values.
///
/// When set in a mutex raw tag value, indicates that there are threads waiting to acquire the mutex.
/// The mutex raw tag value is expected to be `owner_thread_handle | HANDLE_WAIT_MASK` when threads
/// are waiting.
pub const HANDLE_WAIT_MASK: u32 = 0x40000000;

/// Arbitrates a mutex lock operation in userspace.
///
/// Attempts to acquire a mutex by arbitrating the lock with the owner thread.
///
/// # Arguments
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _owner_thread_handle_ | The owner thread's kernel handle. Must be a valid thread handle. |
/// | IN | _mutex_ | Pointer to the mutex raw tag value in userspace memory. The mutex raw tag value must be `owner_thread_handle | [`HANDLE_WAIT_MASK`]`. |
/// | IN | _curr_thread_handle_ | The current thread's kernel handle requesting the lock. |
///
/// # Behavior
/// This function calls the [`__nx_svc_arbitrate_lock`] syscall with the provided arguments.
///
/// Then the kernel will:
/// 1. Validate the current thread's state and memory access
/// 2. Check if mutex value matches expected pattern (`owner_thread_handle | HANDLE_WAIT_MASK`)
/// 3. If matched, add current thread to owner's mutex waiter list
/// 4. Pause current thread execution until mutex is released
/// 5. Remove thread from waiter list upon wake-up
///
/// The current thread will be paused until either:
/// - The mutex is released by the owner
/// - The thread is terminated
/// - An error occurs (invalid handle, invalid memory state)
///
/// # Notes
/// - This is a blocking operation that will pause the current thread if the mutex is held
/// - The mutex must be properly initialized before calling this function
/// - Thread handles must belong to the same process
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences a raw pointer (`mutex`)
/// - Interacts directly with thread scheduling and kernel synchronization primitives
pub unsafe fn arbitrate_lock(
    owner_thread_handle: Handle,
    mutex: *mut u32,
    curr_thread_handle: Handle,
) -> Result<(), ArbitrateLockError> {
    let rc = unsafe { __nx_svc_arbitrate_lock(owner_thread_handle, mutex, curr_thread_handle) };
    raw::Result::from_raw(rc).map((), |rc| {
        let desc = rc.description();

        // Map kernel error codes to the appropriate error enum variant
        if KernelError::InvalidHandle == desc {
            ArbitrateLockError::InvalidHandle
        } else if KernelError::InvalidAddress == desc {
            ArbitrateLockError::InvalidMemState
        } else if KernelError::TerminationRequested == desc {
            ArbitrateLockError::ThreadTerminating
        } else {
            ArbitrateLockError::Unknown(Error::from(rc))
        }
    })
}

/// Error type for [`arbitrate_lock`]
#[derive(Debug, thiserror::Error)]
pub enum ArbitrateLockError {
    /// The owner thread handle is invalid.
    #[error("Invalid handle")]
    InvalidHandle,
    /// The mutex memory address cannot be accessed.
    #[error("Invalid memory state")]
    InvalidMemState,
    /// The current thread is marked for termination.
    #[error("Thread terminating")]
    ThreadTerminating,
    /// An unknown error occurred.
    ///
    /// This variant is used when the error code is not recognized.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

/// Arbitrates a mutex unlock operation in userspace.
///
/// Releases a mutex by arbitrating the unlock operation with waiting threads.
///
/// # Arguments
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _mutex_ | Pointer to the mutex tag value in userspace memory. |
///
/// # Behavior
/// This function calls the [`__nx_svc_arbitrate_unlock`] syscall with the provided arguments.
///
/// Then the kernel will:
/// 1. Validate the current thread's state and memory access
/// 2. Update the mutex value to release the lock
/// 3. If there are waiting threads:
///    - Select the next thread to own the mutex.
///    - Update the mutex value with the new owner
///    - Wake up the selected thread
///
/// ## Notes
/// - The current thread must be the owner of the mutex. Otherwise, this is a no-op
///
/// # Safety
/// This function is unsafe because it:
/// - Dereferences a raw pointer (`mutex`)
/// - Interacts directly with thread scheduling and kernel synchronization primitives
pub unsafe fn arbitrate_unlock(mutex: *mut u32) -> Result<(), ArbitrateUnlockError> {
    let rc = unsafe { __nx_svc_arbitrate_unlock(mutex) };
    raw::Result::from_raw(rc).map((), |rc| {
        let desc = rc.description();

        // Map kernel error codes to the appropriate error enum variant
        if KernelError::InvalidAddress == desc {
            ArbitrateUnlockError::InvalidMemState
        } else {
            ArbitrateUnlockError::Unknown(Error::from(rc))
        }
    })
}

/// Error type for [`arbitrate_unlock`]
#[derive(Debug, thiserror::Error)]
pub enum ArbitrateUnlockError {
    /// The mutex memory address cannot be accessed.
    #[error("Invalid memory state")]
    InvalidMemState,
    /// An unknown error occurred.
    ///
    /// This variant is used when the error code is not recognized.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}
