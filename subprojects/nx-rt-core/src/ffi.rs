//! C-FFI surface for `nx-rt-core`.
//!
//! Override symbols are grouped by the upstream archive they redirect. Every
//! `nx-rt-core` override targets `libnx`, so [`libnx`] is the sole target
//! module; [`common`] holds the target-agnostic FFI helpers shared across the
//! `nx-rt-*` family.
//!
//! This module is gated behind the `ffi` Cargo feature.

pub mod common;
pub mod libnx;
