//! FFI exports for libnx runtime functions

mod argv;
mod common;
mod env;
mod sm;

#[cfg(feature = "service-apm")]
mod apm;
#[cfg(feature = "service-applet")]
mod applet;
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
pub(crate) use argv::set_system_argv;
// Called by argv::strip_nxlink_suffix() when _NXLINK_ suffix detected
pub(crate) use env::set_nxlink_host;
