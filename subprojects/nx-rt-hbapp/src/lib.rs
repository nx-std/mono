//! # nx-rt-hbapp
//!
//! Homebrew-application entry crate for the Nintendo Switch runtime crate
//! family.
//!
//! `nx-rt-hbapp` is the runtime for one launch path: the homebrew application
//! handed control by the homebrew loader (hbloader / hbmenu). The `NRO` it is
//! packaged as does not tell it apart from `nx-rt-module`, which builds an
//! `NRO` the `ro` service loads into a process that is already running; the
//! handoff does.
//!
//! It stacks the loader-specific startup on top of the launch-path-agnostic
//! [`nx_rt_core`]: the hbloader-ABI configuration parser, the loader-supplied
//! command-line (`argv`) path, the process entry point, and the
//! runtime-dispatched Application Manager (applet) handshake.
//!
//! ## Launch-path rows
//!
//! | App type | Executable | Launched by | Applet type | Applet type sourced |
//! |----------|-----------|-------------|-------------|---------------------|
//! | Homebrew application | NRO | hbloader (hbmenu) | `Application` (or `LibraryApplet` on album-launch) | **Runtime**: hbl config |
//! | Homebrew library applet | NRO | hbloader / album | `LibraryApplet` | **Runtime**: hbl config |
//!
//! Unlike a `pm`-launched or kernel-launched process, a loader-launched
//! homebrew application does not select its applet type at build time: the
//! homebrew loader supplies it through the configuration block it hands over,
//! so the applet handshake is dispatched at runtime from the parsed
//! configuration. See [`nx_rt_core`] for the full App-Type / Launch-Path
//! matrix covering every Switch executable.
//!
//! # Cargo features
//!
//! The crate is split between an always-compiled **runtime core** (the
//! loader-supplied `argv` path and the hbloader-ABI `env`/loader-config
//! parser) and **per-service managers** that are compiled in only when their
//! feature is enabled. The launch-path-agnostic pieces: heap init, HOS
//! version, the Service Manager bootstrap: live in [`nx_rt_core`];
//! `services::sm` is a thin re-export of it.
//!
//! The Meson build system maps each `use_nx_service_<name>` option to the
//! corresponding `service-<name>` Cargo feature (and to the matching
//! `overrides/libnx_service_<name>.ld` linker fragment). Enabling a feature
//! pulls in that service's manager module, its FFI submodule (when `ffi` is
//! also on), and its `nx-service-<name>` crate dependency.
//!
//! ## Always compiled
//!
//! - **Runtime core:** `app`, `argv`, `cwd`, `env`
//! - **Startup sequence:** `app`, holding the order libnx's `__appInit` and
//!   `__appExit` open and close the default services in. Every step but the Service
//!   Manager is behind a `service-*` feature, and a step whose feature is off
//!   calls libnx's own entry point instead of skipping.
//! - **Service Manager:** `services::sm`: a thin re-export of
//!   [`nx_rt_core::services::sm`]; the SM bootstrap does not depend on the
//!   launch path and is owned by `nx-rt-core`, along with its FFI surface and
//!   overrides.
//!
//! ## Master switches
//!
//! - `ffi`: gates the entire `pub mod ffi` surface. Without it, no
//!   `__nx_rt_hbapp__libnx_*` symbols are emitted and the linker overrides
//!   have nothing to bind to. Enabling `ffi` alone yields the
//!   `rt_hbapp_libnx_core.ld` entry surface (`env_setup`, `argv`, `app_init`,
//!   `app_exit`, `init_cwd`, `nxlink`).
//! - `rt-link`: emits this crate's hbloader `.crt0` (the loader-handoff
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
//! | `service-apm`     | `services::apm`      | `ffi::libnx::apm`    | -                         |
//! | `service-applet`  | `services::applet`   | `ffi::libnx::applet` | -                         |
//! | `service-hid`     | `services::hid`      | `ffi::libnx::hid`    | `service-applet`          |
//! | `service-nv`      | `services::nv`       | `ffi::libnx::nv`     | `service-applet`          |
//! | `service-set`     | `services::set`      | `ffi::libnx::setsys` | -                         |
//! | `service-time`    | `services::time`     | `ffi::libnx::time`   | -                         |
//! | `service-vi`      | `services::vi`       | `ffi::libnx::vi`     | -                         |
//!
//! `service-hid` and `service-nv` pull in `service-applet` because their
//! managers call `services::applet::get_applet_resource_user_id()`. Cargo
//! closes over those deps automatically; the matching `libnx_service_*.ld`
//! fragments are still gated independently in Meson and are linked in only
//! when their option is explicitly enabled.

#![no_std]
// `app` calls the four startup hooks a program may define, each declared weak
// and undefined so an absent one reads back as null instead of failing the
// link. `extern_weak` linkage is the only way to say that in Rust.
#![feature(linkage)]

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

pub mod app;
pub mod argv;
pub mod cwd;
pub mod env;
pub mod init;
#[cfg(feature = "romfs")]
pub mod romfs;
pub mod services;
