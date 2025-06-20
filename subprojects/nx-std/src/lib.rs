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
#[cfg(feature = "thread")]
pub mod thread {
    pub use nx_std_thread::*;
}
#[cfg(feature = "time")]
pub mod time {
    pub use nx_time::*;
}

#[cfg(any(
    feature = "sys",
    feature = "svc",
    feature = "sys-sync",
    feature = "sys-thread"
))]
pub mod sys {
    #[cfg(any(feature = "sys", feature = "svc"))]
    pub use nx_svc as svc;
    #[cfg(any(feature = "sys", feature = "sys-sync"))]
    pub use nx_sys_sync as sync;
    #[cfg(any(feature = "sys", feature = "sys-thread"))]
    pub use nx_sys_thread as thread;
}

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
