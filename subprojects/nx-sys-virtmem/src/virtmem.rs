//! # Virtual Memory Management

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod reservation;

mod sys;

pub use sys::*;
