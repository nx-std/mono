//! FFI bindings for the `nx-sys-sync` crate - RwLock
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/rwlock.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/rwlock.h)

use crate::sys::switch::RwLock;

/// Initializes a read/write lock at the given memory location.
///
/// # Safety
///
/// - `rw` must point to a valid, properly aligned memory location for a `RwLock`
/// - The memory at `rw` must be writeable
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_init(rw: *mut RwLock) {
    unsafe { rw.write(RwLock::new()) };
}

/// Locks the read/write lock for reading.
///
/// Multiple threads can acquire the read lock simultaneously as long as there is no writer.
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_read_lock(rw: *mut RwLock) {
    unsafe { &*rw }.read_lock()
}

/// Attempts to lock the read/write lock for reading without waiting.
///
/// # Returns
///
/// * `true` if the lock was acquired successfully
/// * `false` if there was contention
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_try_read_lock(rw: *mut RwLock) -> bool {
    unsafe { &*rw }.try_read_lock()
}

/// Unlocks the read/write lock for reading.
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The current thread must hold a read lock on the `RwLock`
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_read_unlock(rw: *mut RwLock) {
    unsafe { &*rw }.read_unlock()
}

/// Locks the read/write lock for writing.
///
/// Only one thread can acquire the write lock at a time, and no readers can acquire
/// the lock while a writer holds it.
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_write_lock(rw: *mut RwLock) {
    unsafe { &*rw }.write_lock()
}

/// Attempts to lock the read/write lock for writing without waiting.
///
/// # Returns
///
/// * `true` if the lock was acquired successfully
/// * `false` if there was contention
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_try_write_lock(rw: *mut RwLock) -> bool {
    unsafe { &*rw }.try_write_lock()
}

/// Unlocks the read/write lock for writing.
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The current thread must hold the write lock
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_write_unlock(rw: *mut RwLock) {
    unsafe { &*rw }.write_unlock()
}

/// Checks if the write lock is held by the current thread.
///
/// # Returns
///
/// * `true` if the current thread holds the write lock
/// * `false` if it does not hold the write lock or only holds read locks
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_is_write_lock_held_by_current_thread(
    rw: *mut RwLock,
) -> bool {
    unsafe { &*rw }.is_write_lock_held_by_current_thread()
}

/// Checks if the read/write lock is owned by the current thread.
///
/// # Returns
///
/// * `true` if the current thread holds the write lock or if it holds read locks
///   acquired while it held the write lock
/// * `false` if it does not
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_sync_rwlock_is_owned_by_current_thread(rw: *mut RwLock) -> bool {
    unsafe { &*rw }.is_owned_by_current_thread()
}
