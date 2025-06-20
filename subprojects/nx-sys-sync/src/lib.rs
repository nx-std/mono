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

#[cfg(feature = "ffi")]
mod ffi;

mod barrier;
mod condvar;
mod mutex;
mod remutex;
mod rwlock;
mod semaphore;

/// #[panic_handler]
///
/// Use different panic handlers for debug and release builds.
/// - 'dev': halt on panic. Easier to debug panics; can put a breakpoint on `rust_begin_unwind`
/// - 'release': abort on panic. Minimal binary size.
///
/// See:
///  - <https://doc.rust-lang.org/nomicon/panic-handler.html>
///  - <https://docs.rust-embedded.org/book/start/panicking.html>
#[cfg(not(debug_assertions))]
#[allow(unused_imports)]
use panic_abort as _;
#[cfg(debug_assertions)]
#[allow(unused_imports)]
use panic_halt as _;

#[doc(inline)]
pub use self::{
    barrier::Barrier, condvar::Condvar, mutex::Mutex, remutex::ReentrantMutex, rwlock::RwLock,
    semaphore::Semaphore,
};
