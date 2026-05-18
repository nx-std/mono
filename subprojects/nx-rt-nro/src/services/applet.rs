//! Applet Manager (AM) — re-exported from [`nx_rt_core`].
//!
//! The applet handshake is kind-agnostic: an NRO and an NSO perform the same
//! libnx-faithful per-role bring-up — only the source of the applet-type
//! value differs. Its single authoritative implementation therefore lives in
//! [`nx_rt_core::services::applet`]; this module re-exports it so the NRO FFI
//! shims and the per-service managers keep resolving `crate::services::applet`.
//!
//! The NRO sources its applet-type value at runtime from the parsed homebrew
//! loader configuration (see [`crate::env::applet_type`]).

pub use nx_rt_core::services::applet::*;
