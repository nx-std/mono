//! FFI bindings for the `nx-sync` crate
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/mutex.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/mutex.h)
//! - [switchbrew/libnx: switch/kernel/condvar.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/condvar.h)
//! - [switchbrew/libnx: switch/kernel/rwlock.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/rwlock.h)
//! - [switchbrew/libnx: switch/kernel/barrier.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/barrier.h)
//! - [switchbrew/libnx: switch/kernel/semaphore.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/semaphore.h)

use nx_svc::result::ResultCode;

use crate::sys::switch::{Barrier, Condvar, Mutex, RwLock, Semaphore};

//<editor-fold desc="switch/kernel/mutex.h">

/// Initializes the mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Writes to the memory pointed to by `mutex`
/// - Requires that `mutex` is valid and properly aligned
/// - Requires that `mutex` points to memory that can be safely written to
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_init(mutex: *mut Mutex) {
    unsafe { mutex.write(Mutex::new()) }
}

/// Locks the mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_lock(mutex: *mut Mutex) {
    let mutex = unsafe { &*mutex };
    mutex.lock()
}

/// Attempts to lock the mutex without waiting.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
///
/// # Returns
///
/// Returns `true` if the mutex was successfully locked, `false` if it was already locked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_try_lock(mutex: *mut Mutex) -> bool {
    let mutex = unsafe { &*mutex };
    mutex.try_lock()
}

/// Unlocks the mutex.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_unlock(mutex: *mut Mutex) {
    let mutex = unsafe { &*mutex };
    mutex.unlock()
}

/// Checks if the mutex is locked by the current thread.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Requires that `mutex` points to a valid Mutex instance
/// - Requires that `mutex` is properly aligned
///
/// # Returns
///
/// Returns `true` if the mutex is currently locked by the calling thread,
/// `false` otherwise.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_mutex_is_locked_by_current_thread(mutex: *mut Mutex) -> bool {
    let mutex = unsafe { &*mutex };
    mutex.is_locked_by_current_thread()
}

//</editor-fold>

//<editor-fold desc="switch/kernel/condvar.h">

/// Initializes a condition variable.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to valid memory that can hold a `Condvar`
/// * The memory pointed to by `condvar` remains valid for the entire lifetime of the condition variable
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_init(condvar: *mut Condvar) {
    unsafe { condvar.write(Condvar::new()) };
}

/// Waits on a condition variable with a timeout
///
/// This function atomically releases the mutex and waits on the condition variable.
/// When the function returns, regardless of the reason, the mutex is guaranteed to be
/// re-acquired.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
/// * `mutex` points to a valid initialized mutex
/// * The current thread owns the mutex
///
/// # Parameters
///
/// * `condvar`: Pointer to the condition variable to wait on
/// * `mutex`: Pointer to the mutex protecting the condition
/// * `timeout`: Maximum time to wait in nanoseconds
///
/// # Returns
///
/// * `0` on successful wait and wake
/// * `0xEA01` if the wait timed out
/// * Other values indicate an error
///
/// # Notes
///
/// On function return, the underlying mutex is guaranteed to be acquired, even in case
/// of timeout or error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wait_timeout(
    condvar: *mut Condvar,
    mutex: *mut Mutex,
    timeout: u64,
) -> ResultCode {
    let condvar = unsafe { &*condvar };
    let mutex = unsafe { &*mutex };
    condvar.wait_timeout(mutex, timeout)
}

/// Waits on a condition variable indefinitely
///
/// This function atomically releases the mutex and waits on the condition variable
/// with no timeout. When the function returns, the mutex is guaranteed to be re-acquired.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
/// * `mutex` points to a valid initialized mutex
/// * The current thread owns the mutex
///
/// # Parameters
///
/// * `condvar`: Pointer to the condition variable to wait on
/// * `mutex`: Pointer to the mutex protecting the condition
///
/// # Returns
///
/// * `0` on successful wait and wake
/// * Non-zero value indicates an error
///
/// # Notes
///
/// On function return, the underlying mutex is guaranteed to be acquired.
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wait(
    condvar: *mut Condvar,
    mutex: *mut Mutex,
) -> ResultCode {
    let condvar = unsafe { &*condvar };
    let mutex = unsafe { &*mutex };
    condvar.wait_timeout(mutex, u64::MAX)
}

/// Wakes up a specified number of threads waiting on a condition variable.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
///
/// # Parameters
///
/// * `condvar`: Pointer to the condition variable
/// * `num`: Maximum number of threads to wake up
///   * If positive, wake up to that many threads
///   * If <= 0, e.g., -1, wake up all waiting threads
///
/// # Returns
///
/// * `0` on success
/// * Non-zero value indicates an error
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wake(condvar: *mut Condvar, num: i32) -> ResultCode {
    let condvar = unsafe { &*condvar };
    condvar.wake(num);
    0
}

