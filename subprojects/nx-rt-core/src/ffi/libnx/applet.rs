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
//! [`applet_connect_error_to_rc`] converter for the rich `appletInitialize`
//! error mapping.
//!
//! This module is gated behind the `ffi` + `service-applet` Cargo features.

use nx_svc::raw::INVALID_HANDLE;

use crate::{
    ffi::common::{
        GENERIC_ERROR, convert_to_domain_error_to_rc, dispatch_error_to_rc, parse_resp_error_to_rc,
        send_error_to_rc,
    },
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
    let Some(csg) = applet::get_common_state_getter() else {
        return INVALID_HANDLE;
    };

    match csg.get_event_handle() {
        Ok(handle) => handle.to_raw(),
        Err(_) => INVALID_HANDLE,
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

    let Some(csg) = applet::get_common_state_getter() else {
        return GENERIC_ERROR;
    };

    match csg.get_event_handle() {
        Ok(handle) => {
            // SAFETY: caller guarantees `out` is writable.
            unsafe {
                (*out).revent = handle.to_raw();
                (*out).wevent = INVALID_HANDLE;
                (*out).autoclear = false;
            }
            0
        }
        Err(_) => GENERIC_ERROR,
    }
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
        Err(nx_service_applet::SetFocusHandlingModeError::Dispatch(e)) => dispatch_error_to_rc(e),
        Err(nx_service_applet::SetFocusHandlingModeError::SetOutOfFocusSuspending(
            nx_service_applet::SetOutOfFocusSuspendingEnabledError::Dispatch(e),
        )) => dispatch_error_to_rc(e),
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

    if let Err(nx_service_applet::SetOutOfFocusSuspendingEnabledError::Dispatch(e)) =
        app.set_out_of_focus_suspending_enabled(enabled)
    {
        return dispatch_error_to_rc(e);
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

    let result = csg.receive_message();
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
        Err(nx_service_applet::ReceiveMessageError::Dispatch(e)) => dispatch_error_to_rc(e),
        Err(nx_service_applet::ReceiveMessageError::InvalidResponse) => GENERIC_ERROR,
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

    if let Err(nx_service_applet::SetOperationModeChangedNotificationError::Dispatch(e)) =
        sc.set_operation_mode_changed_notification(enabled)
    {
        return dispatch_error_to_rc(e);
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

    if let Err(nx_service_applet::SetPerformanceModeChangedNotificationError::Dispatch(e)) =
        sc.set_performance_mode_changed_notification(enabled)
    {
        return dispatch_error_to_rc(e);
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

    if let Err(nx_service_applet::AcquireForegroundRightsError::Dispatch(e)) =
        wc.acquire_foreground_rights()
    {
        return dispatch_error_to_rc(e);
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

    match sc.create_managed_display_layer() {
        Ok(layer_id) => {
            if !out.is_null() {
                unsafe { *out = layer_id };
            }
            0
        }
        Err(_) => GENERIC_ERROR,
    }
}

/// Maps an applet [`ConnectError`](applet::ConnectError) to a raw libnx result
/// code.
///
/// The applet-init handshake is kind-agnostic, but each entry crate owns its
/// own `appletInitialize` (it sources the applet-type value). Both reuse this
/// converter so the rich error is reported faithfully rather than collapsed to
/// [`GENERIC_ERROR`].
pub fn applet_connect_error_to_rc(err: applet::ConnectError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        applet::ConnectError::Open(e) => open_error_to_rc(e),
        applet::ConnectError::GetEventHandle(e) => match e {
            nx_service_applet::GetEventHandleError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetEventHandleError::MissingHandle => GENERIC_ERROR,
        },
        applet::ConnectError::GetFocusState(e) => match e {
            nx_service_applet::GetCurrentFocusStateError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetCurrentFocusStateError::InvalidResponse => GENERIC_ERROR,
            nx_service_applet::GetCurrentFocusStateError::InvalidValue(_) => GENERIC_ERROR,
        },
        applet::ConnectError::GetOperationMode(e) => match e {
            nx_service_applet::GetOperationModeError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetOperationModeError::InvalidResponse => GENERIC_ERROR,
            nx_service_applet::GetOperationModeError::InvalidValue(_) => GENERIC_ERROR,
        },
        applet::ConnectError::GetPerformanceMode(e) => match e {
            nx_service_applet::GetPerformanceModeError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetPerformanceModeError::InvalidResponse => GENERIC_ERROR,
            nx_service_applet::GetPerformanceModeError::InvalidValue(_) => GENERIC_ERROR,
        },
        applet::ConnectError::WaitSynchronization(e) => e.to_rc(),
        applet::ConnectError::AcquireForegroundRights(e) => match e {
            nx_service_applet::AcquireForegroundRightsError::Dispatch(e) => dispatch_error_to_rc(e),
        },
        applet::ConnectError::SetFocusHandlingMode(e) => match e {
            nx_service_applet::SetFocusHandlingModeError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::SetFocusHandlingModeError::SetOutOfFocusSuspending(
                nx_service_applet::SetOutOfFocusSuspendingEnabledError::Dispatch(e),
            ) => dispatch_error_to_rc(e),
        },
        applet::ConnectError::NotifyRunning(e) => match e {
            nx_service_applet::NotifyRunningError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::NotifyRunningError::InvalidResponse => GENERIC_ERROR,
        },
        applet::ConnectError::SetOperationModeNotification(e) => match e {
            nx_service_applet::SetOperationModeChangedNotificationError::Dispatch(e) => {
                dispatch_error_to_rc(e)
            }
        },
        applet::ConnectError::SetPerformanceModeNotification(e) => match e {
            nx_service_applet::SetPerformanceModeChangedNotificationError::Dispatch(e) => {
                dispatch_error_to_rc(e)
            }
        },
    }
}

fn open_error_to_rc(err: nx_service_applet::proxy::OpenError) -> u32 {
    use nx_service_applet::proxy::OpenError;

    match err {
        OpenError::Connect(e) => match e {
            nx_service_applet::ConnectError::GetService(e) => match e {
                nx_service_sm::GetServiceCmifError::SendRequest(e) => send_error_to_rc(e),
                nx_service_sm::GetServiceCmifError::ParseResponse(e) => parse_resp_error_to_rc(e),
                nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
            },
            nx_service_applet::ConnectError::ConvertToDomain(e) => convert_to_domain_error_to_rc(e),
        },
        OpenError::NoneAppletType => GENERIC_ERROR,
        OpenError::OpenProxy(e) => match e {
            nx_service_applet::OpenProxyError::InvalidAppletType => GENERIC_ERROR,
            nx_service_applet::OpenProxyError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::OpenProxyError::MissingObject => GENERIC_ERROR,
            nx_service_applet::OpenProxyError::Timeout => GENERIC_ERROR,
        },
        OpenError::GetCommonStateGetter(e) => match e {
            nx_service_applet::GetCommonStateGetterError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetCommonStateGetterError::MissingObject => GENERIC_ERROR,
        },
        OpenError::GetSelfController(e) => match e {
            nx_service_applet::GetSelfControllerError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetSelfControllerError::MissingObject => GENERIC_ERROR,
        },
        OpenError::GetWindowController(e) => match e {
            nx_service_applet::GetWindowControllerError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetWindowControllerError::MissingObject => GENERIC_ERROR,
        },
        OpenError::GetSubInterface(e) => match e {
            nx_service_applet::GetSubInterfaceError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetSubInterfaceError::MissingObject => GENERIC_ERROR,
        },
        OpenError::DrainExtras(e) => match e {
            nx_service_applet::role::DrainExtrasError::GetApplicationFunctions(e) => match e {
                nx_service_applet::GetApplicationFunctionsError::Dispatch(e) => {
                    dispatch_error_to_rc(e)
                }
                nx_service_applet::GetApplicationFunctionsError::MissingObject => GENERIC_ERROR,
            },
            nx_service_applet::role::DrainExtrasError::GetSubInterface(e) => match e {
                nx_service_applet::GetSubInterfaceError::Dispatch(e) => dispatch_error_to_rc(e),
                nx_service_applet::GetSubInterfaceError::MissingObject => GENERIC_ERROR,
            },
        },
    }
}
