//! Kind-agnostic Application Manager (applet) FFI.
//!
//! Redirects the role-independent `libnx` `applet*` accessor surface — the
//! operation/performance/focus-mode queries, the message-event pair, the
//! notification setters, the ARUID and managed-display-layer calls, and
//! `appletExit` — to `nx-rt-core`. Every Switch executable kind performs the
//! same calls against the runtime applet singleton in
//! [`crate::services::applet`]; none of them read the applet-type value, so
//! the single authoritative implementation lives here and both the NRO and
//! NSO entry crates expose the full ABI from it.
//!
//! The applet-type-sourcing entry points — `appletInitialize` and
//! `appletGetAppletType` — are **not** here. They are kind-specific (an NRO
//! reads the applet type from the loader configuration, an NSO from a
//! build-time selection), so each entry crate owns them and reuses the shared
//! `ToResultCode` mappings for the rich `appletInitialize`
//! error mapping.
//!
//! This module is gated behind the `ffi` + `service-applet` Cargo features.

use nx_sf::error::ToResultCode as _;
use nx_svc::raw::INVALID_HANDLE;

use crate::{
    ffi::common::GENERIC_ERROR,
    services::applet,
};

/// Closes the applet service connection.
///
/// Corresponds to `appletExit()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_exit() {
    applet::exit();
}

/// Gets the current operation mode (handheld/docked).
///
/// Corresponds to `appletGetOperationMode()` in `applet.h`. Returns the cached
/// value populated during init and refreshed by [`applet::process_message`]; no
/// IPC is issued per call.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_get_operation_mode() -> u8 {
    applet::cached_operation_mode() as u8
}

/// Gets the current performance mode.
///
/// Corresponds to `appletGetPerformanceMode()` in `applet.h`. Returns the cached
/// value populated during init and refreshed by [`applet::process_message`].
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_get_performance_mode() -> u32 {
    applet::cached_performance_mode() as u32
}

/// Gets the current focus state.
///
/// Corresponds to `appletGetFocusState()` in `applet.h`. Returns the cached
/// value populated during init and refreshed by [`applet::process_message`].
/// Falls back to `InFocus` when uninitialized to match libnx's
/// default-initialized global.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_get_focus_state() -> u8 {
    applet::cached_focus_state().unwrap_or(nx_service_applet::AppletFocusState::InFocus) as u8
}

/// Gets the message event handle.
///
/// Corresponds to part of `appletGetMessageEventHandle()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_get_message_event_handle() -> u32 {
    // The session's own event, named rather than re-requested: asking the
    // Application Manager again would mint a second handle that neither side
    // closes. The C caller receives the number to wait on, not ownership.
    match applet::message_event_handle() {
        Some(handle) => handle,
        None => INVALID_HANDLE,
    }
}

/// libnx `Event` struct layout, used by `appletGetMessageEvent`.
///
/// Matches `Event` in `libnx/include/switch/kernel/event.h`:
/// ```c
/// typedef struct {
///     Handle revent;     // u32
///     Handle wevent;     // u32
///     bool   autoclear;  // u8
/// } Event;
/// ```
#[repr(C)]
pub struct LibnxEvent {
    revent: u32,
    wevent: u32,
    autoclear: bool,
}

/// Fills a libnx-compatible `Event` struct with the applet message event.
///
/// Corresponds to `appletGetMessageEvent()` in `applet.h`. libnx initialises
/// this event with `autoclear = false`; callers that wait on it must reset the
/// signal manually.
///
/// # Safety
///
/// `out` must point to writable memory at least the size of libnx's `Event`
/// struct (`sizeof(Handle)*2 + sizeof(bool)` ≈ 12 bytes with C padding).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_get_message_event(out: *mut LibnxEvent) -> u32 {
    if out.is_null() {
        return GENERIC_ERROR;
    }

    let Some(handle) = applet::message_event_handle() else {
        return GENERIC_ERROR;
    };

    // SAFETY: caller guarantees `out` is writable.
    unsafe {
        (*out).revent = handle;
        (*out).wevent = INVALID_HANDLE;
        (*out).autoclear = false;
    }
    0
}

