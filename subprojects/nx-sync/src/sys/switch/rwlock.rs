//! # Read-Write Lock
//!
//! A read/write lock synchronization primitive that allows multiple readers or a single writer.

use core::cell::UnsafeCell;

use nx_svc::raw::{Handle, INVALID_HANDLE};
use nx_sys_thread::sys::thread_vars;
use static_assertions::const_assert_eq;

use super::{Condvar, Mutex};

/// Read/write lock structure that allows multiple readers or a single writer.
#[repr(C)]
pub struct RwLock {
    mutex: Mutex,
    condvar_reader_wait: Condvar,
    condvar_writer_wait: Condvar,
    read_lock_count: UnsafeCell<u32>,
    read_waiter_count: UnsafeCell<u32>,
    write_lock_count: UnsafeCell<u32>,
    write_waiter_count: UnsafeCell<u32>,
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
            read_lock_count: UnsafeCell::new(0),
            read_waiter_count: UnsafeCell::new(0),
            write_lock_count: UnsafeCell::new(0),
            write_waiter_count: UnsafeCell::new(0),
            write_owner_tag: WriteOwnerTag::new(),
        }
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
        let curr_thread_handle = get_curr_thread_handle();

        // If the current thread already holds the write lock, increment the read count
        // without blocking.
        let read_lock_count = unsafe { &mut *self.read_lock_count.get() };
        if self.write_owner_tag == curr_thread_handle {
            *read_lock_count += 1;
            return;
        }

        // Lock the mutex to prevent concurrent modifications.
        self.mutex.lock();

        // If there are waiting writers, increment the reader waiter count and wait for
        // the writer to finish.
        let write_waiter_count = unsafe { &*self.write_waiter_count.get() };
        let read_waiter_count = unsafe { &mut *self.read_waiter_count.get() };
        #[allow(clippy::while_immutable_condition)]
        while *write_waiter_count > 0 {
            *read_waiter_count += 1;
            self.condvar_reader_wait.wait(&self.mutex);
            *read_waiter_count -= 1;
        }

        // Increment the read count.
        *read_lock_count += 1;

        // Unlock the mutex to allow other threads to acquire the lock
        self.mutex.unlock();
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
        let curr_thread_handle = get_curr_thread_handle();

        // If the current thread already holds the write lock, increment the read count
        // without blocking.
        let read_lock_count = unsafe { &mut *self.read_lock_count.get() };
        if self.write_owner_tag == curr_thread_handle {
            *read_lock_count += 1;
            return true;
        }

        // Try to lock the mutex
        if !self.mutex.try_lock() {
            return false;
        }

        // If there are no waiting writers, increment the read count
        let write_waiter_count = unsafe { &*self.write_waiter_count.get() };
        let got_lock = *write_waiter_count == 0;
        if got_lock {
            *read_lock_count += 1;
        }

        // Unlock the mutex to allow other threads to acquire the lock
        self.mutex.unlock();

        got_lock
    }

    /// Unlocks the [`RwLock`] for reading.
    ///
    /// This method must only be called by a thread that currently holds a read lock.
    /// If this is the last read lock and there are waiting writers, one of them will
    /// be woken up.
    pub fn read_unlock(&self) {
        let curr_thread_handle = get_curr_thread_handle();

        // If the current thread does not hold the write lock, decrement the read count
        // and wake up a writer if there are any.
        if self.write_owner_tag != curr_thread_handle {
            self.mutex.lock();

            // Decrement the read count.
            let read_lock_count = unsafe { &mut *self.read_lock_count.get() };
            *read_lock_count -= 1;

            // If there are no more readers and there are waiting writers, wake up one writer
            let write_waiter_count = unsafe { &*self.write_waiter_count.get() };
            if *read_lock_count == 0 && *write_waiter_count > 0 {
                self.condvar_writer_wait.wake_one();
            }

            self.mutex.unlock();
        } else {
            // If the current thread holds the write lock, decrement the read count without blocking
            let read_lock_count = unsafe { &mut *self.read_lock_count.get() };
            *read_lock_count -= 1;

            // If there are no more readers and there are waiting writers, wake up one writer
            let write_lock_count = unsafe { &*self.write_lock_count.get() };
            if *read_lock_count == 0 && *write_lock_count == 0 {
                self.write_owner_tag.clear();

                // Wake up a waiting writer if there are any,
                // otherwise wake up all waiting readers
                let write_waiter_count = unsafe { &*self.write_waiter_count.get() };
                let read_waiter_count = unsafe { &*self.read_waiter_count.get() };
                if *write_waiter_count > 0 {
                    self.condvar_writer_wait.wake_one();
                } else if *read_waiter_count > 0 {
                    self.condvar_reader_wait.wake_all();
                }

                self.mutex.unlock();
            }
        }
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
        let curr_thread_handle = get_curr_thread_handle();

        // If the current thread already holds the write lock, increment the write count
        // without blocking.
        let write_lock_count = unsafe { &mut *self.write_lock_count.get() };
        if self.write_owner_tag == curr_thread_handle {
            *write_lock_count += 1;
            return;
        }

        self.mutex.lock();

        // If there are waiting readers, increment the writer waiter count and wait for
        // the readers to finish.
        let read_lock_count = unsafe { &*self.read_lock_count.get() };
        let write_waiter_count = unsafe { &mut *self.write_waiter_count.get() };
        #[allow(clippy::while_immutable_condition)]
        while *read_lock_count > 0 {
            *write_waiter_count += 1;
            self.condvar_writer_wait.wait(&self.mutex);
            *write_waiter_count -= 1;
        }

        // Increment the write count, and set the write owner tag to the current thread
        *write_lock_count = 1;
        self.write_owner_tag.set(curr_thread_handle);

        // NOTE: The mutex is intentionally not unlocked here.
        //       It will be unlocked by a call to read_unlock or write_unlock.
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
        let curr_thread_handle = get_curr_thread_handle();

        // If the current thread already holds the write lock, increment the write count
        // without blocking.
        if self.write_owner_tag == curr_thread_handle {
            let write_lock_count = unsafe { &mut *self.write_lock_count.get() };
            *write_lock_count += 1;
            return true;
        }

        if !self.mutex.try_lock() {
            return false;
        }

        // If there are waiting readers, return false
        let read_lock_count = unsafe { &*self.read_lock_count.get() };
        if *read_lock_count > 0 {
            self.mutex.unlock();
            return false;
        }

        // Set the write count to 1, and set the write ownWriteUnlocker tag to the current thread
        let write_lock_count = unsafe { &mut *self.write_lock_count.get() };
        *write_lock_count = 1;
        self.write_owner_tag.set(curr_thread_handle);

        // NOTE: The mutex is intentionally not unlocked here.
        //       It will be unlocked by a call to read_unlock or write_unlock.

        true
    }

    /// Unlocks the [`RwLock`] for writing.
    ///
    /// This method must only be called by a thread that currently holds the write lock.
    /// When the last write lock is released, waiting writers are given priority over
    /// waiting readers to prevent writer starvation.
    pub fn write_unlock(&self) {
        // NOTE: This function assumes the write lock is held.
        //       This means that the mutex is locked, and the write owner tag is set
        //       to the current thread (write_owner_tag == curr_thread_handle).

        let write_lock_count = unsafe { &mut *self.write_lock_count.get() };
        *write_lock_count -= 1;

        // If there are no more writers and no readers, unlock the mutex and wake up
        // a waiting writer or all waiting readers.
        let read_lock_count = unsafe { &*self.read_lock_count.get() };
        if *write_lock_count == 0 && *read_lock_count == 0 {
            self.write_owner_tag.clear();

            // Wake up a waiting writer if there are any, otherwise wake up all waiting readers
            let write_waiter_count = unsafe { &*self.write_waiter_count.get() };
            let read_waiter_count = unsafe { &*self.read_waiter_count.get() };
            if *write_waiter_count > 0 {
                self.condvar_writer_wait.wake_one();
            } else if *read_waiter_count > 0 {
                self.condvar_reader_wait.wake_all();
            }

            self.mutex.unlock();
        }
    }

    /// Checks if the write lock is held by the current thread.
    ///
    /// # Returns
    ///
    /// * `true` if the current thread holds the write lock
    /// * `false` if it does not hold the write lock or only holds read locks
    pub fn is_write_lock_held_by_current_thread(&self) -> bool {
        self.write_owner_tag == get_curr_thread_handle() && {
            let write_lock_count = unsafe { &*self.write_lock_count.get() };
            *write_lock_count > 0
        }
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
        self.write_owner_tag == get_curr_thread_handle()
    }
}

impl Default for RwLock {
    fn default() -> Self {
        Self::new()
    }
}

/// Tag used to identify the owner of the write lock.
#[repr(transparent)]
struct WriteOwnerTag(UnsafeCell<u32>);

impl WriteOwnerTag {
    /// Creates a new [`WriteOwnerTag`] not associated with any handle.
    const fn new() -> Self {
        Self(UnsafeCell::new(INVALID_HANDLE))
    }

    fn set(&self, handle: Handle) {
        let inner = unsafe { &mut *self.0.get() };
        *inner = handle;
    }

    fn clear(&self) {
        self.set(INVALID_HANDLE)
    }
}

impl PartialEq<Handle> for WriteOwnerTag {
    fn eq(&self, other: &Handle) -> bool {
        let inner = unsafe { &*self.0.get() };
        *inner == *other
    }
}

/// Get the current thread's kernel handle
#[inline(always)]
fn get_curr_thread_handle() -> Handle {
    thread_vars::get_current_thread_handle()
}
