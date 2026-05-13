//! CMIF protocol operations for applet service.
//!
//! This module implements applet service commands using the CMIF protocol.

use core::mem::size_of;

use nx_sf::{
    cmif::ParseResponseError,
    service::{BufferAttr, ConvertToDomainError, DispatchError, Domain, DomainObject},
};
use nx_svc::{process::Handle as ProcessHandle, thread};

use crate::{
    AppletCommonFunctions, AppletProxyService, ApplicationCreator, ApplicationFunctions,
    AudioController, CommonStateGetter, DebugFunctions, DisplayController, GlobalStateController,
    HomeMenuFunctions, LibraryAppletCreator, LibraryAppletSelfAccessor, ProcessWindingController,
    SelfController, WindowController, alias_domain,
    aruid::Aruid,
    proto::{
        AppletAttribute, AppletFocusHandlingMode, AppletType, CMD_AF_NOTIFY_RUNNING,
        CMD_GET_APPLET_COMMON_FUNCTIONS, CMD_GET_APPLET_COMMON_FUNCTIONS_SYSTEM,
        CMD_GET_APPLICATION_CREATOR, CMD_GET_APPLICATION_FUNCTIONS, CMD_GET_AUDIO_CONTROLLER,
        CMD_GET_COMMON_STATE_GETTER, CMD_GET_DEBUG_FUNCTIONS, CMD_GET_DISPLAY_CONTROLLER,
        CMD_GET_FUNCTIONS_OR_SELF_ACCESSOR, CMD_GET_LIBRARY_APPLET_CREATOR,
        CMD_GET_PROCESS_WINDING_CONTROLLER, CMD_GET_SELF_CONTROLLER, CMD_GET_WINDOW_CONTROLLER,
        CMD_OPEN_APPLICATION_PROXY, CMD_OPEN_LIBRARY_APPLET_PROXY,
        CMD_OPEN_LIBRARY_APPLET_PROXY_OLD, CMD_OPEN_OVERLAY_APPLET_PROXY,
        CMD_OPEN_SYSTEM_APPLET_PROXY, CMD_OPEN_SYSTEM_APPLICATION_PROXY,
        CMD_SC_CREATE_MANAGED_DISPLAY_LAYER, CMD_SC_SET_FOCUS_HANDLING_MODE,
        CMD_SC_SET_OPERATION_MODE_CHANGED_NOTIFICATION, CMD_SC_SET_OUT_OF_FOCUS_SUSPENDING_ENABLED,
        CMD_SC_SET_PERFORMANCE_MODE_CHANGED_NOTIFICATION, CMD_WC_ACQUIRE_FOREGROUND_RIGHTS,
        CMD_WC_GET_APPLET_RESOURCE_USER_ID,
    },
};

/// Result code returned by AM when the proxy session is temporarily busy.
///
/// libnx names this `AM_BUSY_ERROR`. The runtime retries (with a 100ms back-off)
/// until the call succeeds or a timeout elapses, matching libnx behaviour.
const AM_BUSY_ERROR: u32 = 0x19280;

/// Back-off between AM-busy retries (libnx uses 100ms).
const AM_BUSY_RETRY_NS: u64 = 100_000_000;

/// Default maximum number of busy retries before [`open_proxy`] gives up.
/// At 100ms each, this gives ~10 seconds of total wait time.
const AM_BUSY_DEFAULT_MAX_RETRIES: u32 = 100;

