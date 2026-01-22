//! FFI exports for libnx runtime functions

mod apm;
mod applet;
mod argv;
mod common;
mod env;
mod hid;
mod nv;
mod setsys;
mod sm;
mod time;
mod vi;

// Called by argv::setup() after parsing argv from loader config
pub(crate) use argv::set_system_argv;
// Called by argv::strip_nxlink_suffix() when _NXLINK_ suffix detected
pub(crate) use env::set_nxlink_host;
