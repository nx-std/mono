//! # Runtime Module
//!
//! This crate provides runtime initialization functions for Nintendo Switch applications,
//! including command-line argument parsing and environment setup.
//!
//! It re-exports all `nx-rt-env` APIs and adds allocation-dependent functionality like `argv_setup`.

#![no_std]

extern crate alloc;
extern crate nx_alloc; // Provides #[global_allocator]
extern crate nx_panic_handler; // Provides #[panic_handler]

#[cfg(feature = "ffi")]
mod ffi;

pub mod argv;

// Re-export all nx-rt-env APIs
pub mod env {
    pub use nx_rt_env::*;
}
