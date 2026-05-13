//! C-FFI surface for the `pm:*` services.
//!
//! Exports `__nx_pm__*` symbols that the `pm_override.ld` linker script
//! aliases to libnx's `pm.h` ABI (`pmbm*`, `pmdmnt*`, `pminfo*`, `pmshell*`).
//! The shape mirrors `nx-rt`'s service-FFI pattern: lazy SM connection,
//! per-service singleton state guarded by `RwLock`, and a libnx-compatible
//! `Service` shadow buffer returned by `pm*GetServiceSession`.

mod bm;
mod common;
mod dmnt;
mod info;
mod shell;
mod state;
