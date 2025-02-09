//! # nx-cpu
//!
//! A Rust library for interacting with the Nintendo Switch's ARM Cortex-A57 (aarch64) CPU.

#![no_std]

pub mod counter;
pub mod tls;

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
