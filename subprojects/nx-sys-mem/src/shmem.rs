//! Shared memory management

#[cfg(feature = "ffi")]
pub mod ffi;

mod sys;

pub use sys::*;
