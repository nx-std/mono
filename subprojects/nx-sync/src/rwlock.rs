//! # Read-Write Lock
//!
//! A read/write lock synchronization primitive that allows multiple readers or a single writer.

use nx_svc::raw::{Handle, INVALID_HANDLE};
use static_assertions::const_assert_eq;

use crate::{condvar::Condvar, mutex::Mutex};

/// Read/write lock structure that allows multiple readers or a single writer.
#[repr(C)]
pub struct RwLock {
    mutex: Mutex,
    condvar_reader_wait: Condvar,
    condvar_writer_wait: Condvar,
    read_lock_count: u32,
    read_waiter_count: u32,
    write_lock_count: u32,
    write_waiter_count: u32,
    write_owner_tag: WriteOwnerTag,
}

// Ensure the struct is the same size as the C struct and has the same layout
const_assert_eq!(size_of::<RwLock>(), 32);
const_assert_eq!(align_of::<RwLock>(), align_of::<u32>());

impl RwLock {
    /// Creates a new [`RwLock`] in an unlocked state.
    ///
    /// The lock is initialized with no readers or writers, and can be immediately used
    /// for synchronization.
    pub const fn new() -> Self {
        Self {
            mutex: Mutex::new(),
            condvar_reader_wait: Condvar::new(),
            condvar_writer_wait: Condvar::new(),
            read_lock_count: 0,
            read_waiter_count: 0,
            write_lock_count: 0,
            write_waiter_count: 0,
            write_owner_tag: WriteOwnerTag::new(),
        }
    }

    /// Gets a raw pointer to this [`RwLock`].
    ///
    /// This is primarily used for FFI purposes and should be used with caution.
    pub fn as_ptr(&self) -> *mut Self {
        self as *const _ as *mut Self
    }

    /// Locks the [`RwLock`] for reading.
    ///
    /// Multiple threads can acquire the read lock simultaneously as long as there is no writer.
    /// If the current thread already holds the write lock, it can also acquire read locks
    /// without blocking.
    ///
    /// This call will block if:
    /// - Another thread holds the write lock
    /// - There are waiting writers (to prevent writer starvation)
    pub fn read_lock(&self) {
        unsafe { __nx_sync_rwlock_read_lock(self.as_ptr()) }
    }

    /// Attempts to lock the [`RwLock`] for reading without waiting.
    ///
    /// This method will never block. If the lock cannot be acquired immediately,
    /// it returns false.
    ///
    /// # Returns
    ///
    /// * `true` if the lock was acquired successfully:
    ///   - No other thread holds the write lock
    ///   - No writers are waiting
    ///   - The current thread holds the write lock
    /// * `false` if there was contention
    pub fn try_read_lock(&self) -> bool {
        unsafe { __nx_sync_rwlock_try_read_lock(self.as_ptr()) }
    }

    /// Unlocks the [`RwLock`] for reading.
    ///
    /// This method must only be called by a thread that currently holds a read lock.
    /// If this is the last read lock and there are waiting writers, one of them will
    /// be woken up.
    pub fn read_unlock(&self) {
        unsafe { __nx_sync_rwlock_read_unlock(self.as_ptr()) }
    }

    /// Locks the [`RwLock`] for writing.
    ///
    /// Only one thread can acquire the write lock at a time, and no readers can acquire
    /// the lock while a writer holds it. If the current thread already holds the write lock,
    /// the write count is incremented without blocking.
    ///
    /// This call will block if:
    /// - Other threads hold read locks
    /// - Another thread holds the write lock
    pub fn write_lock(&self) {
        unsafe { __nx_sync_rwlock_write_lock(self.as_ptr()) }
    }

    /// Attempts to lock the [`RwLock`] for writing without waiting.
    ///
    /// This method will never block. If the lock cannot be acquired immediately,
    /// it returns `false`.
    ///
    /// # Returns
    ///
    /// * `true` if the lock was acquired successfully:
    ///   - No other thread holds read locks
    ///   - No other thread holds the write lock
    ///   - The current thread already holds the write lock
    /// * `false` if there was contention
    pub fn try_write_lock(&self) -> bool {
        unsafe { __nx_sync_rwlock_try_write_lock(self.as_ptr()) }
    }

