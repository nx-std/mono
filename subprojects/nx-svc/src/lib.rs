//! # nx-svc
//!
//! A Rust library for interacting with Horizon OS via _Supervisor Calls_ (SVCs).
//!
//! ## C FFI API
//!
//! This library exposes all the Raw SVC functions from the [Raw API](raw) module so they can be
//! called from C code.
//!
//! ```C
//! #include <nx_svc.h>
//!
//! Result svcSetHeapSize(void** out_addr, u64 size) {
//!   return __nx_svc_set_heap_size(out_addr, size);  /* Call Rust function */
//! }
//! ```
//!
//! The C header file, `nx_svc.h`, can be found under `include` directory.
//!
//! ## References:
//! - [Switchbrew Wiki: SVC](https://switchbrew.org/wiki/SVC)
//! - [switchbrew/libnx: `svc.h`](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/svc.h)
//! - [switchbrew/libnx: `svc.s`](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/source/kernel/svc.s)

#![no_std]

pub mod code;
pub mod debug;
pub mod error;
pub mod raw;
pub mod result;
pub mod sync;

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
