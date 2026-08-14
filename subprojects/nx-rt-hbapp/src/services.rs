//! Per-service manager modules.
//!
//! Each submodule owns the lifecycle (init/exit), session state, and accessor
//! surface for one Horizon OS service. The Service Manager (`sm`) submodule is
//! always compiled because it is the bootstrap every other service depends on;
//! the remaining submodules are gated behind their corresponding `service-*`
//! Cargo feature.
//!
//! Each manager stores its session in module-local static state guarded by a
//! `RwLock`, exposing typed accessors that return RAII guards.

pub mod sm;

#[cfg(feature = "service-apm")]
pub mod apm;
#[cfg(feature = "service-applet")]
pub mod applet;
#[cfg(feature = "service-fs")]
pub mod fs;
#[cfg(feature = "service-hid")]
pub mod hid;
#[cfg(feature = "service-nv")]
pub mod nv;
#[cfg(feature = "service-set")]
pub mod set;
#[cfg(feature = "service-time")]
pub mod time;
#[cfg(feature = "service-vi")]
pub mod vi;
