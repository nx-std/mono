//! `IEventNotifier` commands.
//!
//! Commands without an implementation are aliased to panicking stubs: one
//! left to libnx hangs rather than failing. See the parent module.
//!
//! Struct parameters are typed as opaque pointers; every one is a pointer, so
//! the ABI is exact without restating a layout this crate cannot check.

use core::ffi::c_void;

use nx_sf::ffi::Service;

/// Stands in for libnx's `fsOpenSdCardDetectionEventNotifier`.
///
/// # Safety
///
/// `out` must point to a writable `Service`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_open_sd_card_detection_event_notifier(
    _out: *mut Service,
) -> u32 {
    todo!("fsOpenSdCardDetectionEventNotifier")
}

/// Stands in for libnx's `fsEventNotifierGetEventHandle`.
///
/// # Safety
///
/// `e` must point to a `Service` this module handed out, and `out` to a
/// writable `Event`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_event_notifier_get_event_handle(
    _e: *mut Service,
    _out: *mut c_void,
    _autoclear: bool,
) -> u32 {
    todo!("fsEventNotifierGetEventHandle")
}

/// Stands in for libnx's `fsEventNotifierClose`.
///
/// # Safety
///
/// `e` must point to a `Service` this module handed out, and must not be closed
/// twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_nro__libnx_fs_event_notifier_close(_e: *mut Service) {
    todo!("fsEventNotifierClose")
}
