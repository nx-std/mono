//! FFI bindings for libsysbase syscalls (newlib integration).
//!
//! Provides implementations for newlib's synchronization syscalls, allowing
//! pthread and other POSIX synchronization primitives to use nx-sys-sync.
//!
//! # References
//!
//! - libgloss/libsysbase/syscall_support.c
//! - libgloss/libsysbase/pthread.c
//! - newlib/libc/machine/aarch64/sys/lock.h

use core::ffi::c_int;

use nx_svc::error::{KernelError, ToRawResultCode as _};

use crate::{condvar::Condvar, mutex::Mutex, remutex::ReentrantMutex};

/// POSIX error: Bad file descriptor (used when recursive lock counter != 1).
const EBADF: c_int = 9;

/// POSIX error: I/O error (generic error fallback).
const EIO: c_int = 5;

/// POSIX error: Connection timed out.
const ETIMEDOUT: c_int = 110;

/// Converts a kernel result code to an errno value.
fn errno_from_result(result: nx_svc::result::ResultCode) -> c_int {
    if result == 0 {
        return 0;
    }

    // Check for kernel TimedOut error
    if result == KernelError::TimedOut.to_rc() {
        return ETIMEDOUT;
    }

    EIO
}

/// Acquires a non-recursive lock.
///
/// Corresponds to libsysbase's `__syscall_lock_acquire`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_acquire(lock: *mut Mutex) {
    unsafe { &*lock }.lock()
}

/// Attempts to acquire a non-recursive lock without blocking.
///
/// Returns 0 on success, non-zero if the lock is already held.
/// Corresponds to libsysbase's `__syscall_lock_try_acquire`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_try_acquire(
    lock: *mut Mutex,
) -> c_int {
    if unsafe { &*lock }.try_lock() { 0 } else { 1 }
}

/// Releases a non-recursive lock.
///
/// Corresponds to libsysbase's `__syscall_lock_release`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_release(lock: *mut Mutex) {
    unsafe { &*lock }.unlock()
}

/// Initializes a non-recursive lock.
///
/// Corresponds to libsysbase's `__syscall_lock_init`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_init(lock: *mut Mutex) {
    unsafe { lock.write(Mutex::new()) }
}

/// Acquires a recursive lock.
///
/// Corresponds to libsysbase's `__syscall_lock_acquire_recursive`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_acquire_recursive(
    lock: *mut ReentrantMutex,
) {
    unsafe { &*lock }.lock()
}

/// Attempts to acquire a recursive lock without blocking.
///
/// Returns 0 on success, non-zero if the lock is already held by another thread.
/// Corresponds to libsysbase's `__syscall_lock_try_acquire_recursive`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_try_acquire_recursive(
    lock: *mut ReentrantMutex,
) -> c_int {
    if unsafe { &*lock }.try_lock() { 0 } else { 1 }
}

/// Releases a recursive lock.
///
/// Corresponds to libsysbase's `__syscall_lock_release_recursive`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_release_recursive(
    lock: *mut ReentrantMutex,
) {
    unsafe { &*lock }.unlock()
}

/// Initializes a recursive lock.
///
/// Corresponds to libsysbase's `__syscall_lock_init_recursive`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_lock_init_recursive(
    lock: *mut ReentrantMutex,
) {
    unsafe { lock.write(ReentrantMutex::new()) }
}

/// Signals one thread waiting on a condition variable.
///
/// Corresponds to libsysbase's `__syscall_cond_signal`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_cond_signal(
    cond: *mut Condvar,
) -> c_int {
    unsafe { &*cond }.wake_one();
    0
}

/// Broadcasts to all threads waiting on a condition variable.
///
/// Corresponds to libsysbase's `__syscall_cond_broadcast`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_cond_broadcast(
    cond: *mut Condvar,
) -> c_int {
    unsafe { &*cond }.wake_all();
    0
}

/// Waits on a condition variable with a non-recursive mutex.
///
/// Corresponds to libsysbase's `__syscall_cond_wait`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_cond_wait(
    cond: *mut Condvar,
    lock: *mut Mutex,
    timeout_ns: u64,
) -> c_int {
    let result = unsafe { &*cond }.wait_timeout(unsafe { &*lock }, timeout_ns);
    errno_from_result(result)
}

/// Waits on a condition variable with a recursive mutex.
///
/// The recursive mutex must be held exactly once (counter == 1).
/// Corresponds to libsysbase's `__syscall_cond_wait_recursive`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync__libsysbase_syscall_cond_wait_recursive(
    cond: *mut Condvar,
    lock: *mut ReentrantMutex,
    timeout_ns: u64,
) -> c_int {
    match unsafe { &*lock }.cond_wait(unsafe { &*cond }, timeout_ns) {
        Ok(result) => errno_from_result(result),
        Err(()) => EBADF,
    }
}
