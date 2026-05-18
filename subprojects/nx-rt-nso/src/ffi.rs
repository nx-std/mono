//! C-FFI surface for `nx-rt-nso`.
//!
//! Override symbols are grouped by the upstream archive they redirect. Every
//! `nx-rt-nso` override targets `libnx`, so [`libnx`] is the sole target
//! module.
//!
//! This module is gated behind the `ffi` Cargo feature.

pub mod libnx;

// Re-exported for the `argv` runtime path, which publishes the C-facing
// argc/argv globals through this setter.
pub(crate) use self::libnx::set_system_argv;
