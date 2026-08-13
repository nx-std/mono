//! `libnx` symbol-override FFI for `nx-rt-nro`.
//!
//! Holds the `__nx_rt_nro__libnx_*` symbols that redirect the homebrew-NRO
//! `libnx` entry points to this crate: the loader-config `env` setup, the
//! `argv` path, the service bring-up and teardown sequence, the startup
//! working directory, the `nxlink` host global, the
//! applet-type-sourcing applet shims, and the per-service surfaces. The override aliases live in
//! `overrides/rt_nro_libnx_core.ld` and the per-service `overrides/rt_nro_libnx_service_*.ld`
//! fragments.

mod app;
mod argv;
mod cwd;
mod env;
mod init;

#[cfg(feature = "service-applet-album")]
mod album;
#[cfg(feature = "service-apm")]
mod apm;
#[cfg(feature = "service-applet")]
mod applet;
#[cfg(feature = "service-applet-err")]
mod applet_err;
#[cfg(feature = "service-applet-friends")]
mod friends;
#[cfg(feature = "service-fs")]
mod fs;
#[cfg(feature = "service-hid")]
mod hid;
#[cfg(feature = "service-applet-hid")]
mod hid_la;
#[cfg(feature = "service-applet-mii")]
mod mii;
#[cfg(feature = "service-applet-nfp")]
mod nfp;
#[cfg(feature = "service-nv")]
mod nv;
#[cfg(feature = "nvmap")]
mod nvmap;
#[cfg(feature = "service-applet-pctlauth")]
mod pctlauth;
#[cfg(feature = "service-applet-psel")]
mod psel;
#[cfg(feature = "romfs")]
mod romfs;
#[cfg(feature = "service-set")]
mod setsys;
#[cfg(feature = "service-time")]
mod time;
#[cfg(feature = "service-vi")]
mod vi;

// The startup sequence parses the environment and the command line, then
// calls these to point the C-facing globals at what it parsed. The parse and
// the publication are separate so that the modules holding the parsed values
// do not have to reach into this one.
pub(crate) use self::{
    argv::publish as publish_argv,
    env::publish_applet_type,
};
