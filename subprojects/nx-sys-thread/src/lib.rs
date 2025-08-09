#![no_std]

// The `alloc` crate enables memory allocation.
extern crate alloc;
// The `nx-alloc` crate exposes the `#[global_allocator]` for the dependent crates.
extern crate nx_alloc;

mod init;
mod registry;
mod thread_context;
mod thread_create;
mod thread_exit;
mod thread_handle;
mod thread_impl;
mod thread_inner;
mod thread_pause;
mod thread_resume;
mod thread_sleep;
mod thread_stackmem;
mod thread_start;
mod thread_wait;
pub mod tls_block;
pub mod tls_region;
pub use init::*;
pub use registry::*;
pub use thread_impl::*;

#[cfg(feature = "ffi")]
mod ffi;
