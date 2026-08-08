//! `libnx` symbol-override FFI for `nx-rt-core`.
//!
//! Holds the `__nx_rt_core__libnx_*` symbols that redirect the kind-agnostic
//! `libnx` runtime entry points — heap init, main-thread TLS, the environment
//! read accessors, the HOS-version API, the Service Manager set, and the
//! role-independent Application Manager (`applet*`) accessor surface — to this
//! crate. The override aliases that bind them live in
//! `overrides/rt_core_libnx_core.ld` and `overrides/rt_core_libnx_service_applet.ld`.
//!
//! The kind-specific entry points (`envSetup`, `argvSetup`, `__system_argc` /
//! `__system_argv`, `__nxlink_host`, `__nx_applet_type`, `appletInitialize`,
//! `appletGetAppletType`) are intentionally absent: each output-kind entry
//! crate owns them, since they source the applet-type value.

#[cfg(feature = "service-applet")]
pub mod applet;
#[cfg(feature = "service-applet")]
pub mod libapplet;

mod env;
mod sm;
