//! Thread management for Horizon OS (Nintendo Switch)
//!
//! This module provides a thin, `no_std`-friendly wrapper around the Switch
//! kernel's thread-related SVCs.  Each safe wrapper maps almost one-to-one to
//! its underlying system call while translating raw [`ResultCode`] values into
//! strongly typed Rust error enums.

use core::ffi::c_void;

use crate::{
    error::{KernelError as KError, ToRawResultCode},
    raw,
    result::{Error, ResultCode, raw::Result as RawResult},
};

define_waitable_handle_type! {
    /// A handle to a thread kernel object.
    pub struct Handle
}

impl Handle {
    /// Creates a new [`Handle`] for the current thread.
    pub fn current_thread() -> Self {
        Self(raw::CUR_THREAD_HANDLE)
    }

    /// Returns `true` if the handle is the current thread.
    pub fn is_current_thread(&self) -> bool {
        self.0 == raw::CUR_THREAD_HANDLE
    }
}

/// Creates a new thread in the *created* (suspended) state.
///
/// This is a safe wrapper around [`raw::create_thread`] that forwards its
/// parameters verbatim:
///
/// * `entry` – pointer to the thread's entry function.
/// * `arg` – argument passed unchanged to `entry`.
/// * `stack_top` – top-of-stack pointer (must be 16-byte aligned and remain
///   valid for the thread's entire lifetime).
/// * `prio` – thread priority in the range `0..=0x3F` (lower values indicate
///   higher priority).
/// * `cpuid` – target CPU core ID (`-2` for no affinity).
///
/// On success, returns a [`Handle`] to the newly created thread.  The thread
/// must subsequently be transitioned to *runnable* with [`start`] before it can
/// execute.
///
/// On failure, the function yields a [`CreateThreadError`] detailing the cause.
pub fn create(
    entry: *mut c_void,
    arg: *mut c_void,
    stack_top: *mut c_void,
    prio: i32,
    cpuid: i32,
) -> Result<Handle, CreateThreadError> {
    let mut handle: raw::Handle = raw::INVALID_HANDLE;
    let rc = unsafe { raw::create_thread(&mut handle, entry, arg, stack_top, prio, cpuid) };

    RawResult::from_raw(rc).map(Handle(handle), |rc| match rc.description() {
        desc if KError::OutOfMemory == desc => CreateThreadError::OutOfMemory,
        desc if KError::OutOfResource == desc => CreateThreadError::OutOfResource,
        desc if KError::LimitReached == desc => CreateThreadError::LimitReached,
        desc if KError::OutOfHandles == desc => CreateThreadError::OutOfHandles,
        desc if KError::InvalidPriority == desc => CreateThreadError::InvalidPriority,
        desc if KError::InvalidCoreId == desc => CreateThreadError::InvalidCoreId,
        _ => CreateThreadError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum CreateThreadError {
    #[error("Out of memory")]
    OutOfMemory,
    /// The kernel ran out of generic thread-related resources — maps to
    /// `KernelError::OutOfResource` (raw code `0x267`).
    #[error("Out of generic thread resources")]
    OutOfResource,
    /// The per-process thread quota has been exhausted —
    /// `KernelError::LimitReached` (raw code `0x284`).
    #[error("Thread limit reached for process")]
    LimitReached,
    /// The process handle table contains no free slots —
    /// `KernelError::OutOfHandles` (raw code `0x269`).
    #[error("Handle table full")]
    OutOfHandles,
    /// The supplied priority is outside `0..=0x3F` or not permitted by the
    /// process — `KernelError::InvalidPriority` (raw code `0x270`).
    #[error("Invalid priority")]
    InvalidPriority,
    /// The requested CPU core is invalid or outside the process affinity mask —
    /// `KernelError::InvalidCoreId` (raw code `0x271`).
    #[error("Invalid core id")]
    InvalidCoreId,
    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for CreateThreadError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::OutOfMemory => KError::OutOfMemory.to_rc(),
            Self::OutOfResource => KError::OutOfResource.to_rc(),
            Self::LimitReached => KError::LimitReached.to_rc(),
            Self::OutOfHandles => KError::OutOfHandles.to_rc(),
            Self::InvalidPriority => KError::InvalidPriority.to_rc(),
            Self::InvalidCoreId => KError::InvalidCoreId.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Transitions a thread from the *created* state to *runnable*.
///
/// The target `handle` must refer to a thread that has been successfully
/// spawned with [`create`] and not yet started.  Attempting to start an
/// already-running or invalid thread results in [`StartThreadError::InvalidHandle`].
pub fn start(handle: Handle) -> Result<(), StartThreadError> {
    let rc = unsafe { raw::start_thread(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => StartThreadError::InvalidHandle,
        _ => StartThreadError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum StartThreadError {
    /// The supplied handle is not a valid thread handle —
    /// `KernelError::InvalidHandle` (raw code `0xE401`).
    #[error("Invalid handle")]
    InvalidHandle,
    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for StartThreadError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Pauses a thread.
///
/// Under the hood this invokes [`raw::set_thread_activity`] with [`ThreadActivity::Paused`].
/// The operation is asynchronous: a successful return only indicates the request was enqueued.
pub fn pause(handle: Handle) -> Result<(), PauseThreadError> {
    let rc = unsafe { raw::set_thread_activity(handle.to_raw(), raw::ThreadActivity::Paused) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => PauseThreadError::InvalidHandle,
        _ => PauseThreadError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum PauseThreadError {
    /// The supplied handle is not a valid thread handle —
    /// `KernelError::InvalidHandle` (raw code `0xE401`).
    #[error("Invalid handle")]
    InvalidHandle,
    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for PauseThreadError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Resumes a previously paused thread.
///
/// Under the hood this invokes [`raw::set_thread_activity`] with [`ThreadActivity::Runnable`].
/// The operation is asynchronous: a successful return only indicates the request was enqueued.
pub fn resume(handle: Handle) -> Result<(), ResumeThreadError> {
    let rc = unsafe { raw::set_thread_activity(handle.to_raw(), raw::ThreadActivity::Runnable) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => ResumeThreadError::InvalidHandle,
        _ => ResumeThreadError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ResumeThreadError {
    /// The supplied handle is not a valid thread handle —
    /// `KernelError::InvalidHandle` (raw code `0xE401`).
    #[error("Invalid handle")]
    InvalidHandle,
    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for ResumeThreadError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Exits the current thread and never returns.
///
/// Internally this issues the `svcExitThread` syscall. The kernel will perform
/// final housekeeping, dispose of TLS, and pick another thread to schedule.
pub fn exit() -> ! {
    unsafe { raw::exit_thread() }
}

/// Closes (dereferences) a thread handle without affecting the thread's
/// execution.
///
/// This mirrors the semantics of [`raw::close_handle`]: the underlying kernel
/// object is only destroyed once **all** outstanding handles are closed.  In
/// particular, calling this on the current thread's handle does **not** abort
/// the thread—it merely drops the user-space reference.
pub fn close_handle(handle: Handle) -> Result<(), CloseHandleError> {
    let rc = unsafe { raw::close_handle(handle.to_raw()) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => CloseHandleError::InvalidHandle,
        _ => CloseHandleError::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum CloseHandleError {
    /// The supplied handle is not a valid thread handle —
    /// `KernelError::InvalidHandle` (raw code `0xE401`).
    #[error("Invalid handle")]
    InvalidHandle,
    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for CloseHandleError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Dumps the CPU context of a *paused* thread into `ctx`.
///
/// The target thread must have been paused beforehand (see [`pause`]) to ensure
/// a consistent snapshot.
pub fn get_context3(thread: Handle) -> Result<raw::ThreadContext, GetContext3Error> {
    let mut ctx = raw::ThreadContext::zeroed();
    let rc = unsafe { raw::get_thread_context3(&mut ctx, thread.0) };
    RawResult::from_raw(rc).map(ctx, |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => GetContext3Error::InvalidHandle,
        _ => GetContext3Error::Unknown(rc.into()),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum GetContext3Error {
    #[error("Invalid handle")]
    InvalidHandle,
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for GetContext3Error {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

/// Suspends the current thread for *at least* the specified number of
/// nanoseconds.
///
/// Note: `svcSleepThread` takes an `i64`, but negative values are used for yielding,
/// which is a different concern. This function only handles sleeping and will cap
/// the input at `i64::MAX`.
pub fn sleep(nanos: u64) {
    let nanos = nanos.min(i64::MAX as u64) as i64;
    unsafe { raw::sleep_thread(nanos) }
}

/// Yields execution to a different thread that is scheduled on the *same* CPU
/// core.
///
/// This function calls the `svcSleepThread` syscall with `raw::YieldType::NoMigration` (0),
/// signaling the kernel to yield to a different thread scheduled on the same CPU core.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
pub fn yield_no_migration() {
    unsafe { raw::sleep_thread(raw::YieldType::NoMigration as i64) }
}

/// Yields execution to another thread, permitting migration to a different CPU
/// core.
///
/// This function calls the `svcSleepThread` syscall with `raw::YieldType::WithMigration` (-1),
/// signaling the kernel to yield to a different thread, which may be on another CPU core.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
pub fn yield_with_migration() {
    unsafe { raw::sleep_thread(raw::YieldType::WithMigration as i64) }
}

/// Yields execution to any other thread, forcing cross-core load-balancing.
///
/// This function calls the `svcSleepThread` syscall with `raw::YieldType::ToAnyThread` (-2),
/// signaling the kernel to yield and perform a forced load-balancing of threads across cores.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
pub fn yield_to_any_thread() {
    unsafe { raw::sleep_thread(raw::YieldType::ToAnyThread as i64) }
}
