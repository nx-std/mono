//! # nx-sys-sync
//!
//! Switchbrew libnx synchronization primitives
//!
//! This module contains synchronization primitives ported from Switchbrew's libnx.
//!
//! # References
//!
//! - [switchbrew/libnx: switch/kernel/mutex.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/mutex.h)
//! - [switchbrew/libnx: switch/kernel/condvar.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/condvar.h)
//! - [switchbrew/libnx: switch/kernel/rwlock.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/rwlock.h)
//! - [switchbrew/libnx: switch/kernel/barrier.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/barrier.h)
//! - [switchbrew/libnx: switch/kernel/semaphore.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/semaphore.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

#[cfg(feature = "ffi")]
mod ffi;

mod barrier;
mod condvar;
mod mutex;
mod once;
mod remutex;
mod rwlock;
mod semaphore;

#[doc(inline)]
pub use self::{
    barrier::Barrier, condvar::Condvar, mutex::Mutex, once::Once, remutex::ReentrantMutex,
    rwlock::RwLock, semaphore::Semaphore,
};