    /// Unlocks the [`RwLock`] for writing.
    ///
    /// This method must only be called by a thread that currently holds the write lock.
    /// When the last write lock is released, waiting writers are given priority over
    /// waiting readers to prevent writer starvation.
    pub fn write_unlock(&self) {
        unsafe { __nx_sync_rwlock_write_unlock(self.as_ptr()) }
    }

    /// Checks if the write lock is held by the current thread.
    ///
    /// # Returns
    ///
    /// * `true` if the current thread holds the write lock
    /// * `false` if it does not hold the write lock or only holds read locks
    pub fn is_write_lock_held_by_current_thread(&self) -> bool {
        unsafe { __nx_sync_rwlock_is_write_lock_held_by_current_thread(self.as_ptr()) }
    }

    /// Checks if the [`RwLock`] is owned by the current thread.
    ///
    /// A thread owns the lock if it holds the write lock or if it holds read locks
    /// that were acquired while it held the write lock.
    ///
    /// # Returns
    ///
    /// * `true` if the current thread holds the write lock or if it holds read locks
    ///   acquired while it held the write lock
    /// * `false` if it does not own the lock
    pub fn is_owned_by_current_thread(&self) -> bool {
        unsafe { __nx_sync_rwlock_is_owned_by_current_thread(self.as_ptr()) }
    }
}

impl Default for RwLock {
    fn default() -> Self {
        Self::new()
    }
}

/// Tag used to identify the owner of the write lock.
#[repr(transparent)]
struct WriteOwnerTag(u32);

impl WriteOwnerTag {
    /// Creates a new [`WriteOwnerTag`] not associated with any handle.
    const fn new() -> Self {
        Self(INVALID_HANDLE)
    }

    fn set(&mut self, handle: Handle) {
        self.0 = handle;
    }

    fn clear(&mut self) {
        self.0 = INVALID_HANDLE;
    }
}

