//! # Mutex

use core::{arch::asm, mem, ptr};

use nx_svc::{
    debug::break_event,
    raw::{BreakReason, INVALID_HANDLE},
    sync::{HANDLE_WAIT_MASK, arbitrate_lock, arbitrate_unlock},
};
use nx_thread::raw::Handle;
use static_assertions::const_assert_eq;

/// Mutex type.
///
/// A mutex is a synchronization primitive that can be used to protect shared data from being
/// simultaneously accessed by multiple threads.
// NOTE: The in-memory representation of the Mutex must be u32 for FFI compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Mutex(u32);

// Ensure the in-memory sisze of the Mutex is the same as u32
const_assert_eq!(size_of::<Mutex>(), size_of::<u32>());

impl Default for Mutex {
    /// Creates a new mutex.
    ///
    /// The mutex is initially unlocked.
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Add Loc, TryLock, etc. methods
impl Mutex {
    /// Creates a new mutex.
    ///
    /// The mutex is initially unlocked.
    pub const fn new() -> Self {
        Self(INVALID_HANDLE)
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

    /// Whether the mutex is locked.
    fn is_locked(&self) -> bool {
        matches!(self, Self::Locked(_))
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

/// Initializes the mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_init(mutex: *mut u32) {
    unsafe {
        *mutex = mem::transmute::<Mutex, u32>(Mutex::new());
    }
}

/// Locks the mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_lock(mutex: *mut u32) {
    // Get the current thread's handle (thread tag)
    let curr_thread_handle = get_curr_thread_handle();

    loop {
        let state = MutexState::from_raw(load_exclusive(mutex));

        match state {
            MutexState::Unlocked => {
                // Try to acquire the mutex by storing the current thread's tag
                match store_exclusive(mutex, MutexState::Locked(MutexTag(curr_thread_handle))) {
                    Ok(_) => break,
                    Err(_) => {
                        continue; // If failed, try again
                    }
                }
            }
            MutexState::Locked(mut tag) => {
                // If the mutex doesn't have any waiters, try to register ourselves as the first
                // waiter.
                //
                // If the waiters bit in the mutex is not set, set it so we are the first
                // waiter
                //
                // By setting the waiters bit, we are telling the kernel that there are other
                // threads waiting for the mutex. This will be used by the kernel to arbitrate the
                // lock for us.
                if !tag.has_waiters() {
                    tag.set_waiters_bitflag();
                    if store_exclusive(mutex, MutexState::Locked(tag)).is_err() {
                        continue; // Try again on failure
                    }
                }

                // Ask the kernel to arbitrate the lock for us
                // - Extracts the mutex owner thread's handle (removing the waiter bit)
                // - Tell the kernel to put the current thread to sleep until the mutex is unlocked
                //
                // Internally, arbitrate_lock, tells the kernel to:
                // - Check if the mutex is still locked by checking if the mutex's value is equal to
                //   the owner's handle with the waiter bit set. If it is, the kernel will put the
                //   current thread to sleep.
                unsafe {
                    if arbitrate_lock(tag.get_owner_handle(), mutex, curr_thread_handle).is_err() {
                        // This should never happen
                        let _ = break_event(BreakReason::Assert, ptr::null_mut(), 0);
                    }
                }

                // Reload the mutex tag, and check if we acquired the lock
                let state = MutexState::from_raw(load_exclusive(mutex));
                if matches!(state, MutexState::Locked(owner_tag) if owner_tag.get_owner_handle() == curr_thread_handle)
                {
                    clear_exclusive();
                    break;
                }
            }
        }
    }
}

/// Attempts to lock the mutex without waiting.
///
/// Returns `true` if the mutex was successfully locked, `false` otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_try_lock(mutex: *mut u32) -> bool {
    let curr_thread_handle = get_curr_thread_handle();

    let state = MutexState::from_raw(load_exclusive(mutex));
    if !state.is_locked() {
        // Try to acquire the mutex by storing the current thread's handle
        return store_exclusive(mutex, MutexState::Locked(MutexTag(curr_thread_handle))).is_ok();
    }

