//! # nx-std
#![no_std]

extern crate nx_panic_handler; // provides #[panic_handler]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
pub extern crate nx_alloc; // Provides #[global_allocator]

// FFI exports - re-export FFI symbols from dependent crates to ensure they're
// included in the staticlib. This module is only compiled when the `ffi` feature
// is enabled.
#[cfg(feature = "ffi")]
pub mod ffi;

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
#[cfg(feature = "rt")]
pub mod rt {
    pub use nx_rt::*;
}

#[cfg(any(
    feature = "sys",
    feature = "alloc",
    feature = "svc",
    feature = "sys-mem",
    feature = "sys-sync",
    feature = "sys-thread",
    feature = "sys-thread-tls"
))]
pub mod sys {
    #[cfg(any(feature = "sys", feature = "alloc"))]
    pub use nx_alloc as alloc;
    #[cfg(any(feature = "sys", feature = "svc"))]
    pub use nx_svc as svc;
    #[cfg(any(feature = "sys", feature = "sys-mem"))]
    pub use nx_sys_mem as mem;
    #[cfg(any(feature = "sys", feature = "sys-sync"))]
    pub use nx_sys_sync as sync;
    #[cfg(any(feature = "sys", feature = "sys-thread"))]
    pub use nx_sys_thread as thread;
    #[cfg(any(feature = "sys", feature = "sys-thread-tls"))]
    pub use nx_sys_thread_tls as thread_tls;
}