/// Sets the focus handling mode.
///
/// Corresponds to `appletSetFocusHandlingMode()` in `applet.h`. AM rejects
/// this command for any role other than `Application`
/// (`LibnxError_NotInitialized` in libnx); the typestate routes it through
/// [`applet::as_application`] so the gate is enforced before the IPC call.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_set_focus_handling_mode(mode: u32) -> u32 {
    let Some(app) = applet::as_application() else {
        return GENERIC_ERROR;
    };

    let mode = match mode {
        0 => nx_service_applet::AppletFocusHandlingMode::SuspendHomeSleep,
        1 => nx_service_applet::AppletFocusHandlingMode::NoSuspend,
        2 => nx_service_applet::AppletFocusHandlingMode::SuspendHomeSleepNotify,
        3 => nx_service_applet::AppletFocusHandlingMode::AlwaysSuspend,
        _ => return GENERIC_ERROR,
    };

    match app.set_focus_handling_mode(mode) {
        Ok(()) => 0,
        Err(err) => err.to_rc(),
    }
}

/// Sets whether to suspend when out of focus.
///
/// Corresponds to `appletSetOutOfFocusSuspendingEnabled()` in `applet.h`. AM
/// rejects this command for any role other than `Application` (HOS 2.0.0+
/// only); the typestate routes it through [`applet::as_application`] so the
/// gate is enforced before the IPC call.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_set_out_of_focus_suspending_enabled(
    enabled: bool,
) -> u32 {
    let Some(app) = applet::as_application() else {
        return GENERIC_ERROR;
    };

    if let Err(err) = app.set_out_of_focus_suspending_enabled(enabled) {
        return err.to_rc();
    }

    0
}

/// Receives a message from the applet message queue.
///
/// Corresponds to `appletReceiveMessage()` in `applet.h`.
///
/// # Safety
///
/// `msg` must point to valid, writable memory for a u32.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_receive_message(msg: *mut u32) -> u32 {
    if msg.is_null() {
        return GENERIC_ERROR;
    }

    let Some(csg) = applet::get_common_state_getter() else {
        return GENERIC_ERROR;
    };

    let result = csg.get().receive_message();
    // Drop the borrow before invoking process_message (which re-acquires the
    // read lock to refresh the cache).
    drop(csg);

    match result {
        Ok(Some(message)) => {
            // SAFETY: Caller guarantees msg points to valid memory.
            unsafe { *msg = message as u32 };
            // Refresh cached state for messages that signal a state change
            // (libnx's `appletProcessMessage` equivalent).
            applet::process_message(message);
            0
        }
        // No message available — propagate libnx's "queue empty" result code so
        // callers (e.g. `appletReceiveMessage` consumers) can distinguish empty
        // from success the same way they would against libnx.
        Ok(None) => 0x680,
        Err(err) => err.to_rc(),
    }
}

/// Sets operation mode change notification.
///
/// Corresponds to `appletSetOperationModeChangedNotification()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_set_operation_mode_changed_notification(
    enabled: bool,
) -> u32 {
    let Some(sc) = applet::get_self_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(err) = sc.get().set_operation_mode_changed_notification(enabled) {
        return err.to_rc();
    }

    0
}

/// Sets performance mode change notification.
///
/// Corresponds to `appletSetPerformanceModeChangedNotification()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_set_performance_mode_changed_notification(
    enabled: bool,
) -> u32 {
    let Some(sc) = applet::get_self_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(err) = sc.get().set_performance_mode_changed_notification(enabled) {
        return err.to_rc();
    }

    0
}

/// Gets the applet resource user ID.
///
/// Corresponds to `appletGetAppletResourceUserId()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_get_applet_resource_user_id() -> u64 {
    applet::get_applet_resource_user_id()
        .map(|a| a.to_raw())
        .unwrap_or(nx_service_applet::aruid::NO_ARUID)
}

/// Acquires foreground rights.
///
/// Corresponds to `appletAcquireForegroundRights()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_acquire_foreground_rights() -> u32 {
    let Some(wc) = applet::get_window_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(err) = wc.get().acquire_foreground_rights() {
        return err.to_rc();
    }

    0
}

/// Creates a managed display layer.
///
/// Corresponds to `appletCreateManagedDisplayLayer()` in `applet.h`.
///
/// # Safety
///
/// `out` must be a valid pointer to write the layer ID.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_create_managed_display_layer(
    out: *mut u64,
) -> u32 {
    let Some(sc) = applet::get_self_controller() else {
        return GENERIC_ERROR;
    };

    match sc.get().create_managed_display_layer() {
        Ok(layer_id) => {
            if !out.is_null() {
                unsafe { *out = layer_id };
            }
            0
        }
        Err(_) => GENERIC_ERROR,
    }
}
