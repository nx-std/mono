#![no_std]

extern crate alloc;
extern crate nx_alloc; // Provides #[global_allocator]
extern crate nx_panic_handler; // provides #[panic_handler]

#[cfg(feature = "ffi")]
mod ffi;

mod thread_impl;
pub mod tls_block;

pub use nx_sys_thread_tls as tls_region;

pub use self::thread_impl::*;
