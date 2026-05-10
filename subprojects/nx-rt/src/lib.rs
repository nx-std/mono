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
//! - **Runtime core:** `argv`, `env`, `init`, `thread_registry`
//! - **Service Manager:** `service_manager` (depends on `nx-service-sm` —
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
//! Each gates a manager module under `crate::*_manager` (or
//! `crate::service_registry` for `set:sys`) and, when `ffi` is also on, the
//! matching `crate::ffi::*` submodule.
//!
//! | Feature           | Manager module      | FFI submodule       | Implicit feature deps     |
//! |-------------------|---------------------|---------------------|---------------------------|
//! | `service-apm`     | `apm_manager`       | `ffi::apm`          | —                         |
//! | `service-applet`  | `applet_manager`    | `ffi::applet`       | —                         |
//! | `service-hid`     | `hid_manager`       | `ffi::hid`          | `service-applet`          |
//! | `service-nv`      | `nv_manager`        | `ffi::nv`           | `service-applet`          |
//! | `service-set`     | `service_registry`  | `ffi::setsys`       | —                         |
//! | `service-time`    | `time_manager`      | `ffi::time`         | —                         |
//! | `service-vi`      | `vi_manager`        | `ffi::vi`           | —                         |
//!
//! `service-hid` and `service-nv` pull in `service-applet` because their
//! managers call `applet_manager::get_applet_resource_user_id()`. Cargo
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
pub mod service_manager;
pub mod thread_registry;

#[cfg(feature = "service-apm")]
pub mod apm_manager;
#[cfg(feature = "service-applet")]
pub mod applet_manager;
#[cfg(feature = "service-hid")]
pub mod hid_manager;
#[cfg(feature = "service-nv")]
pub mod nv_manager;
#[cfg(feature = "service-set")]
pub mod service_registry;
#[cfg(feature = "service-time")]
pub mod time_manager;
#[cfg(feature = "service-vi")]
pub mod vi_manager;
