//! # Runtime Module
//!
//! This crate provides runtime initialization functions for Nintendo Switch applications,
//! including command-line argument parsing and environment setup.

#![no_std]

extern crate alloc;
extern crate nx_alloc; // Provides #[global_allocator]
extern crate nx_panic_handler; // Provides #[panic_handler]

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod apm_manager;
pub mod applet_manager;
pub mod argv;
pub mod env;
pub mod hid_manager;
pub mod init;
pub mod service_manager;
pub mod service_registry;
pub mod thread_registry;