/// Opens a proxy session for the specified applet type.
///
/// The proxy command varies by applet type:
/// - Application: cmd 0
/// - SystemApplet: cmd 100
/// - LibraryApplet: cmd 200 (or 201 with attributes)
/// - OverlayApplet: cmd 300
/// - SystemApplication: cmd 350
///
/// # AM-busy retries
///
/// AM returns result code `0x19280` while the proxy session is being torn down
/// from a prior process. Mirroring libnx `_appletInitialize`, this function
/// retries up to [`AM_BUSY_DEFAULT_MAX_RETRIES`] times with a 100ms delay between
/// attempts, returning [`OpenProxyError::Timeout`] if AM is still busy after that.
pub fn open_proxy(
    domain: &Domain,
    applet_type: AppletType,
    process_handle: ProcessHandle,
    attr: Option<&AppletAttribute>,
) -> Result<AppletProxyService, OpenProxyError> {
    // Determine command ID based on applet type
    let cmd_id = match applet_type {
        AppletType::Application => CMD_OPEN_APPLICATION_PROXY,
        AppletType::SystemApplet => CMD_OPEN_SYSTEM_APPLET_PROXY,
        AppletType::LibraryApplet => {
            if attr.is_some() {
                CMD_OPEN_LIBRARY_APPLET_PROXY
            } else {
                CMD_OPEN_LIBRARY_APPLET_PROXY_OLD
            }
        }
        AppletType::OverlayApplet => CMD_OPEN_OVERLAY_APPLET_PROXY,
        AppletType::SystemApplication => CMD_OPEN_SYSTEM_APPLICATION_PROXY,
        AppletType::None | AppletType::Default => {
            return Err(OpenProxyError::InvalidAppletType);
        }
    };

    // Input data: u64 reserved = 0
    let reserved: u64 = 0;

    let mut attempts: u32 = 0;
    let result = loop {
        // Dispatch builders are consumed by send(); rebuild each iteration.
        let mut dispatch = domain
            .dispatch(cmd_id)
            .send_pid()
            .in_handle(process_handle.to_raw())
            .out_objects(1);

        // SAFETY: reserved is valid and lives until send() completes.
        dispatch = unsafe { dispatch.in_raw((&raw const reserved).cast::<u8>(), size_of::<u64>()) };

        // Add attribute buffer for LibraryApplet with attributes
        if let Some(attr) = attr {
            dispatch = dispatch.buffer(
                (attr as *const AppletAttribute).cast::<u8>(),
                size_of::<AppletAttribute>(),
                BufferAttr::IN.or(BufferAttr::HIPC_MAP_ALIAS),
            );
        }

        match dispatch.send() {
            Ok(r) => break r,
            Err(DispatchError::ParseResponse(ParseResponseError::ServiceError(AM_BUSY_ERROR))) => {
                attempts += 1;
                if attempts >= AM_BUSY_DEFAULT_MAX_RETRIES {
                    return Err(OpenProxyError::Timeout);
                }
                thread::sleep(AM_BUSY_RETRY_NS);
            }
            Err(err) => return Err(OpenProxyError::Dispatch(err)),
        }
    };

    // Extract the domain object ID for the proxy
    if result.objects.is_empty() {
        return Err(OpenProxyError::MissingObject);
    }

    let object_id = result.objects[0];

    // Build the proxy wrapper. The domain alias shares the parent's kernel
    // handle and the close-on-drop is suppressed; the root [`AppletService`]
    // closes the kernel handle once on its own [`Drop`].
    Ok(AppletProxyService {
        domain: alias_domain(domain),
        object_id,
    })
}

/// Error returned by [`open_proxy`].
#[derive(Debug, thiserror::Error)]
pub enum OpenProxyError {
    /// Invalid applet type (None or Default).
    #[error("invalid applet type")]
    InvalidAppletType,
    /// Failed to dispatch the proxy request.
    #[error("failed to dispatch proxy request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected domain object.
    #[error("missing domain object in response")]
    MissingObject,
    /// AM remained busy (returned `0x19280`) past the retry budget.
    #[error("applet manager remained busy past retry budget")]
    Timeout,
}

