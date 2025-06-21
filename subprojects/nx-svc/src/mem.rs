//! Memory management system calls and utilities for the Horizon OS kernel.
//!
//! This module provides safe wrappers around memory-related system calls for querying
//! memory properties and unmapping memory.

pub mod core;
pub mod shmem;
pub mod tmem;

pub use self::core::*;
