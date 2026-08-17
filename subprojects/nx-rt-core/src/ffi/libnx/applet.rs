//! Kind-agnostic Application Manager (applet) FFI.
//!
//! Redirects the role-independent `libnx` `applet*` accessor surface: the
//! operation/performance/focus-mode queries, the message-event pair, the
//! notification setters, the ARUID and managed-display-layer calls, and
//! `appletExit`: to `nx-rt-core`. Every Switch executable kind performs the
//! same calls against the runtime applet singleton in
//! [`crate::services::applet`]; none of them read the applet-type value, so
//! the single authoritative implementation lives here and both the NRO and
//! NSO entry crates expose the full ABI from it.
//!
//! The applet-type-sourcing entry points, `appletInitialize` and
//! `appletGetAppletType`, are **not** here. They are kind-specific (an NRO
//! reads the applet type from the loader configuration, an NSO from a
//! build-time selection), so each entry crate owns them and reuses the shared
//! `ToResultCode` mappings for the rich `appletInitialize`
//! error mapping.
//!
//! This module is gated behind the `ffi` + `service-applet` Cargo features.

use nx_sf::error::{
    LibnxError,
    ToResultCode as _,
    libnx_error,
};
use nx_svc::raw::INVALID_HANDLE;

use crate::{
    env::hos_version::{
        self,
        HosVersion,
    },
    error::ToResultCode as _,
    ffi::common::GENERIC_ERROR,
    services::applet,
};

/// The result code the Application Manager reports for an empty message queue.
///
/// A caller testing for "nothing waiting" compares against this, so the two
/// entry points that can find an empty queue report the same value rather than
/// each spelling it out.
const NO_MESSAGE: u32 = 0x680;

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
            // Acting on the message is what refreshes the cached state the
            // accessors hand back. The verdict it also produces is dropped here
            // because this entry point has nowhere to put it: its C signature
            // reports a result and a message and nothing else. Whoever pumps
            // messages is the one that has to stop, and it learns to from
            // `appletMainLoop`, not from here.
            let _ = applet::process_message(message);
            0
        }
        Ok(None) => NO_MESSAGE,
        Err(err) => err.to_rc(),
    }
}

/// Takes one pass of a program's message pump, and reports whether it lives on.
///
/// Corresponds to `appletMainLoop()` in `applet.h`. Returns `false` once the
/// system has asked the program to close, which is what ends a
/// `while (appletMainLoop())`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_main_loop() -> bool {
    match applet::main_loop() {
        applet::MainLoop::Continue => true,
        applet::MainLoop::Exit => false,
    }
}

/// Acts on a message the system posted, and reports whether the program lives on.
///
/// Corresponds to `appletProcessMessage()` in `applet.h`.
///
/// # Hooks are not called
///
/// The C implementation this replaces also dispatched the `appletHook`
/// callbacks a program may have registered. This does not, and a program that
/// registered one is not told: the registration call still succeeds and the
/// callback never runs. Nothing here can restore that half — the C
/// implementation is replaced whole or not at all.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_process_message(msg: u32) -> bool {
    let Some(message) = nx_service_applet::AppletMessage::from_raw(msg) else {
        // The queue hands over whatever it holds, including values this
        // runtime attaches no meaning to. Acting on nothing is not a reason to
        // stop, so such a message leaves the program running.
        return true;
    };

    match applet::process_message(message) {
        applet::MainLoop::Continue => true,
        applet::MainLoop::Exit => false,
    }
}

/// Takes the next message the system has posted, if one is waiting.
///
/// Corresponds to `appletGetMessage()` in `applet.h`. Where that aborted the
/// process when the queue refused to hand over a message it had announced, this
/// reports the refusal: nothing is consumed by a refused request, so the caller
/// loses one pass of its loop rather than the program.
///
/// # Safety
///
/// `msg` must be null or point to writable memory for a `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_get_message(msg: *mut u32) -> u32 {
    if msg.is_null() {
        return libnx_error(LibnxError::BadInput);
    }

    match applet::poll_message() {
        Ok(Some(message)) => {
            // SAFETY: `msg` was tested for null above, and the caller guarantees
            // a non-null one points to memory writable for a `u32`, which is
            // what is written through it here. The pointer is not retained.
            unsafe { *msg = message as u32 };
            0
        }
        // Nothing was waiting, which is what most passes of a loop find. The
        // poll does not say which half found nothing — the event or the queue
        // behind it — and a caller has no use for the difference, so both
        // arrive as the code an empty queue reports.
        Ok(None) => NO_MESSAGE,
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

/// Sets whether the console's automatic sleep is disabled.
///
/// Corresponds to `appletSetAutoSleepDisabled()` in `applet.h`. The command does
/// not exist before HOS 5.0.0, so on older firmware the call is refused here
/// rather than sent to a system that has nothing to answer it with.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt_core__libnx_applet_set_auto_sleep_disabled(disabled: bool) -> u32 {
    // The command was introduced by a firmware and simply does not exist before
    // it, so what decides here is whether the system has it to answer with. The
    // version is a run-constant the entry crate stores once during environment
    // setup, so nothing it is compared against could have moved since.
    if hos_version::get() < HosVersion::new(5, 0, 0) {
        return libnx_error(LibnxError::IncompatSysVer);
    }

    let Some(sc) = applet::get_self_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(err) = sc.get().set_auto_sleep_disabled(disabled) {
        return err.to_rc();
    }

    0
}
