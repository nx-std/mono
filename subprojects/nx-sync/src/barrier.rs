//! Barrier
//! Multi-threading Barrier

use static_assertions::const_assert_eq;

use crate::{condvar::Condvar, mutex::Mutex};

/// Barrier structure.
// TODO: Review this. We could do this with a single atomic variable,
//       but we're going to use a mutex and a condition variable for compatibility
pub struct Barrier {
    /// Number of threads to reach the barrier
    count: u64,
    /// Number of threads to wait on
    total: u64,
    /// Mutex for synchronization
    mutex: Mutex,
    /// Condition variable for thread waiting
    condvar: Condvar,
}

// Ensure that the Barrier has a 24 bytes size, and is properly aligned
const_assert_eq!(size_of::<Barrier>(), 24);
const_assert_eq!(align_of::<Barrier>(), align_of::<u64>());

impl Barrier {
    /// Initializes a barrier and the number of threads to wait on.
    ///
    /// # Arguments
    /// * `thread_count` - Initial value for the number of threads the barrier must wait for.
    pub fn new(thread_count: u64) -> Self {
        Barrier {
            count: 0,
            total: thread_count - 1,
            mutex: Mutex::new(),
            condvar: Condvar::new(),
        }
    }

    /// Forces threads to wait until all threads have called barrier_wait.
    pub fn wait(&mut self) {
        unsafe { __nx_sync_barrier_wait(self) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_barrier_init(bar: *mut Barrier, thread_count: u64) {
    unsafe { bar.write(Barrier::new(thread_count)) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sync_barrier_wait(bar: *mut Barrier) {
    let bar = unsafe { &mut *bar };
    bar.mutex.lock();

    if bar.count == bar.total {
        bar.count = 0;
        bar.condvar.wake(bar.total as i32);
    } else {
        bar.count = bar.count.checked_add(1).expect("Barrier count overflow");
        bar.condvar.wait(&bar.mutex);
    }

    bar.mutex.unlock();
}
