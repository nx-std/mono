//! # nx-cpu
//!
//! A Rust library for interacting with the Nintendo Switch's ARM Cortex-A57 (aarch64) CPU.

#![no_std]

#[cfg(not(target_arch = "aarch64"))]
compile_error!("nx-cpu only supports aarch64 CPUs");

extern crate nx_panic_handler as _; // provides #[panic_handler]

pub mod barrier;
pub mod control_regs;