impl PartialEq<Handle> for WriteOwnerTag {
    fn eq(&self, other: &Handle) -> bool {
        self.0 == *other
    }
}

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
    let rw = unsafe { &mut *rw };
    let curr_thread_handle = get_curr_thread_handle();

    // If the current thread already holds the write lock, increment the read count
    // without blocking.
    if rw.write_owner_tag == curr_thread_handle {
        rw.read_lock_count += 1;
        return;
    }

    // Lock the mutex to prevent concurrent modifications.
    rw.mutex.lock();

    // If there are waiting writers, increment the reader waiter count and wait for
    // the writer to finish.
    while rw.write_waiter_count > 0 {
        rw.read_waiter_count += 1;
        rw.condvar_reader_wait.wait(&rw.mutex);
        rw.read_waiter_count -= 1;
    }

    // Increment the read count.
    rw.read_lock_count += 1;

    // Unlock the mutex to allow other threads to acquire the lock
    rw.mutex.unlock();
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
    let rw = unsafe { &mut *rw };
    let curr_thread_handle = get_curr_thread_handle();

    // If the current thread already holds the write lock, increment the read count
    // without blocking.
    if rw.write_owner_tag == curr_thread_handle {
        rw.read_lock_count += 1;
        return true;
    }

    // Try to lock the mutex
    if !rw.mutex.try_lock() {
        return false;
    }

    // If there are no waiting writers, increment the read count
    let got_lock = rw.write_waiter_count == 0;
    if got_lock {
        rw.read_lock_count += 1;
    }

    // Unlock the mutex to allow other threads to acquire the lock
    rw.mutex.unlock();

    got_lock
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
    let rw = unsafe { &mut *rw };
    let curr_thread_handle = get_curr_thread_handle();

    // If the current thread does not hold the write lock, decrement the read count
    // and wake up a writer if there are any.
    if rw.write_owner_tag != curr_thread_handle {
        rw.mutex.lock();

        // Decrement the read count.
        rw.read_lock_count -= 1;

        // If there are no more readers and there are waiting writers, wake up one writer
        if rw.read_lock_count == 0 && rw.write_waiter_count > 0 {
            rw.condvar_writer_wait.wake_one();
        }

        rw.mutex.unlock();
    } else {
        // If the current thread holds the write lock, decrement the read count without blocking
        rw.read_lock_count -= 1;

        // If there are no more readers and there are waiting writers, wake up one writer
        if rw.read_lock_count == 0 && rw.write_lock_count == 0 {
            rw.write_owner_tag.clear();

            // Wake up a waiting writer if there are any,
            // otherwise wake up all waiting readers
            if rw.write_waiter_count > 0 {
                rw.condvar_writer_wait.wake_one();
            } else if rw.read_waiter_count > 0 {
                rw.condvar_reader_wait.wake_all();
            }

            rw.mutex.unlock();
        }
    }
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
    let rw = unsafe { &mut *rw };
    let curr_thread_handle = get_curr_thread_handle();

    // If the current thread already holds the write lock, increment the write count
    // without blocking.
    if rw.write_owner_tag == curr_thread_handle {
        rw.write_lock_count += 1;
        return;
    }

    rw.mutex.lock();

    // If there are waiting readers, increment the writer waiter count and wait for
    // the readers to finish.
    while rw.read_lock_count > 0 {
        rw.write_waiter_count += 1;
        rw.condvar_writer_wait.wait(&rw.mutex);
        rw.write_waiter_count -= 1;
    }

    // Increment the write count, and set the write owner tag to the current thread
    rw.write_lock_count = 1;
    rw.write_owner_tag.set(curr_thread_handle);

    // NOTE: The mutex is intentionally not unlocked here.
    //       It will be unlocked by a call to read_unlock or write_unlock.
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
    let rw = unsafe { &mut *rw };
    let curr_thread_handle = get_curr_thread_handle();

    // If the current thread already holds the write lock, increment the write count
    // without blocking.
    if rw.write_owner_tag == curr_thread_handle {
        rw.write_lock_count += 1;
        return true;
    }

    if !rw.mutex.try_lock() {
        return false;
    }

    // If there are waiting readers, return false
    if rw.read_lock_count > 0 {
        rw.mutex.unlock();
        return false;
    }

    // Set the write count to 1, and set the write ownWriteUnlocker tag to the current thread
    rw.write_lock_count = 1;
    rw.write_owner_tag.set(curr_thread_handle);

    // NOTE: The mutex is intentionally not unlocked here.
    //       It will be unlocked by a call to read_unlock or write_unlock.

    true
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
    let rw = unsafe { &mut *rw };

    // NOTE: This function assumes the write lock is held.
    //       This means that the mutex is locked, and the write owner tag is set
    //       to the current thread (write_owner_tag == curr_thread_handle).

    rw.write_lock_count -= 1;

    // If there are no more writers and no readers, unlock the mutex and wake up
    // a waiting writer or all waiting readers.
    if rw.write_lock_count == 0 && rw.read_lock_count == 0 {
        rw.write_owner_tag.clear();

        // Wake up a waiting writer if there are any, otherwise wake up all waiting readers
        if rw.write_waiter_count > 0 {
            rw.condvar_writer_wait.wake_one();
        } else if rw.read_waiter_count > 0 {
            rw.condvar_reader_wait.wake_all();
        }

        rw.mutex.unlock();
    }
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
    unsafe { (*rw).write_owner_tag == get_curr_thread_handle() && (*rw).write_lock_count > 0 }
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
    unsafe { (*rw).write_owner_tag == get_curr_thread_handle() }
}

/// Get the current thread's kernel handle
#[inline(always)]
fn get_curr_thread_handle() -> Handle {
    unsafe { nx_thread::raw::__nx_thread_get_current_thread_handle() }
}