/// Wakes up a single thread waiting on a condition variable
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
///
/// # Returns
///
/// * `0` on success
/// * Non-zero value indicates an error
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wake_one(condvar: *mut Condvar) -> ResultCode {
    let condvar = unsafe { &*condvar };
    condvar.wake_one();
    0
}

/// Wakes up all threads waiting on a condition variable.
///
/// # Safety
///
/// The caller must ensure that:
/// * `condvar` points to a valid initialized condition variable
///
/// # Returns
///
/// * `0` on success
/// * Non-zero value indicates an error
#[inline]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_condvar_wake_all(condvar: *mut Condvar) -> ResultCode {
    let condvar = unsafe { &*condvar };
    condvar.wake_all();
    0
}

//</editor-fold>

//<editor-fold desc="switch/kernel/rwlock.h">

/// Initializes a read/write lock at the given memory location.
///
/// # Safety
///
/// - `rw` must point to a valid, properly aligned memory location for a `RwLock`
/// - The memory at `rw` must be writeable
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_rwlock_init(rw: *mut RwLock) {
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
pub unsafe extern "C" fn __nx_sync_rwlock_read_lock(rw: *mut RwLock) {
    let rw = unsafe { &*rw };
    rw.read_lock()
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
pub unsafe extern "C" fn __nx_sync_rwlock_try_read_lock(rw: *mut RwLock) -> bool {
    let rw = unsafe { &*rw };
    rw.try_read_lock()
}

/// Unlocks the read/write lock for reading.
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The current thread must hold a read lock on the `RwLock`
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_rwlock_read_unlock(rw: *mut RwLock) {
    let rw = unsafe { &*rw };
    rw.read_unlock()
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
pub unsafe extern "C" fn __nx_sync_rwlock_write_lock(rw: *mut RwLock) {
    let rw = unsafe { &*rw };
    rw.write_lock()
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
pub unsafe extern "C" fn __nx_sync_rwlock_try_write_lock(rw: *mut RwLock) -> bool {
    let rw = unsafe { &*rw };
    rw.try_write_lock()
}

/// Unlocks the read/write lock for writing.
///
/// # Safety
///
/// - `rw` must point to a valid, initialized `RwLock`
/// - The current thread must hold the write lock
/// - The `RwLock` must not be concurrently modified except through its synchronized methods
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_rwlock_write_unlock(rw: *mut RwLock) {
    let rw = unsafe { &*rw };
    rw.write_unlock()
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
pub unsafe extern "C" fn __nx_sync_rwlock_is_write_lock_held_by_current_thread(
    rw: *mut RwLock,
) -> bool {
    let rw = unsafe { &*rw };
    rw.is_write_lock_held_by_current_thread()
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
pub unsafe extern "C" fn __nx_sync_rwlock_is_owned_by_current_thread(rw: *mut RwLock) -> bool {
    let rw = unsafe { &*rw };
    rw.is_owned_by_current_thread()
}

//</editor-fold>

//<editor-fold desc="switch/kernel/barrier.h">

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
    let bar = unsafe { &*bar };
    bar.wait();
}

//</editor-fold>

//<editor-fold desc="switch/kernel/semaphore.h">

/// Initializes a semaphore with an initial counter value.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object to initialize
/// * `count` - Initial value for the semaphore's counter. It must be >= 1.
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to valid memory that is properly aligned for a Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_semaphore_init(sem: *mut Semaphore, count: u64) {
    unsafe { sem.write(Semaphore::new(count)) };
}

/// Increments the semaphore's counter and wakes one waiting thread.
///
/// This function is used when a thread is done with a resource, making it
/// available for other threads.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to a valid, initialized Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_semaphore_signal(sem: *mut Semaphore) {
    let sem = unsafe { &*sem };
    sem.signal();
}

/// Decrements the semaphore's counter, blocking if no resources are available.
///
/// If the counter is 0, the calling thread will block until another thread
/// signals the semaphore.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to a valid, initialized Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_semaphore_wait(sem: *mut Semaphore) {
    let sem = unsafe { &*sem };
    sem.wait();
}

/// Attempts to decrement the semaphore's counter without blocking.
///
/// # Arguments
/// * `sem` - Pointer to the semaphore object
///
/// # Returns
/// * `true` if the counter was successfully decremented
/// * `false` if the counter was 0 (no resources available)
///
/// # Safety
/// The caller must ensure that:
/// * `sem` points to a valid, initialized Semaphore object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_semaphore_try_wait(sem: *mut Semaphore) -> bool {
    let sem = unsafe { &*sem };
    sem.try_wait()
}

//</editor-fold>
