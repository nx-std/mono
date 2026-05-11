//! # Runtime Module
//!
//! This crate provides runtime initialization functions for Nintendo Switch applications,
//! including command-line argument parsing and environment setup.
//!
//! # Cargo features
//!
//! The crate is split between an always-compiled **runtime core** (heap init,
//! argv, env/loader config, HOS version, Service Manager) and **per-service
//! managers** that are compiled in only when their feature is enabled.
//!
//! The Meson build system maps each `use_nx_service_<name>` option to the
//! corresponding `service-<name>` Cargo feature (and to the matching
//! `overrides/rt_service_<name>.ld` linker fragment). Enabling a feature
//! pulls in that service's manager module, its FFI submodule (when `ffi` is
//! also on), and its `nx-service-<name>` crate dependency.
//!
//! ## Always compiled
//!
//! - **Runtime core:** `argv`, `env`, `init`
//! - **Service Manager:** `services::sm` (depends on `nx-service-sm` —
//!   non-optional; `sm` is the foundation every other service builds on)
//!
//! ## Master switches
//!
//! - `ffi` — gates the entire `pub mod ffi` surface. Without it, no
//!   `__nx_rt__*` symbols are emitted and the linker overrides have nothing
//!   to bind to. Enabling `ffi` alone yields runtime core + sm overrides.
//!
//! ## Per-service features
//!
//! Each gates a manager module under `crate::services::*` and, when `ffi`
//! is also on, the matching `crate::ffi::*` submodule.
//!
//! | Feature           | Manager module       | FFI submodule       | Implicit feature deps     |
//! |-------------------|----------------------|---------------------|---------------------------|
//! | `service-apm`     | `services::apm`      | `ffi::apm`          | —                         |
//! | `service-applet`  | `services::applet`   | `ffi::applet`       | —                         |
//! | `service-hid`     | `services::hid`      | `ffi::hid`          | `service-applet`          |
//! | `service-nv`      | `services::nv`       | `ffi::nv`           | `service-applet`          |
//! | `service-set`     | `services::set`      | `ffi::setsys`       | —                         |
//! | `service-time`    | `services::time`     | `ffi::time`         | —                         |
//! | `service-vi`      | `services::vi`       | `ffi::vi`           | —                         |
//!
//! `service-hid` and `service-nv` pull in `service-applet` because their
//! managers call `services::applet::get_applet_resource_user_id()`. Cargo
//! closes over those deps automatically; the matching `rt_service_*.ld`
//! fragments are still gated independently in Meson and are linked in only
//! when their option is explicitly enabled.

#![no_std]

extern crate alloc;
extern crate nx_alloc; // Provides #[global_allocator]
extern crate nx_panic_handler; // Provides #[panic_handler]

#[cfg(feature = "ffi")]
pub mod ffi;

pub mod argv;
pub mod env;
pub mod init;
pub mod services;