    clear_exclusive();
    false
}

/// Unlocks the mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_unlock(mutex: *mut u32) {
    let curr_thread_handle = get_curr_thread_handle();

    let mut state = MutexState::from_raw(load_exclusive(mutex));

    // If the mutex is already unlocked, return
    if !state.is_locked() {
        clear_exclusive();
        return;
    }

    // Try to release the lock
    loop {
        match state {
            MutexState::Unlocked => {
                clear_exclusive();
                return;
            }
            MutexState::Locked(tag) => {
                // If we have any listeners, we need to ask the kernel to arbitrate
                if tag.get_owner_handle() != curr_thread_handle || tag.has_waiters() {
                    clear_exclusive();
                    break;
                }

                // Try to release the lock, if failed, reload the mutex state and try again
                if store_exclusive(mutex, MutexState::Unlocked).is_err() {
                    state = MutexState::from_raw(load_exclusive(mutex));
                    continue;
                } else {
                    break;
                }
            }
        }
    }

    // If locked and there are waiters, ask the kernel to arbitrate the mutex unlocking
    if matches!(state, MutexState::Locked(tag) if tag.has_waiters()) {
        unsafe {
            if arbitrate_unlock(mutex).is_err() {
                // This should never happen
                let _ = break_event(BreakReason::Assert, ptr::null_mut(), 0);
            }
        }
    }
}

/// Get the current thread's kernel handle.
#[inline(always)]
fn get_curr_thread_handle() -> Handle {
    unsafe { nx_thread::raw::__nx_thread_get_current_thread_handle() }
}

/// Gets whether the mutex is locked by the current thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_is_locked_by_current_thread(mutex: *mut u32) -> bool {
    // Get the current thread's handle (thread tag)
    let curr_thread_handle = get_curr_thread_handle();
    let state = unsafe { MutexState::from_raw(*mutex) }; // TODO: Review the safety of this dereference

    matches!(state, MutexState::Locked(tag) if tag.get_owner_handle() == curr_thread_handle)
}

/// Load-Exclusive (LDAXR) 32-bit value from the given pointer
///
/// ## References
/// - [ARM aarch64: LDAXR](https://developer.arm.com/documentation/ddi0602/2024-12/Base-Instructions/LDAXR--Load-acquire-exclusive-register-?lang=en)
#[inline(always)]
fn load_exclusive(ptr: *const u32) -> u32 {
    let value: u32;
    unsafe {
        asm!(
            "ldaxr {val:w}, [{ptr:x}]", // Loads the 32-bit value from the memory location pointed to by ptr
            ptr = in(reg) ptr,          // Input: ptr to load from
            val = out(reg) value,       // Output: Capture thr result in value (via a register)
            options(nostack, preserves_flags)
        );
    }
    value
}

/// Store-Exclusive (STLXR) 32-bit value to the given pointer
///
/// ## References
/// - [ARM aarch64: STLXR](https://developer.arm.com/documentation/ddi0602/2024-12/Base-Instructions/STLXR--Store-release-exclusive-register-?lang=en)
#[inline(always)]
fn store_exclusive(ptr: *mut u32, val: impl IntoRawTag) -> Result<(), ()> {
    let mut res: u32;
    unsafe {
        asm!(
            "stlxr {res:w}, {val:w}, [{ptr:x}]", // Stores the 32-bit value to the memory location pointed to by ptr
            val = in(reg) val.into_raw(),        // Input: Value to store
            ptr = in(reg) ptr,                   // Input: ptr to store to
            res = out(reg) res,                  // Output: Capture the result in res (via a register)
            options(nostack, preserves_flags)
        );
    }

    // If `res` is `0`, the operation updated memory, otherwise it failed
    if res == 0 { Ok(()) } else { Err(()) }
}

/// Clears the exclusive reservation using the `clrex` assembly instruction.
///
/// ## References
/// - [ARM aarch64: CLREX](https://developer.arm.com/documentation/ddi0602/2024-12/Base-Instructions/CLREX--Clear-exclusive-?lang=en)
#[inline(always)]
fn clear_exclusive() {
    unsafe {
        asm!("clrex", options(nostack, preserves_flags));
    }
}
