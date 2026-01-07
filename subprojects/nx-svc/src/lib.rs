//! # nx-svc
//!
//! A Rust library for interacting with Horizon OS via _Supervisor Calls_ (SVCs).
//!
//! ## References:
//! - [Switchbrew Wiki: SVC](https://switchbrew.org/wiki/SVC)
//! - [switchbrew/libnx: `svc.h`](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/kernel/svc.h)
//! - [switchbrew/libnx: `svc.s`](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/source/kernel/svc.s)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

#[cfg(feature = "ffi")]
pub mod ffi;

#[macro_use]
mod handle;

pub mod code;
pub mod debug;
pub mod error;
pub mod ipc;
pub mod mem;
pub mod misc;
pub mod process;
pub mod raw;
pub mod result;
pub mod sync;
pub mod thread;