/// Gets the ICommonStateGetter sub-interface from the proxy.
pub fn get_common_state_getter(
    proxy: &DomainObject<'_>,
) -> Result<CommonStateGetter, GetCommonStateGetterError> {
    let result = proxy
        .dispatch(CMD_GET_COMMON_STATE_GETTER)
        .out_objects(1)
        .send()
        .map_err(GetCommonStateGetterError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(GetCommonStateGetterError::MissingObject);
    }

    let object_id = result.objects[0];

    Ok(CommonStateGetter {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}

/// Error returned by [`get_common_state_getter`].
#[derive(Debug, thiserror::Error)]
pub enum GetCommonStateGetterError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected domain object.
    #[error("missing domain object in response")]
    MissingObject,
}

/// Gets the ISelfController sub-interface from the proxy.
pub fn get_self_controller(
    proxy: &DomainObject<'_>,
) -> Result<SelfController, GetSelfControllerError> {
    let result = proxy
        .dispatch(CMD_GET_SELF_CONTROLLER)
        .out_objects(1)
        .send()
        .map_err(GetSelfControllerError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(GetSelfControllerError::MissingObject);
    }

    let object_id = result.objects[0];

    Ok(SelfController {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}

/// Error returned by [`get_self_controller`].
#[derive(Debug, thiserror::Error)]
pub enum GetSelfControllerError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected domain object.
    #[error("missing domain object in response")]
    MissingObject,
}

/// Gets the IWindowController sub-interface from the proxy.
pub fn get_window_controller(
    proxy: &DomainObject<'_>,
) -> Result<WindowController, GetWindowControllerError> {
    let result = proxy
        .dispatch(CMD_GET_WINDOW_CONTROLLER)
        .out_objects(1)
        .send()
        .map_err(GetWindowControllerError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(GetWindowControllerError::MissingObject);
    }

    let object_id = result.objects[0];

    Ok(WindowController {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}

/// Error returned by [`get_window_controller`].
#[derive(Debug, thiserror::Error)]
pub enum GetWindowControllerError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected domain object.
    #[error("missing domain object in response")]
    MissingObject,
}

/// Acquires foreground rights via IWindowController.
pub fn acquire_foreground_rights(
    window_controller: &DomainObject<'_>,
) -> Result<(), AcquireForegroundRightsError> {
    window_controller
        .dispatch(CMD_WC_ACQUIRE_FOREGROUND_RIGHTS)
        .send()
        .map_err(AcquireForegroundRightsError::Dispatch)?;

    Ok(())
}

/// Error returned by [`acquire_foreground_rights`].
#[derive(Debug, thiserror::Error)]
pub enum AcquireForegroundRightsError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

/// Sets the focus handling mode on ISelfController.
///
/// Mirrors libnx `appletSetFocusHandlingMode`: translates the high-level mode
/// into the four flags `(notify_in_focus, notify_out_of_focus, suspend_on_background,
/// out_of_focus_suspending_enabled)`, then issues two IPC dispatches:
///
/// - `ISelfController::SetFocusHandlingMode` (cmd 13) with the first three flags
/// - `ISelfController::SetOutOfFocusSuspendingEnabled` (cmd 16) with the fourth flag
///
/// The cmd-16 dispatch is what differentiates `AlwaysSuspend` from `SuspendHomeSleep`
/// (both share the cmd-13 flags `(0,0,1)`); omitting it leaves the always-suspend
/// behavior unconfigured.
///
/// # Version
///
/// Cmd 16 was introduced in HOS 2.0.0; on older firmware the second dispatch will
/// fail. The runtime targets modern HOS so this is unconditional here. Callers that
/// must support <2.0.0 should call cmd 13 and cmd 16 separately and gate the latter.
pub fn set_focus_handling_mode(
    self_controller: &DomainObject<'_>,
    mode: AppletFocusHandlingMode,
) -> Result<(), SetFocusHandlingModeError> {
    // Translate the high-level mode into the four-flag representation libnx uses.
    let (notify_in_focus, notify_out_of_focus, suspend_on_background, out_of_focus_suspending) =
        match mode {
            AppletFocusHandlingMode::SuspendHomeSleep => (false, false, true, false),
            AppletFocusHandlingMode::NoSuspend => (true, true, false, false),
            AppletFocusHandlingMode::SuspendHomeSleepNotify => (true, false, true, false),
            AppletFocusHandlingMode::AlwaysSuspend => (false, false, true, true),
        };

    // cmd 13: SetFocusHandlingMode — three bools
    let input: [u8; 3] = [
        notify_in_focus as u8,
        notify_out_of_focus as u8,
        suspend_on_background as u8,
    ];

    let dispatch = self_controller.dispatch(CMD_SC_SET_FOCUS_HANDLING_MODE);

    // SAFETY: input is valid and lives until send() completes.
    let dispatch = unsafe { dispatch.in_raw(input.as_ptr(), input.len()) };

    dispatch
        .send()
        .map_err(SetFocusHandlingModeError::Dispatch)?;

    // cmd 16: SetOutOfFocusSuspendingEnabled — single bool. Required for AlwaysSuspend
    // semantics; libnx always sends it on HOS 2.0.0+ so the flag does not leak across
    // mode transitions.
    set_out_of_focus_suspending_enabled(self_controller, out_of_focus_suspending)
        .map_err(SetFocusHandlingModeError::SetOutOfFocusSuspending)?;

    Ok(())
}

/// Error returned by [`set_focus_handling_mode`].
#[derive(Debug, thiserror::Error)]
pub enum SetFocusHandlingModeError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Failed to dispatch the companion `SetOutOfFocusSuspendingEnabled` (cmd 16).
    #[error("failed to set out-of-focus suspending")]
    SetOutOfFocusSuspending(#[source] SetOutOfFocusSuspendingEnabledError),
}

/// Sets whether to suspend when out of focus (ISelfController, 2.0.0+).
pub fn set_out_of_focus_suspending_enabled(
    self_controller: &DomainObject<'_>,
    enabled: bool,
) -> Result<(), SetOutOfFocusSuspendingEnabledError> {
    let input: u8 = enabled as u8;

    let dispatch = self_controller.dispatch(CMD_SC_SET_OUT_OF_FOCUS_SUSPENDING_ENABLED);

    // SAFETY: input is valid and lives until send() completes.
    let dispatch = unsafe { dispatch.in_raw((&raw const input).cast::<u8>(), size_of::<u8>()) };

    dispatch
        .send()
        .map_err(SetOutOfFocusSuspendingEnabledError::Dispatch)?;

    Ok(())
}

/// Error returned by [`set_out_of_focus_suspending_enabled`].
#[derive(Debug, thiserror::Error)]
pub enum SetOutOfFocusSuspendingEnabledError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

/// Error returned by [`crate::connect`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to get service from SM.
    #[error("failed to get applet service")]
    GetService(#[source] nx_service_sm::GetServiceCmifError),
    /// Failed to convert service to domain.
    #[error("failed to convert to domain")]
    ConvertToDomain(#[source] ConvertToDomainError),
}

/// Enables operation mode change notifications (ISelfController, cmd 11).
///
/// When enabled, the applet receives `OperationModeChanged` messages
/// when the console transitions between handheld and docked modes.
pub fn set_operation_mode_changed_notification(
    self_controller: &DomainObject<'_>,
    enabled: bool,
) -> Result<(), SetOperationModeChangedNotificationError> {
    let input: u8 = enabled as u8;

    let dispatch = self_controller.dispatch(CMD_SC_SET_OPERATION_MODE_CHANGED_NOTIFICATION);

    // SAFETY: input is valid and lives until send() completes.
    let dispatch = unsafe { dispatch.in_raw((&raw const input).cast::<u8>(), size_of::<u8>()) };

    dispatch
        .send()
        .map_err(SetOperationModeChangedNotificationError::Dispatch)?;

    Ok(())
}

/// Error returned by [`set_operation_mode_changed_notification`].
#[derive(Debug, thiserror::Error)]
pub enum SetOperationModeChangedNotificationError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

/// Enables performance mode change notifications (ISelfController, cmd 12).
///
/// When enabled, the applet receives `PerformanceModeChanged` messages
/// when CPU/GPU clock speeds change.
pub fn set_performance_mode_changed_notification(
    self_controller: &DomainObject<'_>,
    enabled: bool,
) -> Result<(), SetPerformanceModeChangedNotificationError> {
    let input: u8 = enabled as u8;

    let dispatch = self_controller.dispatch(CMD_SC_SET_PERFORMANCE_MODE_CHANGED_NOTIFICATION);

    // SAFETY: input is valid and lives until send() completes.
    let dispatch = unsafe { dispatch.in_raw((&raw const input).cast::<u8>(), size_of::<u8>()) };

    dispatch
        .send()
        .map_err(SetPerformanceModeChangedNotificationError::Dispatch)?;

    Ok(())
}

/// Error returned by [`set_performance_mode_changed_notification`].
#[derive(Debug, thiserror::Error)]
pub enum SetPerformanceModeChangedNotificationError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

/// Gets the applet resource user ID (IWindowController, cmd 1).
///
/// This ID is used by various system services (HID, audio, etc.) to identify
/// the applet. It's obtained during applet initialization and stored globally.
///
/// Returns `Ok(None)` if the system returns ARUID 0 (invalid).
pub fn get_applet_resource_user_id(
    window_controller: &DomainObject<'_>,
) -> Result<Option<Aruid>, GetAppletResourceUserIdError> {
    let result = window_controller
        .dispatch(CMD_WC_GET_APPLET_RESOURCE_USER_ID)
        .out_size(size_of::<u64>())
        .send()
        .map_err(GetAppletResourceUserIdError::Dispatch)?;

    if result.data.len() < size_of::<u64>() {
        return Err(GetAppletResourceUserIdError::InvalidResponse);
    }

    // SAFETY: Response data contains u64 applet resource user ID.
    let raw = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(Aruid::new(raw))
}

/// Error returned by [`get_applet_resource_user_id`].
#[derive(Debug, thiserror::Error)]
pub enum GetAppletResourceUserIdError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
}

/// Gets the IApplicationFunctions sub-interface from the proxy (Application type only).
pub fn get_application_functions(
    proxy: &DomainObject<'_>,
) -> Result<ApplicationFunctions, GetApplicationFunctionsError> {
    let result = proxy
        .dispatch(CMD_GET_APPLICATION_FUNCTIONS)
        .out_objects(1)
        .send()
        .map_err(GetApplicationFunctionsError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(GetApplicationFunctionsError::MissingObject);
    }

    let object_id = result.objects[0];

    Ok(ApplicationFunctions {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}

/// Error returned by [`get_application_functions`].
#[derive(Debug, thiserror::Error)]
pub enum GetApplicationFunctionsError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected domain object.
    #[error("missing domain object in response")]
    MissingObject,
}

/// Notifies the system that the application has completed initialization (IApplicationFunctions).
///
/// This should be called after waiting for InFocus state, acquiring foreground rights,
/// and setting up focus handling mode.
pub fn notify_running(app_funcs: &DomainObject<'_>) -> Result<bool, NotifyRunningError> {
    let result = app_funcs
        .dispatch(CMD_AF_NOTIFY_RUNNING)
        .out_size(size_of::<u8>())
        .send()
        .map_err(NotifyRunningError::Dispatch)?;

    if result.data.is_empty() {
        return Err(NotifyRunningError::InvalidResponse);
    }

    // SAFETY: Response data contains a bool (u8).
    let can_continue = unsafe { *result.data.as_ptr() != 0 };
    Ok(can_continue)
}

/// Error returned by [`notify_running`].
#[derive(Debug, thiserror::Error)]
pub enum NotifyRunningError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
}

/// Creates a managed display layer (ISelfController, cmd 40).
pub fn create_managed_display_layer(
    self_controller: &DomainObject<'_>,
) -> Result<u64, CreateManagedDisplayLayerError> {
    let result = self_controller
        .dispatch(CMD_SC_CREATE_MANAGED_DISPLAY_LAYER)
        .out_size(size_of::<u64>())
        .send()
        .map_err(CreateManagedDisplayLayerError::Dispatch)?;

    if result.data.len() < size_of::<u64>() {
        return Err(CreateManagedDisplayLayerError::InvalidResponse);
    }

    // SAFETY: Response data contains u64 layer ID.
    let layer_id = unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<u64>()) };

    Ok(layer_id)
}

/// Error returned by [`create_managed_display_layer`].
#[derive(Debug, thiserror::Error)]
pub enum CreateManagedDisplayLayerError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response data was invalid.
    #[error("invalid response data")]
    InvalidResponse,
}

