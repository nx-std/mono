//! Synchronization primitives

use crate::{
    error::{KernelError as KError, ResultCode, ToRawResultCode},
    handle::{Reset, Waitable},
    raw::{self, Handle},
    result::{Error, Result, raw::Result as RawResult},
};

/// Bitmask for the _waiters bitflag_ in mutex raw tag values.
///
/// When set in a mutex raw tag value, indicates that there are threads waiting to acquire the mutex.
/// The mutex raw tag value is expected to be `owner_thread_handle | HANDLE_WAIT_MASK` when threads
/// are waiting.
pub const HANDLE_WAIT_MASK: u32 = 0x40000000;

define_reset_handle_type! {
    /// A handle to a kernel event object (KReadableEvent).
    ///
    /// This represents a waitable event handle obtained from services via copy handles.
    /// Events are signaled by the system when specific conditions occur, and can be
    /// waited on using `wait_synchronization_single` or `wait_synchronization_multiple`.
    ///
    /// # Distinction from SessionHandle
    ///
    /// `EventHandle` is distinct from `SessionHandle` (IPC sessions):
    /// - `EventHandle`: Kernel event objects (KReadableEvent) for notification
    /// - `SessionHandle`: IPC communication channels (IPC sessions)
    ///
    /// # Reset Behavior
    ///
    /// Events obtained from services typically have `autoclear=false`, meaning the
    /// signal must be manually reset using `reset_signal` after waiting. Failure to
    /// reset the signal will cause subsequent waits to return immediately without blocking.
    pub struct EventHandle
}

