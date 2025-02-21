//! # Mutex
//!
//! Mutex synchronization primitive
//!
//! This module provides a mutex implementation that uses the Nintendo Switch kernel's
//! synchronization primitives. A mutex is used to protect shared data from being
//! simultaneously accessed by multiple threads.
//!
//! The implementation is FFI-compatible with libnx's mutex implementation, allowing
//! it to be used seamlessly with C code. The mutex is represented as a 32-bit value
//! in memory that contains both the owner's thread handle and a waiters flag.
//!
//! When locked, only the thread that acquired the lock can unlock it. If other threads
//! attempt to lock an already locked mutex, they will be suspended by the kernel until
//! the mutex becomes available.

use core::{
    ptr,
    sync::atomic::{AtomicU32, Ordering},
};

use nx_svc::{
    debug::break_event,
    raw::{BreakReason, INVALID_HANDLE},
    sync::{HANDLE_WAIT_MASK, arbitrate_lock, arbitrate_unlock},
};
use nx_thread::raw::Handle;
use static_assertions::const_assert_eq;

/// A mutual exclusion primitive useful for protecting shared data
///
/// A mutex is a synchronization primitive that can be used to protect shared data from being
/// simultaneously accessed by multiple threads.
// NOTE: The in-memory representation of the Mutex must be u32 for FFI compatibility
#[repr(C)]
pub struct Mutex(AtomicU32);

// Ensure the in-memory size of the Mutex is the same as u32
const_assert_eq!(size_of::<Mutex>(), size_of::<u32>());

impl Mutex {
    /// Creates a new [`Mutex`].
    ///
    /// The mutex is initialized in an unlocked state, ready to be locked by any thread.
    pub const fn new() -> Self {
        Self(AtomicU32::new(INVALID_HANDLE))
    }

    /// Returns a raw pointer to the underlying atomic integer.
    ///
    /// # Safety
    ///
    /// This function is intended for FFI purposes and should be used with care.
    /// The caller must ensure that:
    /// - The pointer is not used after the mutex is dropped
    /// - The pointer is only used with Nintendo Switch kernel synchronization primitives
    /// - The pointer is properly aligned and valid for the lifetime of the mutex
    pub fn as_ptr(&self) -> *mut u32 {
        self.0.as_ptr()
    }

    /// Locks the mutex, blocking the current thread until the lock can be acquired.
    ///
    /// This function will block the current thread until it is able to acquire the mutex.
    /// When the function returns, the current thread will be the only thread with the
    /// mutex locked.
    ///
    /// # Panics
    ///
    /// Panics if the kernel's lock arbitration fails. This should never happen under
    /// normal circumstances.
    pub fn lock(&self) {
        let curr_thread_handle = get_curr_thread_handle();
        let mut curr_state = MutexState::from_raw(self.0.load(Ordering::Acquire));

        loop {
            match curr_state {
                MutexState::Unlocked => {
                    // Attempt to acquire the lock
                    match self.0.compare_exchange(
                        curr_state.into_raw(),
                        MutexState::Locked(MutexTag(curr_thread_handle)).into_raw(),
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return, // Lock acquired successfully
                        Err(new_value) => {
                            // Another thread modified the mutex; retry with the new value
                            curr_state = MutexState::from_raw(new_value);
                            continue;
                        }
                    }
                }
                MutexState::Locked(mut tag) => {
                    // If there are no waiters, set the waiters bitflag and proceed to arbitration
                    if !tag.has_waiters() {
                        tag.set_waiters_bitflag();

                        if let Err(new_value) = self.0.compare_exchange(
                            curr_state.into_raw(),
                            MutexState::Locked(tag).into_raw(),
                            Ordering::Acquire,
                            Ordering::Relaxed,
                        ) {
                            // Another thread modified the mutex; retry with the new value
                            curr_state = MutexState::from_raw(new_value);
                            continue;
                        }
                    }

                    // Ask the kernel to arbitrate the mutex locking
                    // This will pause the current thread until the mutex is unlocked
                    let arb_result = unsafe {
                        arbitrate_lock(tag.get_owner_handle(), self.0.as_ptr(), curr_thread_handle)
                    };
                    if arb_result.is_err() {
                        // This should never happen
                        // TODO: Handle the arbitrate_lock errors
                        let _ = unsafe { break_event(BreakReason::Assert, ptr::null_mut(), 0) };
                    }

                    // The arbitration has completed; check if we acquired the lock
                    curr_state = MutexState::from_raw(self.0.load(Ordering::Acquire));
                    if matches!(curr_state, MutexState::Locked(tag) if tag.get_owner_handle() == curr_thread_handle)
                    {
                        return;
                    }

                    continue;
                }
            }
        }
    }

