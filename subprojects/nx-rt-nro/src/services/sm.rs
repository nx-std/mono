//! Service Manager (SM) — re-exported from [`nx_rt_core`].
//!
//! The Service Manager bootstrap is kind-agnostic: every output kind shares
//! one SM session and one override table. Its single authoritative
//! implementation lives in [`nx_rt_core::services::sm`]; this module
//! re-exports it so the per-service managers keep resolving
//! `crate::services::sm`.

pub use nx_rt_core::services::sm::*;
