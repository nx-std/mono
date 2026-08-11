//! Socket driver initialization.
//!
//! The socket driver itself lives in [`nx_sys_net`], and every one of its C symbols is exported
//! from there — except this one.
//!
//! # Why this entry point is here
//!
//! `socketInitialize`'s C contract is that the interface revision it declares to the BSD service
//! follows the running firmware. That makes it the one socket call whose behaviour depends on the
//! system version, and the system version is held by this crate, which [`nx_sys_net`] may not
//! depend on. So the ladder from firmware to revision lives in [`version`] below, and the symbol
//! that needs it lives beside it, calling [`nx_sys_net`]'s Rust entry with the choice already made.
//!
//! This is the same arrangement the controller applet uses, and for the same reason.

use core::ffi::c_void;

use nx_service_bsd::ConfigVersion;
use nx_sys_net::ffi::driver::{
    DEFAULT_INIT_CONFIG,
    SocketInitConfig,
};

use crate::{
    env::hos_version::{
        self,
        HosVersion,
    },
    services::sm,
};

/// Brings the socket driver up.
///
/// A null `config` selects the driver's default, which is how `socketInitializeDefault` is
/// written.
///
/// # Safety
///
/// `config` must be null or point to a readable [`SocketInitConfig`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_socketInitialize(config: *const c_void) -> u32 {
    let config = if config.is_null() {
        DEFAULT_INIT_CONFIG
    } else {
        // SAFETY: the caller guarantees a readable configuration at a non-null pointer.
        unsafe { *config.cast::<SocketInitConfig>() }
    };

    // A process gets one service manager session and this crate holds it, so the driver is handed
    // the session rather than opening a second one — which does not get a second session, it
    // fails.
    let Ok(sm) = sm::session() else {
        return nx_sys_net::ffi::driver::NO_SERVICE_MANAGER;
    };

    nx_sys_net::ffi::driver::initialize(&sm, &config, version())
}

/// Picks the interface revision the running firmware introduced.
///
/// The service accepts any revision up to its own, so this is an upper bound rather than an exact
/// match: declaring less than the firmware supports still works, and declaring more does not.
fn version() -> ConfigVersion {
    let current = hos_version::get();

    if current >= HosVersion::new(16, 0, 0) {
        ConfigVersion::V9
    } else if current >= HosVersion::new(13, 0, 0) {
        ConfigVersion::V8
    } else if current >= HosVersion::new(9, 0, 0) {
        ConfigVersion::V7
    } else if current >= HosVersion::new(8, 0, 0) {
        ConfigVersion::V6
    } else if current >= HosVersion::new(6, 0, 0) {
        ConfigVersion::V5
    } else if current >= HosVersion::new(5, 0, 0) {
        ConfigVersion::V4
    } else if current >= HosVersion::new(4, 0, 0) {
        ConfigVersion::V3
    } else if current >= HosVersion::new(3, 0, 0) {
        ConfigVersion::V2
    } else {
        ConfigVersion::V1
    }
}