/// Arbitrates a mutex lock operation in userspace
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
/// This function calls the [`__nx_svc__svc_arbitrate_lock`] syscall with the provided arguments.
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
/// - This is a blocking operation that will pause the current thread if the mutex is held.
/// - The mutex must be properly initialized before calling this function.
/// - Thread handles must belong to the same process.
///
/// # Safety
/// The caller **must uphold** *all* of the following invariants:
/// 1. `mutex` must point to a 4-byte aligned, readable **and writable** `u32` that is mapped in
///    the caller's address space for the whole duration of the call **and** until the mutex is
///    subsequently unlocked.  The pointed-to memory **must not** be unmapped, have its
///    permissions changed or otherwise invalidated while the kernel may access it.
/// 2. `owner_thread_handle` and `curr_thread_handle` are valid kernel handles referring to
///    threads that belong to the **same** process.
/// 3. Immediately before the call, the value stored at `mutex` follows the Horizon mutex format:
///    `owner_thread_handle | HANDLE_WAIT_MASK`.
/// 4. No safe-Rust mutable aliasing of the memory behind `mutex` may happen while the kernel is
///    arbitrating the lock.
///
/// Violating any of these requirements results in **undefined behaviour**.
pub unsafe fn arbitrate_lock(
    owner_thread_handle: Handle,
    mutex: *mut u32,
    curr_thread_handle: Handle,
) -> Result<(), ArbitrateLockError> {
    let rc = unsafe { raw::arbitrate_lock(owner_thread_handle, mutex, curr_thread_handle) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => ArbitrateLockError::InvalidHandle,
        desc if KError::InvalidAddress == desc => ArbitrateLockError::InvalidMemState,
        desc if KError::TerminationRequested == desc => ArbitrateLockError::ThreadTerminating,
        _ => ArbitrateLockError::Unknown(Error::from(rc)),
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

/// Arbitrates a mutex unlock operation in userspace
///
/// Releases a mutex by arbitrating the unlock operation with waiting threads.
///
/// # Arguments
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _mutex_ | Pointer to the mutex tag value in userspace memory. |
///
/// # Behavior
/// This function calls the [`__nx_svc__svc_arbitrate_unlock`] syscall with the provided arguments.
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
/// In addition to the invariants listed for [`arbitrate_lock`], the caller must ensure:
/// 1. The **current thread actually owns** the mutex referenced by `mutex`. Calling this function
///    when the mutex is owned by another thread will lead to kernel-level assertion failures and
///    is therefore *undefined behaviour* from Rust's perspective.
/// 2. The mutex value is in the expected format: `owner_thread_handle | HANDLE_WAIT_MASK`.
/// 3. No safe-Rust mutable aliasing of the memory behind `mutex` may happen while the kernel is
///    arbitrating the unlock.
///
/// Violating any of these requirements results in **undefined behaviour**.
pub unsafe fn arbitrate_unlock(mutex: *mut u32) -> Result<(), ArbitrateUnlockError> {
    let rc = unsafe { raw::arbitrate_unlock(mutex) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidAddress == desc => ArbitrateUnlockError::InvalidMemState,
        _ => ArbitrateUnlockError::Unknown(Error::from(rc)),
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

/// Atomically releases a mutex and waits on a condition variable
///
/// Atomically releases the mutex and suspends the current thread until the condition variable is
/// signaled or a timeout occurs.
///
/// # Arguments
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _condvar_ | Pointer to the condition variable in userspace memory. |
/// | IN | _mutex_ | Pointer to the mutex raw tag value in userspace memory. |
/// | IN | _tag_ | The thread handle value associated with the mutex. |
/// | IN | _timeout_ns_ | Timeout in nanoseconds. Use 0 for no timeout, -1 for infinite wait. |
///
/// # Behavior
/// This function calls the [`__nx_svc__svc_wait_process_wide_key_atomic`] syscall with the provided arguments.
///
/// Then the kernel will:
/// 1. Validate the current thread's state and memory access
/// 2. Release the mutex (updating mutex value and waking waiters)
/// 3. Add the current thread to the condition variable's waiter list
/// 4. Pause the current thread until either:
///    - The condition variable is signaled
///    - The timeout expires (if timeout > 0)
///    - The thread is terminated
/// 5. Remove thread from condition variable waiter list upon wake-up
/// 6. Re-acquire the mutex before returning
///
/// # Notes
/// - This is a blocking operation that will pause the current thread
/// - The mutex must be held by the current thread before calling this function
/// - The operation is atomic - no other thread can acquire the mutex between release and wait
/// - If timeout is 0, returns immediately after releasing mutex
/// - If timeout is -1, waits indefinitely
///
/// # Safety
/// The caller must guarantee:
/// 1. `mutex` and `condvar` each point to a 4-byte aligned, readable **and writable** `u32`
///    residing in the current process' address space. Both pointers must remain valid for the
///    entire wait – which may extend **beyond** the function call if the thread blocks – and until
///    the mutex is re-acquired.
/// 2. The calling thread **owns** the mutex when this function is invoked.
/// 3. After this function returns, the mutex is held again by the calling thread; normal mutex
///    invariants therefore apply.
///
/// Violating any of these requirements results in **undefined behaviour**.
pub unsafe fn wait_process_wide_key_atomic(
    condvar: *mut u32,
    mutex: *mut u32,
    tag: u32,
    timeout_ns: u64,
) -> Result<(), WaitProcessWideKeyError> {
    let res = unsafe { raw::wait_process_wide_key_atomic(mutex, condvar, tag, timeout_ns) };
    RawResult::from_raw(res).map((), |rc| match rc.description() {
        desc if KError::InvalidAddress == desc => WaitProcessWideKeyError::InvalidMemState,
        desc if KError::TerminationRequested == desc => WaitProcessWideKeyError::ThreadTerminating,
        desc if KError::TimedOut == desc => WaitProcessWideKeyError::TimedOut,
        _ => WaitProcessWideKeyError::Unknown(Error::from(rc)),
    })
}

/// Error type for [`wait_process_wide_key_atomic`]
#[derive(Debug, thiserror::Error)]
pub enum WaitProcessWideKeyError {
    /// The mutex or condvar memory address cannot be accessed.
    #[error("Invalid memory state")]
    InvalidMemState,
    /// The current thread is marked for termination.
    #[error("Thread terminating")]
    ThreadTerminating,
    /// The wait operation timed out.
    #[error("Operation timed out")]
    TimedOut,
    /// An unknown error occurred.
    ///
    /// This variant is used when the error code is not recognized.
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for WaitProcessWideKeyError {
    fn to_rc(self) -> ResultCode {
        match self {
            WaitProcessWideKeyError::InvalidMemState => KError::InvalidAddress.to_rc(),
            WaitProcessWideKeyError::ThreadTerminating => KError::TerminationRequested.to_rc(),
            WaitProcessWideKeyError::TimedOut => KError::TimedOut.to_rc(),
            WaitProcessWideKeyError::Unknown(err) => err.to_raw(),
        }
    }
}

/// Signals a condition variable to wake waiting threads
///
/// Wakes up one or more threads waiting on the specified condition variable.
///
/// # Arguments
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _condvar_ | Pointer to the condition variable in userspace memory. |
/// | IN | _count_ | Number of threads to wake. If greater than the number of waiting threads, all threads are woken. If less than or equal to 0, wakes all waiting threads. |
///
/// # Behavior
/// This function calls the [`__nx_svc__svc_signal_process_wide_key`] syscall with the provided arguments.
///
/// Then the kernel will:
/// 1. Select threads to wake based on:
///    - Threads must be waiting on the specified condition variable
///    - Threads are ordered by their dynamic priority
///    - Up to _count_ threads are selected (or all threads if _count_ ≤ 0, e.g. -1)
/// 2. For each selected thread:
///    - Remove it from the condition variable's waiter list
///    - Attempt to re-acquire its associated mutex
/// 3. If no threads remain waiting:
///    - Reset the condition variable value to the default value
///
/// # Notes
/// - This is a non-blocking operation
/// - If no threads are waiting on the condition variable, this is effectively a no-op
/// - Woken threads will attempt to re-acquire their associated mutexes before resuming
/// - Thread selection is priority-aware, favoring threads with higher dynamic priority
///
/// # Safety
/// The caller must ensure that `condvar` is a valid, 4-byte aligned, writable pointer to a `u32`
/// located in process memory. The pointed-to memory must stay valid until all woken threads have
/// attempted to re-acquire their mutex. Passing an invalid pointer or allowing the memory to be
/// unmapped while the kernel still references it constitutes undefined behaviour.
pub unsafe fn signal_process_wide_key(condvar: *mut u32, count: i32) {
    unsafe { raw::signal_process_wide_key(condvar, count) };
}

/// Upper bound on how many synchronization objects the high-level public wrappers
/// ([`wait_synchronization_multiple`] and [`wait_synchronization_single`]) will forward to the
/// kernel.
///
/// If the caller supplies a longer slice, only the first `MAX_WAIT_HANDLES` elements are forwarded
/// and the remainder is **silently ignored**.  This mirrors the Horizon kernel limit (64) while
/// avoiding a panic or allocation inside the wrapper.
pub const MAX_WAIT_HANDLES: usize = 64;

/// Blocks the current thread until `handle` is signalled, a timeout expires or the wait is
/// cancelled.
///
/// This is a convenience wrapper around [`wait_synchronization_multiple`] that forwards exactly
/// one handle.  On success it discards the signalled‐index information and simply returns `Ok(())`.
///
/// # Arguments
/// * `handle`     – Object implementing [`Waitable`].  Its underlying kernel handle is extracted
///   via [`Waitable::raw_handle`].
/// * `timeout`    – Timeout in nanoseconds (use `u64::MAX` for an infinite wait, `0` for an
///   immediate check).
///
/// Returns an [`Ok(())`] on success or a [`WaitSyncError`] if the wait fails.
///
/// # Safety
/// See [`wait_synchronization_multiple`].  In addition, the caller must uphold those invariants
/// for the single `handle` passed here.
pub unsafe fn wait_synchronization_single<W>(handle: &W, timeout: u64) -> Result<(), WaitSyncError>
where
    W: Waitable,
{
    // SAFETY: We forward a single raw handle; the slice we create lives on the stack for the full
    // duration of the syscall and thus fulfils the memory invariants documented below.
    unsafe { wait_synchronization(&[handle.raw_handle()], timeout).map(|_| ()) }
}

/// Waits on *up to* [`MAX_WAIT_HANDLES`] objects, returning the index of the first one that becomes
/// signalled.
///
/// Compared to the low-level [`raw::wait_synchronization`] syscall this helper accepts any type
/// implementing [`Waitable`], automatically truncates slices longer than the kernel's maximum and
/// hides the `unsafe` call site from the user.
///
/// # Arguments
/// * `handles`    – Any iterator yielding [`Waitable`] objects. If it produces more than
///   [`MAX_WAIT_HANDLES`] elements, only the first `MAX_WAIT_HANDLES` values are
///   considered; additional items are **silently ignored**.
/// * `timeout`    – Timeout in nanoseconds (`u64::MAX` ≅ infinite).
///
/// Returns an [`Ok(usize)`] indicating the index of the signalled handle on success or a
/// [`WaitSyncError`] if the wait fails.
///
/// # Safety
/// The caller must guarantee that **each handle yielded by the iterator and actually forwarded to
/// the kernel (i.e. the first *n* items with `n ≤ MAX_WAIT_HANDLES`)**:
/// 1. Yields a *valid* kernel handle owned by the current process.
/// 2. Is *not* one of the special pseudo-handles [`raw::CUR_THREAD_HANDLE`] or
///    [`raw::CUR_PROCESS_HANDLE`].
///
/// The underlying kernel handles must remain valid for the entire duration of the wait.  No
/// additional memory-safety requirements apply because the values are copied into a stack buffer
/// before the syscall is issued.
pub unsafe fn wait_synchronization_multiple<'a, W, I>(
    handles: I,
    timeout: u64,
) -> Result<usize, WaitSyncError>
where
    W: Waitable + 'a,
    I: IntoIterator<Item = &'a W>,
{
    // Limit the number of handles to the kernel limit
    let handles_iter = handles.into_iter().take(MAX_WAIT_HANDLES);

    // Build a stack-allocated array and copy up to `MAX_WAIT_HANDLES` raw handles into it.
    let mut raw_handles: [Handle; MAX_WAIT_HANDLES] = [raw::INVALID_HANDLE; MAX_WAIT_HANDLES];
    let mut raw_handles_count = 0usize;
    for (slot, h) in raw_handles.iter_mut().zip(handles_iter) {
        *slot = h.raw_handle();
        raw_handles_count += 1;
    }

    // SAFETY: We forward at most `MAX_WAIT_HANDLES` handles, each obtained from a `Waitable`
    // supplied by the caller. The slice lives on the stack for the entire syscall.
    unsafe { wait_synchronization(&raw_handles[..raw_handles_count], timeout) }
}

/// Waits on one or more synchronization objects
///
/// Suspends the current thread until one of the given synchronization handles is signalled,
/// a timeout occurs or the wait gets cancelled.
///
/// # Arguments
/// | Arg | Name | Description |
/// | --- | --- | --- |
/// | IN | _handles_ | Slice of raw kernel handles. If the slice is longer than [`MAX_WAIT_HANDLES`], only the first [`MAX_WAIT_HANDLES`] elements are considered. |
/// | IN | _timeout_ | Timeout in nanoseconds. Use `u64::MAX` for an infinite wait, `0` for an immediate check. |
///
/// Returns an [`Ok(usize)`] indicating the index of the signalled handle on success or a
/// [`WaitSyncError`] if the wait fails.
///
/// # Behavior
/// This function calls the [`__nx_svc__svc_wait_synchronization`] syscall under the hood.
/// The kernel will:
/// 1. Validate all provided handles and memory access.
/// 2. If any of the objects are already signalled, return immediately with its index.
/// 3. Otherwise, block the current thread until either:
///    - One of the objects becomes signalled → success, returning its index.
///    - The timeout expires              → [`WaitSyncError::TimedOut`].
///    - The wait gets cancelled via [`__nx_svc__svc_cancel_synchronization`] → [`WaitSyncError::Cancelled`].
///
/// # Notes
/// - Passing an empty slice results in a sleep until `timeout_ns` elapses (or indefinitely when
///   `timeout_ns == u64::MAX`). In that case the returned index value is implementation-defined and
///   should not be relied upon.
/// - The special pseudo-handles [`raw::CUR_THREAD_HANDLE`] and [`raw::CUR_PROCESS_HANDLE`] **must not**
///   appear among the first [`MAX_WAIT_HANDLES`] entries – doing so triggers
///   [`WaitSyncError::InvalidHandle`].
/// - The error variant [`WaitSyncError::OutOfRange`] is unlikely to be returned by this
///   wrapper because the argument list is clamped to [`MAX_WAIT_HANDLES`] before the syscall is issued;
///   it is kept for forward-compatibility.
///
/// # Safety
/// The caller must uphold the following invariants:
/// 1. Only the first `handles.len().min(MAX_WAIT_HANDLES)` entries are forwarded. Each of those
///    handles **must** be valid, owned by the current process and **must not** be one of the
///    pseudo-handles [`raw::CUR_THREAD_HANDLE`] or [`raw::CUR_PROCESS_HANDLE`].
/// 2. The memory backing the `handles` slice must remain valid and immutable for the entire
///    duration of the syscall (it is read by the kernel while the thread is in user-space).
///
/// Violating any of these requirements results in **undefined behaviour**.
unsafe fn wait_synchronization(handles: &[Handle], timeout: u64) -> Result<usize, WaitSyncError> {
    let mut idx: i32 = -1;

    // SAFETY: The pointer passed to the kernel is valid for `handles.len()` * size_of::<Handle>()
    // bytes because the slice lives on the stack (borrowed from `handles`) for the entire syscall
    // duration and is immutable.
    let rc = unsafe {
        raw::wait_synchronization(&mut idx, handles.as_ptr(), handles.len() as i32, timeout)
    };

    RawResult::from_raw(rc).map(idx as usize, |rc| match rc.description() {
        desc if KError::TerminationRequested == desc => WaitSyncError::TerminationRequested,
        desc if KError::InvalidHandle == desc => WaitSyncError::InvalidHandle,
        desc if KError::InvalidPointer == desc => WaitSyncError::InvalidPointer,
        desc if KError::TimedOut == desc => WaitSyncError::TimedOut,
        desc if KError::Cancelled == desc => WaitSyncError::Cancelled,
        desc if KError::OutOfRange == desc => WaitSyncError::OutOfRange,
        _ => WaitSyncError::Unknown(Error::from(rc)),
    })
}

/// Error type returned by wait synchronization functions.
///
/// Based on Atmosphere kernel implementation (`kern_svc_synchronization.cpp`),
/// these are ALL possible error codes from `svcWaitSynchronization`:
///
/// | Code | Description | Condition |
/// |------|-------------|-----------|
/// | 59   | TerminationRequested | Thread is being terminated |
/// | 114  | InvalidHandle | Handle doesn't exist or wrong type |
/// | 115  | InvalidPointer | Invalid user-space pointer (internal) |
/// | 117  | TimedOut | Wait timed out |
/// | 118  | Cancelled | Wait cancelled via CancelSynchronization |
/// | 119  | OutOfRange | num_handles < 0 or > 0x40 |
///
/// The `Unknown` catch-all is kept for forward-compatibility in case Nintendo extends the
/// interface with additional error codes.
#[derive(Debug, thiserror::Error)]
pub enum WaitSyncError {
    /// Thread termination was requested while waiting.
    #[error("termination requested")]
    TerminationRequested,
    /// One (or more) of the supplied handles is invalid.
    #[error("invalid handle")]
    InvalidHandle,
    /// Invalid pointer to handle array (internal kernel error).
    #[error("invalid pointer")]
    InvalidPointer,
    /// The wait operation timed out.
    #[error("operation timed out")]
    TimedOut,
    /// The wait was cancelled via `CancelSynchronization` SVC.
    #[error("wait cancelled")]
    Cancelled,
    /// The number of handles supplied is out of range (must be 0..=64).
    #[error("out of range")]
    OutOfRange,
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for WaitSyncError {
    fn to_rc(self) -> ResultCode {
        match self {
            WaitSyncError::TerminationRequested => KError::TerminationRequested.to_rc(),
            WaitSyncError::InvalidHandle => KError::InvalidHandle.to_rc(),
            WaitSyncError::InvalidPointer => KError::InvalidPointer.to_rc(),
            WaitSyncError::TimedOut => KError::TimedOut.to_rc(),
            WaitSyncError::Cancelled => KError::Cancelled.to_rc(),
            WaitSyncError::OutOfRange => KError::OutOfRange.to_rc(),
            WaitSyncError::Unknown(err) => err.to_raw(),
        }
    }
}

/// Resets a signaled synchronization object.
///
/// This clears the signal state of an event, allowing subsequent waits
/// to block until the object is signaled again.
///
/// Based on Atmosphere kernel implementation (`kern_svc_synchronization.cpp`),
/// these are ALL possible error codes from `svcResetSignal`:
///
/// | Code | Description | Condition |
/// |------|-------------|-----------|
/// | 114  | InvalidHandle | Handle doesn't refer to resettable object |
/// | 125  | InvalidState | Object is not currently signaled |
///
/// # Safety
///
/// The handle must be a valid synchronization object that supports reset.
pub unsafe fn reset_signal<T: Reset>(handle: &T) -> Result<(), ResetSignalError> {
    // SAFETY: Caller ensures handle is valid and supports reset
    let rc = unsafe { raw::reset_signal(handle.raw_handle()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => ResetSignalError::InvalidHandle,
        desc if KError::InvalidState == desc => ResetSignalError::InvalidState,
        _ => ResetSignalError::Unknown(Error::from(rc)),
    })
}

/// Error type returned by [`reset_signal`].
#[derive(Debug, thiserror::Error)]
pub enum ResetSignalError {
    /// The handle does not refer to a resettable object.
    #[error("invalid handle")]
    InvalidHandle,
    /// The object is not in a signaled state (cannot reset a non-signaled object).
    #[error("invalid state")]
    InvalidState,
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for ResetSignalError {
    fn to_rc(self) -> ResultCode {
        match self {
            ResetSignalError::InvalidHandle => KError::InvalidHandle.to_rc(),
            ResetSignalError::InvalidState => KError::InvalidState.to_rc(),
            ResetSignalError::Unknown(err) => err.to_raw(),
        }
    }
}
