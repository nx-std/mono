//! # nx-sys-thread
//!
//! A musl libc-style thread core for the Nintendo Switch's Horizon OS, paired
//! with C ABI adapters that override libnx and devkitPro/libsysbase thread
//! symbols at link time.
//!
//! The crate is split into an idiomatic Rust core and — behind the `ffi`
//! feature — thin C ABI adapter layers. The module tree is being built up
//! incrementally; today it exposes the TCB/DTV TLS header types in [`tcb`],
//! the ELF TLS segment management in [`tls_block`], the runtime
//! thread-specific-data foundations in [`tsd`], the idiomatic core thread
//! state in [`thread`], the pthread/newlib syscall core in [`pthread`], the
//! process-wide live-thread registry in [`thread_list`], and the [`ffi`] C ABI
//! adapter surface.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The `alloc` crate enables heap allocation (e.g. the per-thread DTV node).
extern crate alloc;
// `nx-alloc` exposes the `#[global_allocator]` backing `alloc` for this crate.
extern crate nx_alloc;

mod detach;
#[cfg(feature = "ffi")]
pub mod error;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod pthread;
pub mod tcb;
pub mod thread;
pub mod thread_list;
pub mod tls_block;
pub mod tsd;
