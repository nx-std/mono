#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

// The `alloc` crate enables memory allocation.
extern crate alloc;
// The `nx-alloc` crate exposes the `#[global_allocator]` for the dependent crates.
extern crate nx_alloc;

mod init;
mod registry;
mod thread_impl;
pub mod tls_block;

pub use init::*;
pub use nx_sys_thread_tls as tls_region;
pub use registry::*;
pub use thread_impl::*;

#[cfg(feature = "ffi")]
mod ffi;
