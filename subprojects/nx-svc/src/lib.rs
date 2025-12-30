//! # nx-svc
//!
//! A Rust library for interacting with Horizon OS via _Supervisor Calls_ (SVCs).
//!
//! ## References:
//! - [Switchbrew Wiki: SVC](https://switchbrew.org/wiki/SVC)
//! - [switchbrew/libnx: `svc.h`](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/svc.h)
//! - [switchbrew/libnx: `svc.s`](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/source/kernel/svc.s)

#![no_std]

/// #[panic_handler]
///
/// Custom panic handler that calls svcBreak with Panic reason.
/// This provides better debugging information for Nintendo Switch homebrew applications.
///
/// See:
///  - <https://doc.rust-lang.org/nomicon/panic-handler.html>
///  - <https://docs.rust-embedded.org/book/start/panicking.html>
mod panic_handler;

#[macro_use]
mod handle;

pub mod code;
pub mod debug;
pub mod error;
pub mod mem;
pub mod misc;
pub mod raw;
pub mod result;
pub mod sync;
pub mod thread;

#[cfg(feature = "ffi")]
mod ffi;
