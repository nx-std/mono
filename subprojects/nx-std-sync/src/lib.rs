//! # nx-std-sync
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The `alloc` crate enables memory allocation.
extern crate alloc;
// The `nx-alloc` crate exposes the `#[global_allocator]` for the dependent crates.
extern crate nx_alloc;

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod barrier;
pub mod condvar;
pub mod mutex;
pub mod once_lock;
pub mod oneshot;
mod result;
pub mod rwlock;
pub mod semaphore;
