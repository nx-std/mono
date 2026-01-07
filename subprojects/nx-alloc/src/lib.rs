//! # nx-alloc
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod config;
#[cfg(feature = "ffi")]
mod ffi;
pub mod global;
pub mod llffalloc;
mod sync;
