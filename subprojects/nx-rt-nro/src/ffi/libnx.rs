//! `libnx` symbol-override FFI for `nx-rt-nro`.
//!
//! Holds the `__nx_rt_nro__libnx_*` symbols that redirect the homebrew-NRO
//! `libnx` entry points — the loader-config `env` setup, the `argv` path, the
//! `nxlink` host global, the applet-type-sourcing applet shims, and the
//! per-service surfaces — to this crate. The override aliases live in
//! `overrides/rt_nro_libnx_core.ld` and the per-service `overrides/rt_nro_libnx_service_*.ld`
//! fragments.

mod argv;
mod env;

#[cfg(feature = "service-apm")]
mod apm;
#[cfg(feature = "service-applet")]
mod applet;
#[cfg(feature = "service-applet-err")]
mod applet_err;
#[cfg(feature = "service-fs")]
mod fs;
#[cfg(feature = "service-hid")]
mod hid;
#[cfg(feature = "service-nv")]
mod nv;
#[cfg(feature = "service-set")]
mod setsys;
#[cfg(feature = "service-time")]
mod time;
#[cfg(feature = "service-vi")]
mod vi;

// Called by argv::setup() after parsing argv from loader config
pub(crate) use self::argv::set_system_argv;
// Called by argv::strip_nxlink_suffix() when _NXLINK_ suffix detected
pub(crate) use self::env::set_nxlink_host;
