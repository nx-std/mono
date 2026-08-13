//! Synchronization primitives

use core::time::Duration;

use crate::{
    error::{
        _sealed,
        KernelError as KError,
        ResultCode,
        ToResultCode,
    },
    handle::{
        Reset,
        Waitable,
    },
    raw::{
        self,
        Handle,
    },
    result::{
        Error,
        Result,
        raw::Result as RawResult,
    },
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
    /// waited on using `wait_synchronization` or `wait_synchronization_multiple`.
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

impl ToResultCode for ArbitrateLockError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidMemState => KError::InvalidAddress.to_rc(),
            Self::ThreadTerminating => KError::TerminationRequested.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for ArbitrateLockError {}

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

impl ToResultCode for ArbitrateUnlockError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidMemState => KError::InvalidAddress.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for ArbitrateUnlockError {}

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
/// | IN | _timeout_ | How long the wait may last; `None` waits until signalled. |
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
/// - A zero timeout returns immediately after releasing the mutex
/// - A `None` timeout waits indefinitely
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
    timeout: Option<Duration>,
) -> Result<(), WaitProcessWideKeyError> {
    // SAFETY: this function's own `# Safety` contract requires of `condvar` and `mutex` exactly
    // what the syscall does: each addressing an aligned, writable `u32` in this process that stays
    // mapped for the whole wait, with the calling thread owning the mutex. The obligation is
    // forwarded to this call site, not discharged here.
    let res =
        unsafe { raw::wait_process_wide_key_atomic(mutex, condvar, tag, timeout_to_raw(timeout)) };
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

impl ToResultCode for WaitProcessWideKeyError {
    fn to_rc(self) -> ResultCode {
        match self {
            WaitProcessWideKeyError::InvalidMemState => KError::InvalidAddress.to_rc(),
            WaitProcessWideKeyError::ThreadTerminating => KError::TerminationRequested.to_rc(),
            WaitProcessWideKeyError::TimedOut => KError::TimedOut.to_rc(),
            WaitProcessWideKeyError::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for WaitProcessWideKeyError {}

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

/// Upper bound on how many synchronization objects one wait may cover.
///
/// This is the Horizon kernel's own limit. A wait covering more objects than this is rejected
/// with [`WaitSyncError::OutOfRange`] rather than silently narrowed: a caller who believes it is
/// waiting on an object the wait never covered blocks until one of the *other* objects fires,
/// which reads as a hang with no failing call to trace it to.
pub const MAX_WAIT_HANDLES: usize = 64;

/// Blocks the current thread until `handle` is signalled, a timeout expires or the wait is
/// cancelled.
///
/// The one-object case of [`wait_synchronization_multiple`]: there is only one object the wait can
/// return for, so the signalled index carries nothing and this returns `Ok(())`.
///
/// A `None` timeout waits until the object is signalled, however long that takes.
/// `Some(Duration::ZERO)` polls instead: the call returns immediately, with
/// [`WaitSyncError::TimedOut`] standing for "not signalled".
///
/// # Errors
///
/// Returns [`WaitSyncError::TimedOut`] if the timeout expires first, and
/// [`WaitSyncError::InvalidHandle`] if `handle` names no live kernel object this process owns or
/// is one of the pseudo-handles [`raw::CUR_THREAD_HANDLE`] / [`raw::CUR_PROCESS_HANDLE`]. See
/// [`WaitSyncError`] for the rest.
pub fn wait_synchronization<W>(handle: &W, timeout: Option<Duration>) -> Result<(), WaitSyncError>
where
    W: Waitable,
{
    wait_synchronization_raw(&[handle.raw_handle()], timeout).map(|_| ())
}

/// Waits on every object in `handles`, returning the index of the first one that becomes
/// signalled.
///
/// Compared to the low-level [`raw::wait_synchronization`] syscall this helper accepts any type
/// implementing [`Waitable`] and copies the raw handles into a stack buffer for the call, so no
/// caller has to keep an array of raw handle words of its own.
///
/// The returned index is an index into `handles`. A `None` timeout waits until one of the objects
/// is signalled, however long that takes.
///
/// # Errors
///
/// Returns [`WaitSyncError::OutOfRange`] if `handles` is empty or holds more than
/// [`MAX_WAIT_HANDLES`] entries; the wait is not issued in either case. An empty wait is rejected
/// here rather than forwarded because the kernel reads it as "sleep for the timeout" and reports
/// an index the caller has no entry for.
///
/// Otherwise returns [`WaitSyncError::TimedOut`] if the timeout expires first, and
/// [`WaitSyncError::InvalidHandle`] if any entry names no live kernel object this process owns or
/// is one of the pseudo-handles [`raw::CUR_THREAD_HANDLE`] / [`raw::CUR_PROCESS_HANDLE`]. See
/// [`WaitSyncError`] for the rest.
pub fn wait_synchronization_multiple<W>(
    handles: &[W],
    timeout: Option<Duration>,
) -> Result<usize, WaitSyncError>
where
    W: Waitable,
{
    if handles.is_empty() || handles.len() > MAX_WAIT_HANDLES {
        return Err(WaitSyncError::OutOfRange);
    }

    // Copy the raw handle words into a stack buffer: the kernel reads the array while this thread
    // is blocked, and `handles` itself holds `Waitable` values whose layout it knows nothing of.
    let mut raw_handles: [Handle; MAX_WAIT_HANDLES] = [raw::INVALID_HANDLE; MAX_WAIT_HANDLES];
    for (slot, handle) in raw_handles.iter_mut().zip(handles) {
        *slot = handle.raw_handle();
    }

    wait_synchronization_raw(&raw_handles[..handles.len()], timeout)
}

/// Waits on one or more synchronization objects
///
/// Suspends the current thread until one of the given synchronization handles is signalled,
/// a timeout occurs or the wait gets cancelled.
///
/// # Behavior
/// This function calls the [`__nx_svc__svc_wait_synchronization`] syscall under the hood.
/// The kernel will:
/// 1. Validate all provided handles and memory access.
/// 2. If any of the objects are already signalled, return immediately with its index.
/// 3. Otherwise, block the current thread until either:
///    - One of the objects becomes signalled, returning its index.
///    - The timeout expires, giving [`WaitSyncError::TimedOut`].
///    - The wait gets cancelled via [`__nx_svc__svc_cancel_synchronization`], giving
///      [`WaitSyncError::Cancelled`].
///
/// # Errors
///
/// A handle that names no live kernel object, and either of the pseudo-handles
/// [`raw::CUR_THREAD_HANDLE`] / [`raw::CUR_PROCESS_HANDLE`], are answered with
/// [`WaitSyncError::InvalidHandle`]. A `handles` length outside `0..=MAX_WAIT_HANDLES` is
/// answered with [`WaitSyncError::OutOfRange`]. Neither faults, which is why this is a safe
/// function: the kernel validates the array it is handed and reports what it rejected.
fn wait_synchronization_raw(
    handles: &[Handle],
    timeout: Option<Duration>,
) -> Result<usize, WaitSyncError> {
    let mut idx: i32 = -1;

    // The length cast cannot lose a bit: both callers build `handles` from a stack array of
    // `MAX_WAIT_HANDLES` (64) entries, so the length is far inside `i32`.
    let handle_count = handles.len() as i32;

    // SAFETY: The pointer passed to the kernel is valid for `handles.len()` * size_of::<Handle>()
    // bytes because the slice lives on the stack (borrowed from `handles`) for the entire syscall
    // duration and is immutable.
    let rc = unsafe {
        raw::wait_synchronization(
            &mut idx,
            handles.as_ptr(),
            handle_count,
            timeout_to_raw(timeout),
        )
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

impl ToResultCode for WaitSyncError {
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

impl _sealed::Sealed for WaitSyncError {}

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

impl ToResultCode for ResetSignalError {
    fn to_rc(self) -> ResultCode {
        match self {
            ResetSignalError::InvalidHandle => KError::InvalidHandle.to_rc(),
            ResetSignalError::InvalidState => KError::InvalidState.to_rc(),
            ResetSignalError::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for ResetSignalError {}

/// Closes an event handle, decrementing the kernel reference count.
///
/// A service that hands out an event does so as a copy handle, which makes the
/// receiver its owner: the kernel object outlives the reply and stays until
/// every handle naming it is closed.
pub fn close_handle(handle: EventHandle) -> Result<(), CloseHandleError> {
    // SAFETY: the kernel validates the handle and reports an error rather than
    // faulting, including for a handle that was already closed.
    let rc = unsafe { raw::close_handle(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => CloseHandleError::InvalidHandle,
        _ => CloseHandleError::Unknown(Error::from(rc)),
    })
}

/// Error returned by [`close_handle`].
#[derive(Debug, thiserror::Error)]
pub enum CloseHandleError {
    /// The supplied handle does not name a kernel object.
    ///
    /// Occurs when the handle was never valid or has already been closed.
    /// Nothing was closed.
    #[error("invalid handle")]
    InvalidHandle,
    /// An unknown error occurred.
    #[error("unknown error: {0}")]
    Unknown(Error),
}

impl ToResultCode for CloseHandleError {
    fn to_rc(self) -> ResultCode {
        match self {
            CloseHandleError::InvalidHandle => KError::InvalidHandle.to_rc(),
            CloseHandleError::Unknown(err) => err.to_raw(),
        }
    }
}

impl _sealed::Sealed for CloseHandleError {}

/// The raw value the blocking SVCs read as "no deadline".
const TIMEOUT_INFINITE_RAW: u64 = u64::MAX;

/// The longest deadline that is still a deadline.
///
/// One nanosecond short of [`TIMEOUT_INFINITE_RAW`], so that clamping a duration too large to
/// express cannot land on the sentinel and turn a bounded wait into an unbounded one. The
/// difference is 1ns in a wait of roughly 584 years; the difference between bounded and unbounded
/// is a hang.
const TIMEOUT_LONGEST_RAW: u64 = u64::MAX - 1;

/// Encodes a wait deadline in the form the blocking SVCs take.
///
/// Private because no caller outside this module needs it: every blocking wrapper here takes the
/// deadline already typed, so the sentinel is encoded once, at the SVC.
///
/// A deadline is an `Option<Duration>`: `Some(d)` bounds the wait, `None` lets it run until it is
/// woken. That is the shape `std` gives the same idea, and it keeps "no deadline" a value no
/// arithmetic can land on by accident. The SVCs underneath take a plain `u64` of nanoseconds in
/// which `u64::MAX` carries that meaning, and this function is the one place that knows it.
///
/// A `Duration` counts nanoseconds in a `u128`, so it reaches further than that `u64`. One that
/// does not fit is clamped to [`TIMEOUT_LONGEST_RAW`] rather than to the sentinel: a caller asking
/// for a wait longer than the ABI can name still gets a wait that ends.
#[inline]
fn timeout_to_raw(timeout: Option<Duration>) -> u64 {
    let Some(duration) = timeout else {
        return TIMEOUT_INFINITE_RAW;
    };

    match u64::try_from(duration.as_nanos()) {
        Ok(nanos) if nanos < TIMEOUT_INFINITE_RAW => nanos,
        _ => TIMEOUT_LONGEST_RAW,
    }
}

/// Decodes the wait deadline a C caller passed across the FFI boundary.
///
/// The decoding half of the sentinel this module owns. Every `u64` denotes a deadline, so this
/// cannot fail: the sentinel is one specific value and everything else is a nanosecond count.
#[inline]
pub fn timeout_from_raw(raw: u64) -> Option<Duration> {
    match raw {
        TIMEOUT_INFINITE_RAW => None,
        nanos => Some(Duration::from_nanos(nanos)),
    }
}