    /// Attempts to lock the mutex without blocking.
    ///
    /// If the mutex is already locked by another thread, this function returns
    /// immediately with `false`. If the mutex is unlocked, it will be locked and
    /// this function will return `true`.
    ///
    /// This function is useful when you want to attempt to acquire the lock but
    /// don't want to block if it's not immediately available.
    ///
    /// # Returns
    ///
    /// * `true` if the mutex was successfully locked
    /// * `false` if the mutex was already locked by another thread
    pub fn try_lock(&self) -> bool {
        let curr_thread_handle = get_curr_thread_handle();

        // Attempt to acquire the lock by setting it from Unlocked to Locked with the current thread's handle
        // This will fail if the mutex is already locked
        self.0
            .compare_exchange(
                MutexState::Unlocked.into_raw(),
                MutexState::Locked(MutexTag(curr_thread_handle)).into_raw(),
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
    }

    /// Unlocks the [`Mutex`].
    ///
    /// This function will unlock the mutex, allowing other threads to lock it.
    /// If there are threads waiting on the mutex, one of them will be woken up
    /// and given the opportunity to acquire the lock.
    ///
    /// # Panics
    ///
    /// Panics if the kernel's unlock arbitration fails. This should never happen
    /// under normal circumstances.
    pub fn unlock(&self) {
        let curr_thread_handle = get_curr_thread_handle();
        let mut curr_state = MutexState::from_raw(self.0.load(Ordering::Acquire));

        loop {
            match curr_state {
                MutexState::Unlocked => return,
                MutexState::Locked(tag) => {
                    // If the mutex is not locked by the current thread, return
                    if tag.get_owner_handle() != curr_thread_handle {
                        return;
                    }

                    // If locked and there are waiters, ask the kernel to arbitrate the mutex unlocking
                    if tag.has_waiters() {
                        unsafe {
                            if arbitrate_unlock(self.0.as_ptr()).is_err() {
                                // This should never happen
                                // TODO: Handle the arbitrate_lock errors
                                let _ = break_event(BreakReason::Assert, ptr::null_mut(), 0);
                            }
                        }
                        return;
                    }

                    // Attempt to set the mutex state to Unlocked
                    match self.0.compare_exchange(
                        curr_state.into_raw(),
                        MutexState::Unlocked.into_raw(),
                        Ordering::Release,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return,
                        Err(current_value) => {
                            // If another thread modified the mutex, retry with the new value
                            curr_state = MutexState::from_raw(current_value);
                            continue;
                        }
                    }
                }
            }
        }
    }

    /// Checks if the mutex is locked by the current thread.
    pub fn is_locked_by_current_thread(&self) -> bool {
        let curr_thread_handle = get_curr_thread_handle();
        let curr_state = MutexState::from_raw(self.0.load(Ordering::Acquire));

        matches!(curr_state, MutexState::Locked(tag) if tag.get_owner_handle() == curr_thread_handle)
    }
}

impl Default for Mutex {
    /// Creates a new [`Mutex`].
    ///
    /// The mutex is initially unlocked.
    fn default() -> Self {
        Self::new()
    }
}

/// Internal representation of the [MutexTag].
type RawMutexTag = u32;

/// A value-to-raw mutex tag value conversion that consumes the inout value.
trait IntoRawTag {
    /// Converts this type into a raw mutex tag.
    fn into_raw(self) -> RawMutexTag;
}

/// Mutex state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MutexState {
    /// Unlocked mutex.
    Unlocked,
    /// Locked mutex.
    Locked(MutexTag),
}

impl MutexState {
    /// Convert a raw mutex tag value into a mutex state.
    fn from_raw(value: RawMutexTag) -> Self {
        if value == INVALID_HANDLE {
            Self::Unlocked
        } else {
            Self::Locked(MutexTag(value))
        }
    }
}

impl IntoRawTag for MutexState {
    /// Converts the [MutexState] into a raw mutex tag value.
    fn into_raw(self) -> RawMutexTag {
        match self {
            Self::Unlocked => INVALID_HANDLE,
            Self::Locked(MutexTag(tag)) => tag,
        }
    }
}

/// Mutex tag
///
/// The mutex tag holds two pieces of information:
///
/// - **The owner's thread kernel handle.**
///   When locked, the mutex tag is used to store the owner's thread kernel handle. And when
///   unlocked, it is reset to `INVALID_HANDLE`.
/// - **The _waiters_ bitflag.**
///   The _waiters bit_ is used to indicate to the kernel that there are other threads waiting for
///   the mutex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutexTag(RawMutexTag);

impl MutexTag {
    /// Get the mutex owner handle.
    ///
    /// Returns the mutex owner's thread kernel handle with the _waiters bitflag_ cleared.
    fn get_owner_handle(&self) -> Handle {
        self.0 & !HANDLE_WAIT_MASK
    }

    /// Check if there is any other thread waiting for the mutex.
    ///
    /// Indicate whether the mutex owner tag's _waiters bitflag_ is set.
    ///
    /// Returns `true` if the _waiters bitflag_ is set, `false` otherwise.
    fn has_waiters(&self) -> bool {
        self.0 & HANDLE_WAIT_MASK != 0
    }

    /// Set the mutex tag's _waiters bitflag_.
    ///
    /// This indicates the kernel that there are other threads waiting for the mutex.
    fn set_waiters_bitflag(&mut self) {
        self.0 |= HANDLE_WAIT_MASK;
    }
}

/// Get the current thread's kernel handle.
#[inline(always)]
fn get_curr_thread_handle() -> Handle {
    unsafe { nx_thread::raw::__nx_thread_get_current_thread_handle() }
}
