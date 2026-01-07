//! # nx-alloc
#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod config;
pub mod global;
pub mod llffalloc;
mod sync;
