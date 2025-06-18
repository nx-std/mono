//! FFI bindings for the `nx-sync` crate
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/mutex.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/mutex.h)
//! - [switchbrew/libnx: switch/kernel/condvar.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/condvar.h)
//! - [switchbrew/libnx: switch/kernel/rwlock.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/rwlock.h)
//! - [switchbrew/libnx: switch/kernel/barrier.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/barrier.h)
//! - [switchbrew/libnx: switch/kernel/semaphore.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/semaphore.h)

mod barrier;
mod condvar;
mod mutex;
mod rwlock;
mod semaphore;