/// Shared error returned by the generic sub-interface getters.
///
/// Used by every `get_*` method on [`AppletProxyService`] except the four
/// originally-implemented ones (`get_common_state_getter`, `get_self_controller`,
/// `get_window_controller`, `get_application_functions`) which retain their
/// dedicated error types for backward compatibility.
#[derive(Debug, thiserror::Error)]
pub enum GetSubInterfaceError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
    /// Response did not contain the expected domain object.
    #[error("missing domain object in response")]
    MissingObject,
}

/// Generic helper: dispatches `cmd_id` and returns the raw object id.
fn get_sub_interface_object_id(
    proxy: &DomainObject<'_>,
    cmd_id: u32,
) -> Result<u32, GetSubInterfaceError> {
    let result = proxy
        .dispatch(cmd_id)
        .out_objects(1)
        .send()
        .map_err(GetSubInterfaceError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(GetSubInterfaceError::MissingObject);
    }

    Ok(result.objects[0])
}

/// Gets IAudioController (cmd 3).
pub fn get_audio_controller(
    proxy: &DomainObject<'_>,
) -> Result<AudioController, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_AUDIO_CONTROLLER).map(|object_id| AudioController {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}

/// Gets IDisplayController (cmd 4).
pub fn get_display_controller(
    proxy: &DomainObject<'_>,
) -> Result<DisplayController, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_DISPLAY_CONTROLLER).map(|object_id| {
        DisplayController {
            domain: alias_domain(proxy.domain()),
            object_id,
        }
    })
}

