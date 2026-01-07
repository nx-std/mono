//! # Virtual Memory Management

#[cfg(feature = "ffi")]
pub mod ffi;

mod sys;

pub use sys::*;
