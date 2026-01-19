//! Thread management for Horizon OS (Nintendo Switch)
//!
//! This module provides a thin, `no_std`-friendly wrapper around the Switch
//! kernel's thread-related SVCs. Each safe wrapper maps almost one-to-one to
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
    pub const fn current_thread() -> Self {
        Self(raw::CUR_THREAD_HANDLE)
    }

    /// Returns `true` if the handle is the current thread.
    pub const fn is_current_thread(&self) -> bool {
        self.0 == raw::CUR_THREAD_HANDLE
    }
}

/// Creates a new thread in the *created* (suspended) state.
///
/// This is a wrapper around [`raw::create_thread`] that forwards its
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
///
/// # Safety
///
/// The caller must ensure:
/// - `entry` points to a valid function with the correct signature
/// - `arg` is valid to pass to the entry function (or null)
/// - `stack_top` points to a valid, 16-byte aligned stack that remains valid
///   for the thread's entire lifetime
pub unsafe fn create(
    entry: *mut c_void,
    arg: *mut c_void,
    stack_top: *mut c_void,
    prio: i32,
    cpuid: i32,
) -> Result<Handle, CreateThreadError> {
    let mut handle = raw::INVALID_HANDLE;
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

/// Gets the current processor/CPU core number.
///
/// Returns the ID of the CPU core that the current thread is running on.
/// The returned value is in the range 0..3 for the Switch's quad-core processor.
///
/// This is a safe wrapper around [`raw::get_current_processor_number`].
pub fn get_current_processor_number() -> u32 {
    // SAFETY: svcGetCurrentProcessorNumber is always safe to call and
    // returns the current processor number without any side effects
    unsafe { raw::get_current_processor_number() }
}

/// Sets the CPU core affinity for a thread.
///
/// This function configures which CPU cores the specified thread is allowed
/// to run on and optionally which core it prefers. You can pass either:
/// - [`CoreAffinity`] - A type-safe enum with validation for common affinity configurations
/// - [`RawCoreAffinity`] - A struct for passing raw values without validation
pub fn set_core_mask(
    handle: Handle,
    affinity: impl IntoCoreAffinityParams,
) -> Result<(), SetCoreMaskError> {
    let (core_id, affinity_mask) = affinity.to_core_id_and_mask();
    let rc = unsafe { raw::set_thread_core_mask(handle.to_raw(), core_id, affinity_mask) };
    RawResult::from_raw(rc).map((), |rc| match rc.description() {
        desc if KError::InvalidHandle == desc => SetCoreMaskError::InvalidHandle,
        desc if KError::InvalidCoreId == desc => SetCoreMaskError::InvalidCoreId,
        desc if KError::InvalidCombination == desc => SetCoreMaskError::InvalidCombination,
        desc if KError::TerminationRequested == desc => SetCoreMaskError::TerminationRequested,
        _ => SetCoreMaskError::Unknown(rc.into()),
    })
}

/// CPU core affinity configuration for threads.
///
/// This enum represents all the different ways to configure which CPU cores
/// a thread can run on and which core it prefers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreAffinity {
    /// Run on a specific preferred core with the given affinity mask.
    Specific(SpecificCoreAffinity),

    /// Use any core specified in the affinity mask with no preferred core.
    Any(AnyCoreAffinity),

    /// Use the process's default core.
    ///
    /// This instructs the kernel to use the process's default core configuration.
    /// The kernel handles this case specially by setting both the preferred core
    /// and affinity mask to match the process's default core.
    ProcessDefault(ProcessDefaultCoreAffinity),

    /// Preserve the current preferred core and apply the new affinity mask.
    NoUpdate(NoUpdateCoreAffinity),
}

/// Trait for types that can be converted into core affinity configurations.
///
/// This trait provides a way to convert various types into the (core_id, affinity_mask)
/// tuple format expected by the underlying SVC calls.
pub trait IntoCoreAffinityParams: _priv::Sealed {
    /// Converts the type into a (core_id, affinity_mask) tuple.
    ///
    /// Returns the core_id and affinity_mask tuple for the underlying SVC call.
    fn to_core_id_and_mask(self) -> (i32, u32);
}

