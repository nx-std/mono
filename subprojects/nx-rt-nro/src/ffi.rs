//! C-FFI surface for `nx-rt-nro`.
//!
//! Override symbols are grouped by the upstream archive they redirect. Every
//! `nx-rt-nro` override targets `libnx`, so [`libnx`] is the sole target
//! module; `common` holds the target-agnostic FFI helpers.
//!
//! This module is gated behind the `ffi` Cargo feature.

#[cfg(any(
    feature = "service-apm",
    feature = "service-applet",
    feature = "service-hid",
    feature = "service-nv",
    feature = "service-set",
    feature = "service-time",
    feature = "service-vi",
))]
mod common;

pub mod libnx;

// Re-exported for the `argv` / `env` runtime paths, which publish the C-facing
// globals through these setters.
pub(crate) use self::libnx::{set_nxlink_host, set_system_argv};