/// Gets IProcessWindingController (cmd 10, LibraryApplet only).
pub fn get_process_winding_controller(
    proxy: &DomainObject<'_>,
) -> Result<ProcessWindingController, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_PROCESS_WINDING_CONTROLLER).map(|object_id| {
        ProcessWindingController {
            domain: alias_domain(proxy.domain()),
            object_id,
        }
    })
}

/// Gets ILibraryAppletCreator (cmd 11).
pub fn get_library_applet_creator(
    proxy: &DomainObject<'_>,
) -> Result<LibraryAppletCreator, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_LIBRARY_APPLET_CREATOR).map(|object_id| {
        LibraryAppletCreator {
            domain: alias_domain(proxy.domain()),
            object_id,
        }
    })
}

/// Gets ILibraryAppletSelfAccessor (cmd 20, LibraryApplet pre-15.0.0).
pub fn get_library_applet_self_accessor(
    proxy: &DomainObject<'_>,
) -> Result<LibraryAppletSelfAccessor, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_FUNCTIONS_OR_SELF_ACCESSOR).map(|object_id| {
        LibraryAppletSelfAccessor {
            domain: alias_domain(proxy.domain()),
            object_id,
        }
    })
}

/// Gets IAppletCommonFunctions (cmd 21 for non-SystemApplet, cmd 23 for SystemApplet).
///
/// HOS 7.0.0+. Returns `MissingObject` when called for an unsupported applet type.
pub fn get_applet_common_functions(
    proxy: &DomainObject<'_>,
    applet_type: AppletType,
) -> Result<AppletCommonFunctions, GetSubInterfaceError> {
    let cmd_id = match applet_type {
        AppletType::SystemApplet => CMD_GET_APPLET_COMMON_FUNCTIONS_SYSTEM,
        AppletType::LibraryApplet | AppletType::OverlayApplet => CMD_GET_APPLET_COMMON_FUNCTIONS,
        _ => return Err(GetSubInterfaceError::MissingObject),
    };
    get_sub_interface_object_id(proxy, cmd_id).map(|object_id| AppletCommonFunctions {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}

/// Gets IGlobalStateController (cmd 21 for SystemApplet, cmd 23 for
/// LibraryApplet/OverlayApplet on HOS 15.0.0+).
pub fn get_global_state_controller(
    proxy: &DomainObject<'_>,
    applet_type: AppletType,
) -> Result<GlobalStateController, GetSubInterfaceError> {
    let cmd_id = match applet_type {
        AppletType::SystemApplet => CMD_GET_APPLET_COMMON_FUNCTIONS,
        AppletType::LibraryApplet | AppletType::OverlayApplet => {
            CMD_GET_APPLET_COMMON_FUNCTIONS_SYSTEM
        }
        _ => return Err(GetSubInterfaceError::MissingObject),
    };
    get_sub_interface_object_id(proxy, cmd_id).map(|object_id| GlobalStateController {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}

/// Gets IApplicationCreator (cmd 22, SystemApplet only).
pub fn get_application_creator(
    proxy: &DomainObject<'_>,
) -> Result<ApplicationCreator, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_APPLICATION_CREATOR).map(|object_id| {
        ApplicationCreator {
            domain: alias_domain(proxy.domain()),
            object_id,
        }
    })
}

/// Gets IHomeMenuFunctions (cmd 22, LibraryApplet on HOS 15.0.0+).
pub fn get_home_menu_functions(
    proxy: &DomainObject<'_>,
) -> Result<HomeMenuFunctions, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_APPLICATION_CREATOR).map(|object_id| {
        HomeMenuFunctions {
            domain: alias_domain(proxy.domain()),
            object_id,
        }
    })
}

/// Gets IDebugFunctions (cmd 1000).
pub fn get_debug_functions(
    proxy: &DomainObject<'_>,
) -> Result<DebugFunctions, GetSubInterfaceError> {
    get_sub_interface_object_id(proxy, CMD_GET_DEBUG_FUNCTIONS).map(|object_id| DebugFunctions {
        domain: alias_domain(proxy.domain()),
        object_id,
    })
}
