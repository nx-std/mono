//! # nx-std
#![no_std]

// The `alloc` crate enables memory allocation.
#[cfg(feature = "alloc")]
extern crate alloc as _;
// The `nx-alloc` crate exposes the `#[global_allocator]` for the dependent crates.
#[cfg(feature = "alloc")]
extern crate nx_alloc;

#[cfg(feature = "alloc")]
pub mod alloc {
    pub use nx_alloc::*;
}
#[cfg(feature = "rand")]
pub mod rand {
    pub use nx_rand::*;
}
#[cfg(feature = "sync")]
pub mod sync {
    pub use nx_std_sync::*;
}
#[cfg(feature = "time")]
pub mod time {
    pub use nx_time::*;
}

#[cfg(any(
    feature = "sys",
    feature = "svc",
    feature = "sys-mem",
    feature = "sys-sync",
    feature = "sys-thread"
))]
pub mod sys {
    #[cfg(any(feature = "sys", feature = "svc"))]
    pub use nx_svc as svc;
    #[cfg(any(feature = "sys", feature = "sys-mem"))]
    pub use nx_sys_mem as mem;
    #[cfg(any(feature = "sys", feature = "sys-sync"))]
    pub use nx_sys_sync as sync;
    #[cfg(any(feature = "sys", feature = "sys-thread"))]
    pub use nx_sys_thread as thread;
}
