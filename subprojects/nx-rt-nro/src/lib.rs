//! # nx-rt-nro
//!
//! Homebrew-NRO entry crate for the Nintendo Switch runtime crate family.
//!
//! `nx-rt-nro` is the runtime for one output kind — the homebrew `NRO`
//! launched by the homebrew loader (hbloader / hbmenu). It stacks the
//! NRO-specific startup on top of the kind-agnostic [`nx_rt_core`]: the
//! hbloader-ABI configuration parser, the NRO command-line (`argv`) path, the
//! NRO process entry point, and the runtime-dispatched Application Manager
//! (applet) handshake.
//!
//! ## Output-kind row
//!
//! | App type / kind | Executable | Launched by | Applet type | Applet type sourced |
//! |-----------------|-----------|-------------|-------------|---------------------|
//! | Homebrew application | NRO | hbloader (hbmenu) | `Application` (or `LibraryApplet` on album-launch) | **Runtime** — hbl config |
//! | Homebrew library applet | NRO | hbloader / album | `LibraryApplet` | **Runtime** — hbl config |
//!
//! Unlike NSO and KIP processes, a homebrew NRO does not select its applet
//! type at build time: the homebrew loader supplies it through the `NRO`
//! configuration block, so the applet handshake is dispatched at runtime from
//! the parsed configuration. See [`nx_rt_core`] for the full App-Type /
//! Output-Kind matrix covering every Switch executable kind.
//!
//! # Cargo features
//!
//! The crate is split between an always-compiled **runtime core** (the NRO
//! `argv` path and the hbloader-ABI `env`/loader-config parser) and
//! **per-service managers** that are compiled in only when their feature is
//! enabled. The kind-agnostic pieces — heap init, HOS version, the Service
//! Manager bootstrap — live in [`nx_rt_core`]; `services::sm` is a thin
//! re-export of it.
//!
//! The Meson build system maps each `use_nx_service_<name>` option to the
//! corresponding `service-<name>` Cargo feature (and to the matching
//! `overrides/libnx_service_<name>.ld` linker fragment). Enabling a feature
//! pulls in that service's manager module, its FFI submodule (when `ffi` is
//! also on), and its `nx-service-<name>` crate dependency.
//!
//! ## Always compiled
//!
//! - **Runtime core:** `argv`, `cwd`, `env`
//! - **Service Manager:** `services::sm` — a thin re-export of
//!   [`nx_rt_core::services::sm`]; the SM bootstrap is kind-agnostic and
//!   owned by `nx-rt-core`, along with its FFI surface and overrides.
//!
//! ## Master switches
//!
//! - `ffi` — gates the entire `pub mod ffi` surface. Without it, no
//!   `__nx_rt_nro__libnx_*` symbols are emitted and the linker overrides have
//!   nothing to bind to. Enabling `ffi` alone yields the `rt_nro_libnx_core.ld`
//!   entry surface (`env_setup`, `argv`, `init_cwd`, `nxlink`).
//! - `rt-link` — emits this crate's hbloader `.crt0` (the homebrew-NRO
//!   `_start`) for the opt-in `rustc`-driven link pipeline. It is off on the
//!   default GCC pipeline, where `_start` is supplied by libnx's
//!   `switch_crt0.s`; enabling it there would collide with that `_start`.
//!
//! ## Per-service features
//!
//! Each gates a manager module under `crate::services::*` and, when `ffi`
//! is also on, the matching `crate::ffi::libnx::*` submodule.
//!
//! | Feature           | Manager module       | FFI submodule        | Implicit feature deps     |
//! |-------------------|----------------------|----------------------|---------------------------|
//! | `service-apm`     | `services::apm`      | `ffi::libnx::apm`    | —                         |
//! | `service-applet`  | `services::applet`   | `ffi::libnx::applet` | —                         |
//! | `service-hid`     | `services::hid`      | `ffi::libnx::hid`    | `service-applet`          |
//! | `service-nv`      | `services::nv`       | `ffi::libnx::nv`     | `service-applet`          |
//! | `service-set`     | `services::set`      | `ffi::libnx::setsys` | —                         |
//! | `service-time`    | `services::time`     | `ffi::libnx::time`   | —                         |
//! | `service-vi`      | `services::vi`       | `ffi::libnx::vi`     | —                         |
//!
//! `service-hid` and `service-nv` pull in `service-applet` because their
//! managers call `services::applet::get_applet_resource_user_id()`. Cargo
//! closes over those deps automatically; the matching `libnx_service_*.ld`
//! fragments are still gated independently in Meson and are linked in only
//! when their option is explicitly enabled.

#![no_std]

extern crate alloc;
extern crate nx_alloc as _; // provides #[global_allocator]
extern crate nx_panic_handler as _; // provides #[panic_handler]

// Homebrew-loader `.crt0` startup section for the `rustc`-link pipeline.
// Gated behind `rt-link` so the `_start` it defines is emitted only when
// `rustc` drives the final link; on the GCC pipeline `_start` comes from
// libnx's `switch_crt0.s`, and an unconditional `.crt0` would collide with it.
#[cfg(feature = "rt-link")]
core::arch::global_asm!(include_str!("crt0.s"));

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod argv;
pub mod cwd;
pub mod env;
pub mod services;