impl CoreAffinity {
    /// Creates a [`CoreAffinity`] for a specific preferred core, validating the core number and affinity mask.
    ///
    /// # Parameters
    /// - `core`: Preferred CPU core number (valid range: 0-3 for Nintendo Switch)
    /// - `mask`: Mask of allowed cores using [`CoreAffinityMask`]
    ///
    /// The preferred core must be included in the affinity mask.
    ///
    /// Returns an error if the core number is invalid (>= 4), the affinity mask is empty,
    /// or the preferred core is not included in the affinity mask.
    pub fn specific(
        core: u8,
        mask: CoreAffinityMask,
    ) -> Result<CoreAffinity, SpecificCoreAffinityError> {
        // Validate core number
        if core >= 4 {
            return Err(SpecificCoreAffinityError::InvalidCore(InvalidCoreError {
                core,
            }));
        }

        // Check that the affinity mask is not empty
        if mask.is_empty() {
            return Err(SpecificCoreAffinityError::InvalidAffinityMask(
                InvalidAffinityMaskError { mask: mask.bits() },
            ));
        }

        // Check that the specific core is included in the affinity mask
        let core_mask = CoreAffinityMask::from_bits_truncate(1 << core);
        if !mask.contains(core_mask) {
            return Err(SpecificCoreAffinityError::InvalidAffinityMask(
                InvalidAffinityMaskError { mask: mask.bits() },
            ));
        }

        Ok(CoreAffinity::Specific(SpecificCoreAffinity { core, mask }))
    }

    /// Creates a [`CoreAffinity`] for any core in the affinity mask.
    ///
    /// # Parameters
    /// - `mask`: Mask of allowed cores using [`CoreAffinityMask`]
    ///
    /// The thread will have no preferred core and the scheduler will choose
    /// from any core allowed by the affinity mask.
    ///
    /// Returns an error if the affinity mask is empty.
    pub fn any(mask: CoreAffinityMask) -> Result<CoreAffinity, InvalidAffinityMaskError> {
        // Validate affinity mask: must not be empty
        if mask.is_empty() {
            return Err(InvalidAffinityMaskError { mask: mask.bits() });
        }

        Ok(CoreAffinity::Any(AnyCoreAffinity { mask }))
    }

    /// Creates a [`CoreAffinity`] that uses the process's default core.
    pub fn process_default() -> CoreAffinity {
        CoreAffinity::ProcessDefault(ProcessDefaultCoreAffinity { _priv: () })
    }

    /// Creates a [`CoreAffinity`] that preserves the current preferred core and updates the affinity mask.
    ///
    /// # Parameters  
    /// - `mask`: Mask of allowed cores using [`CoreAffinityMask`]
    ///
    /// The thread's current preferred core setting is preserved, and only the
    /// affinity mask is updated with the new value.
    ///
    /// Returns an error if the affinity mask is empty.
    pub fn no_update(mask: CoreAffinityMask) -> Result<CoreAffinity, InvalidAffinityMaskError> {
        // Validate affinity mask: Must not be empty
        if mask.is_empty() {
            return Err(InvalidAffinityMaskError { mask: mask.bits() });
        }

        Ok(CoreAffinity::NoUpdate(NoUpdateCoreAffinity { mask }))
    }
}

impl IntoCoreAffinityParams for CoreAffinity {
    fn to_core_id_and_mask(self) -> (i32, u32) {
        match self {
            CoreAffinity::Specific(SpecificCoreAffinity { core, mask }) => {
                (core as i32, mask.bits())
            }
            CoreAffinity::Any(AnyCoreAffinity { mask }) => (-1, mask.bits()),
            CoreAffinity::ProcessDefault(_) => (-2, 0),
            CoreAffinity::NoUpdate(NoUpdateCoreAffinity { mask }) => (-3, mask.bits()),
        }
    }
}

impl _priv::Sealed for CoreAffinity {}

/// Configuration for running a thread on a specific preferred core.
///
/// The preferred core (0-3) must be present in the affinity mask,
/// or the operation will fail with `InvalidCombination`. The thread will
/// prefer to run on the specified core but can migrate to other cores
/// allowed by the mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecificCoreAffinity {
    core: u8,
    mask: CoreAffinityMask,
}

