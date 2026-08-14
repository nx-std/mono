//! C-FFI surface for `nx-rt-hbapp`.
//!
//! Override symbols are grouped by the upstream archive they redirect. Every
//! `nx-rt-hbapp` override targets `libnx`, so [`libnx`] is the sole target
//! module; `common` holds the target-agnostic FFI helpers.
//!
//! This module is gated behind the `ffi` Cargo feature.

#[cfg(any(
    feature = "service-apm",
    feature = "service-applet",
    feature = "service-fs",
    feature = "service-hid",
    feature = "service-nv",
    feature = "service-set",
    feature = "service-time",
    feature = "service-vi",
))]
mod common;

pub mod libnx;
