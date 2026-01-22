//! Applet service FFI

use nx_sf::cmif;
use nx_svc::{process::Handle as ProcessHandle, raw::INVALID_HANDLE};

use super::{
    common::{GENERIC_ERROR, convert_to_domain_error_to_rc, dispatch_error_to_rc},
    env::get_applet_type,
};
use crate::{applet_manager, env};

/// Initializes the applet service. Returns 0 on success, error code on failure.
///
/// Corresponds to `appletInitialize()` in `applet.h`.
///
/// # Safety
///
/// SM must be initialized before calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_initialize() -> u32 {
    // Get applet type from global variable (set by envSetup)
    let raw_type = get_applet_type();

    let Some(applet_type) = nx_service_applet::AppletType::from_raw(raw_type as i32) else {
        return GENERIC_ERROR;
    };

    // Skip initialization for AppletType::None
    if matches!(applet_type, nx_service_applet::AppletType::None) {
        return 0;
    }

    // Resolve Default to Application
    let applet_type = if matches!(applet_type, nx_service_applet::AppletType::Default) {
        nx_service_applet::AppletType::Application
    } else {
        applet_type
    };

    // Get process handle
    let process_handle = env::own_process_handle()
        .map(|h| {
            // SAFETY: Handle from env::own_process_handle() is guaranteed valid.
            unsafe { ProcessHandle::from_raw(h.to_raw()) }
        })
        .unwrap_or_else(ProcessHandle::current_process);

    if let Err(err) = applet_manager::init(applet_type, process_handle) {
        return applet_connect_error_to_rc(err);
    }

    0
}

/// Closes the applet service connection.
///
/// Corresponds to `appletExit()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_exit() {
    applet_manager::exit();
}

/// Gets the current applet type.
///
/// Corresponds to `appletGetAppletType()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_applet_type() -> i32 {
    // Return from the global variable
    get_applet_type() as i32
}

/// Gets the current operation mode (handheld/docked).
///
/// Corresponds to `appletGetOperationMode()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_operation_mode() -> u8 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return nx_service_applet::AppletOperationMode::Handheld as u8;
    };

    csg.get_operation_mode()
        .unwrap_or(nx_service_applet::AppletOperationMode::Handheld) as u8
}

/// Gets the current performance mode.
///
/// Corresponds to `appletGetPerformanceMode()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_performance_mode() -> u32 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return 0;
    };

    csg.get_performance_mode().unwrap_or(0)
}

/// Gets the current focus state.
///
/// Corresponds to `appletGetFocusState()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_focus_state() -> u8 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return nx_service_applet::AppletFocusState::InFocus as u8;
    };

    csg.get_current_focus_state()
        .map(|s| s as u8)
        .unwrap_or(nx_service_applet::AppletFocusState::InFocus as u8)
}

/// Gets the message event handle.
///
/// Corresponds to part of `appletGetMessageEventHandle()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_get_message_event_handle() -> u32 {
    let Some(csg) = applet_manager::get_common_state_getter() else {
        return INVALID_HANDLE;
    };

    match csg.get_event_handle() {
        Ok(handle) => handle.to_raw(),
        Err(_) => INVALID_HANDLE,
    }
}

/// Sets the focus handling mode.
///
/// Corresponds to `appletSetFocusHandlingMode()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_set_focus_handling_mode(mode: u32) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
        return GENERIC_ERROR;
    };

    let mode = match mode {
        0 => nx_service_applet::AppletFocusHandlingMode::SuspendHomeSleep,
        1 => nx_service_applet::AppletFocusHandlingMode::NoSuspend,
        2 => nx_service_applet::AppletFocusHandlingMode::SuspendHomeSleepNotify,
        3 => nx_service_applet::AppletFocusHandlingMode::AlwaysSuspend,
        _ => return GENERIC_ERROR,
    };

    if let Err(nx_service_applet::SetFocusHandlingModeError::Dispatch(e)) =
        sc.set_focus_handling_mode(mode)
    {
        return dispatch_error_to_rc(e);
    }

    0
}