/// Error type for core affinity validation failures.
#[derive(Debug, thiserror::Error)]
pub enum SpecificCoreAffinityError {
    /// Invalid core number.
    #[error(transparent)]
    InvalidCore(#[from] InvalidCoreError),
    /// Invalid affinity mask.
    #[error(transparent)]
    InvalidAffinityMask(#[from] InvalidAffinityMaskError),
}

/// Configuration for running a thread on any core in the affinity mask.
///
/// The thread has no preferred core and the scheduler will choose
/// from any core allowed by the affinity mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnyCoreAffinity {
    mask: CoreAffinityMask,
}

/// Configuration for using the process's default core.
///
/// This instructs the kernel to use the process's default core configuration.
/// The kernel handles this specially by configuring both the preferred core
/// and affinity mask to match the process's default core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessDefaultCoreAffinity {
    _priv: (),
}

/// Configuration for updating only the affinity mask while preserving the current preferred core.
///
/// The thread's current preferred core setting is kept unchanged,
/// and only the affinity mask is updated with the new value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoUpdateCoreAffinity {
    mask: CoreAffinityMask,
}

/// Raw core affinity configuration without validation.
///
/// This struct allows passing arbitrary core_id and affinity_mask values
/// directly to the underlying SVC without any validation. Use with caution
/// as invalid values may cause kernel errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawCoreAffinity {
    core_id: i32,
    affinity_mask: u32,
}

impl RawCoreAffinity {
    /// Creates a new [`RawCoreAffinity`] with the specified raw values without validation.
    ///
    /// # Safety
    ///
    /// This function does not validate the input values. Invalid values may cause
    /// kernel errors when passed to the SVC. Use with caution.
    pub const fn new_unchecked(core_id: i32, affinity_mask: u32) -> Self {
        Self {
            core_id,
            affinity_mask,
        }
    }
}

impl IntoCoreAffinityParams for RawCoreAffinity {
    fn to_core_id_and_mask(self) -> (i32, u32) {
        (self.core_id, self.affinity_mask)
    }
}

impl _priv::Sealed for RawCoreAffinity {}

/// Error type for invalid core numbers.
#[derive(Debug, thiserror::Error)]
#[error("Invalid core number {core}: must be in range 0..4")]
pub struct InvalidCoreError {
    /// The invalid core number that was provided.
    pub core: u8,
}

/// Error type for invalid affinity masks.
#[derive(Debug, thiserror::Error)]
#[error(
    "Invalid affinity mask 0x{mask:X}: must be non-zero and only use bits 0-3 (valid range: 0x1..=0xF)"
)]
pub struct InvalidAffinityMaskError {
    /// The invalid affinity mask that was provided.
    pub mask: u32,
}

bitflags::bitflags! {
    /// CPU core affinity mask for thread scheduling.
    ///
    /// Each bit represents whether a thread can run on the corresponding core.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CoreAffinityMask: u32 {
        /// Allow thread to run on core 0
        const CORE_0 = 1 << 0;
        /// Allow thread to run on core 1
        const CORE_1 = 1 << 1;
        /// Allow thread to run on core 2
        const CORE_2 = 1 << 2;
        /// Allow thread to run on core 3
        const CORE_3 = 1 << 3;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SetCoreMaskError {
    /// The supplied handle is not a valid thread handle —
    /// `KernelError::InvalidHandle` (raw code `0xE401`).
    #[error("Invalid handle")]
    InvalidHandle,

    /// The specified core ID is invalid or affinity mask contains disallowed cores —
    /// `KernelError::InvalidCoreId` (raw code `0x271`).
    #[error("Invalid core id")]
    InvalidCoreId,

    /// Zero affinity mask or core ID not in affinity mask —
    /// `KernelError::InvalidCombination` (raw code `0x274`).
    #[error("Invalid combination")]
    InvalidCombination,

    /// Thread termination was requested during the operation —
    /// `KernelError::TerminationRequested` (raw code `0x23B`).
    #[error("Termination requested")]
    TerminationRequested,

    /// Any unforeseen kernel error. Contains the original [`Error`] so callers
    /// can inspect the raw result (`Error::to_raw`).
    #[error("Unknown error: {0}")]
    Unknown(Error),
}

impl ToRawResultCode for SetCoreMaskError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::InvalidHandle => KError::InvalidHandle.to_rc(),
            Self::InvalidCoreId => KError::InvalidCoreId.to_rc(),
            Self::InvalidCombination => KError::InvalidCombination.to_rc(),
            Self::TerminationRequested => KError::TerminationRequested.to_rc(),
            Self::Unknown(err) => err.to_raw(),
        }
    }
}

mod _priv {
    /// Sealed trait to prevent external implementations.
    pub trait Sealed {}
}