/// Sets whether to suspend when out of focus.
///
/// Corresponds to `appletSetOutOfFocusSuspendingEnabled()` in `applet.h`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rt__applet_set_out_of_focus_suspending_enabled(enabled: bool) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
        return GENERIC_ERROR;
    };

    if let Err(nx_service_applet::SetOutOfFocusSuspendingEnabledError::Dispatch(e)) =
        sc.set_out_of_focus_suspending_enabled(enabled)
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
pub unsafe extern "C" fn __nx_rt__applet_receive_message(msg: *mut u32) -> u32 {
    if msg.is_null() {
        return GENERIC_ERROR;
    }

    let Some(csg) = applet_manager::get_common_state_getter() else {
        return GENERIC_ERROR;
    };

    match csg.receive_message() {
        Ok(Some(message)) => {
            // SAFETY: Caller guarantees msg points to valid memory.
            unsafe { *msg = message as u32 };
            0
        }
        Ok(None) => {
            // No message available - return 0 with no message written
            // This matches libnx behavior where the queue may be empty
            0
        }
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
pub unsafe extern "C" fn __nx_rt__applet_set_operation_mode_changed_notification(
    enabled: bool,
) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
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
pub unsafe extern "C" fn __nx_rt__applet_set_performance_mode_changed_notification(
    enabled: bool,
) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
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
pub unsafe extern "C" fn __nx_rt__applet_get_applet_resource_user_id() -> u64 {
    applet_manager::get_applet_resource_user_id()
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
pub unsafe extern "C" fn __nx_rt__applet_acquire_foreground_rights() -> u32 {
    let Some(wc) = applet_manager::get_window_controller() else {
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
pub unsafe extern "C" fn __nx_rt__applet_create_managed_display_layer(out: *mut u64) -> u32 {
    let Some(sc) = applet_manager::get_self_controller() else {
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

fn applet_connect_error_to_rc(err: applet_manager::ConnectError) -> u32 {
    use nx_svc::error::ToRawResultCode;

    match err {
        applet_manager::ConnectError::Connect(e) => match e {
            nx_service_applet::ConnectError::GetService(e) => match e {
                nx_service_sm::GetServiceCmifError::SendRequest(e) => e.to_rc(),
                nx_service_sm::GetServiceCmifError::ParseResponse(e) => match e {
                    cmif::ParseResponseError::InvalidMagic => GENERIC_ERROR,
                    cmif::ParseResponseError::ServiceError(code) => code,
                },
                nx_service_sm::GetServiceCmifError::MissingHandle => GENERIC_ERROR,
            },
            nx_service_applet::ConnectError::ConvertToDomain(e) => {
                convert_to_domain_error_to_rc(e.0)
            }
        },
        applet_manager::ConnectError::OpenProxy(e) => match e {
            nx_service_applet::OpenProxyError::InvalidAppletType => GENERIC_ERROR,
            nx_service_applet::OpenProxyError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::OpenProxyError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetCommonStateGetter(e) => match e {
            nx_service_applet::GetCommonStateGetterError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetCommonStateGetterError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetSelfController(e) => match e {
            nx_service_applet::GetSelfControllerError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetSelfControllerError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetWindowController(e) => match e {
            nx_service_applet::GetWindowControllerError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetWindowControllerError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetApplicationFunctions(e) => match e {
            nx_service_applet::GetApplicationFunctionsError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetApplicationFunctionsError::MissingObject => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetEventHandle(e) => match e {
            nx_service_applet::GetEventHandleError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetEventHandleError::MissingHandle => GENERIC_ERROR,
        },
        applet_manager::ConnectError::GetFocusState(e) => match e {
            nx_service_applet::GetCurrentFocusStateError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::GetCurrentFocusStateError::InvalidResponse => GENERIC_ERROR,
            nx_service_applet::GetCurrentFocusStateError::InvalidValue(_) => GENERIC_ERROR,
        },
        applet_manager::ConnectError::WaitSynchronization(e) => e.to_rc(),
        applet_manager::ConnectError::AcquireForegroundRights(e) => match e {
            nx_service_applet::AcquireForegroundRightsError::Dispatch(e) => dispatch_error_to_rc(e),
        },
        applet_manager::ConnectError::SetFocusHandlingMode(e) => match e {
            nx_service_applet::SetFocusHandlingModeError::Dispatch(e) => dispatch_error_to_rc(e),
        },
        applet_manager::ConnectError::NotifyRunning(e) => match e {
            nx_service_applet::NotifyRunningError::Dispatch(e) => dispatch_error_to_rc(e),
            nx_service_applet::NotifyRunningError::InvalidResponse => GENERIC_ERROR,
        },
        applet_manager::ConnectError::SetOperationModeNotification(e) => match e {
            nx_service_applet::SetOperationModeChangedNotificationError::Dispatch(e) => {
                dispatch_error_to_rc(e)
            }
        },
        applet_manager::ConnectError::SetPerformanceModeNotification(e) => match e {
            nx_service_applet::SetPerformanceModeChangedNotificationError::Dispatch(e) => {
                dispatch_error_to_rc(e)
            }
        },
    }
}
