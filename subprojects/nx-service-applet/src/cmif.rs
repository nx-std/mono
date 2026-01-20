//! CMIF protocol operations for applet service.
//!
//! This module implements applet service commands using the CMIF protocol.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Service, ServiceConvertToDomainError};
use nx_svc::process::Handle as ProcessHandle;

use crate::{
    AppletProxyService, ApplicationFunctions, CommonStateGetter, SelfController, WindowController,
    aruid::Aruid,
    proto::{
        AppletAttribute, AppletFocusHandlingMode, AppletType, CMD_AF_NOTIFY_RUNNING,
        CMD_GET_APPLICATION_FUNCTIONS, CMD_GET_COMMON_STATE_GETTER, CMD_GET_SELF_CONTROLLER,
        CMD_GET_WINDOW_CONTROLLER, CMD_OPEN_APPLICATION_PROXY, CMD_OPEN_LIBRARY_APPLET_PROXY,
        CMD_OPEN_LIBRARY_APPLET_PROXY_OLD, CMD_OPEN_OVERLAY_APPLET_PROXY,
        CMD_OPEN_SYSTEM_APPLET_PROXY, CMD_OPEN_SYSTEM_APPLICATION_PROXY,
        CMD_SC_CREATE_MANAGED_DISPLAY_LAYER, CMD_SC_SET_FOCUS_HANDLING_MODE,
        CMD_SC_SET_OPERATION_MODE_CHANGED_NOTIFICATION, CMD_SC_SET_OUT_OF_FOCUS_SUSPENDING_ENABLED,
        CMD_SC_SET_PERFORMANCE_MODE_CHANGED_NOTIFICATION, CMD_WC_ACQUIRE_FOREGROUND_RIGHTS,
        CMD_WC_GET_APPLET_RESOURCE_USER_ID,
    },
};

/// Opens a proxy session for the specified applet type.
///
/// The proxy command varies by applet type:
/// - Application: cmd 0
/// - SystemApplet: cmd 100
/// - LibraryApplet: cmd 200 (or 201 with attributes)
/// - OverlayApplet: cmd 300
/// - SystemApplication: cmd 350
pub fn open_proxy(
    service: &Service,
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
                CMD_OPEN_LIBRARY_APPLET_PROXY_OLD
            } else {
                CMD_OPEN_LIBRARY_APPLET_PROXY
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

    // Build dispatch
    let mut dispatch = service
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

    let result = dispatch.send().map_err(OpenProxyError::Dispatch)?;

    // Extract the domain object ID for the proxy
    if result.objects.is_empty() {
        return Err(OpenProxyError::MissingObject);
    }

    let object_id = result.objects[0];

    // Create proxy as domain subservice by constructing Service directly.
    // This shares the parent's session handle with a distinct object_id.
    let proxy_service = Service {
        session: service.session,
        own_handle: 0, // Subservice doesn't own the handle
        object_id,
        pointer_buffer_size: service.pointer_buffer_size,
    };

    Ok(AppletProxyService(proxy_service))
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
}

/// Gets the ICommonStateGetter sub-interface from the proxy.
pub fn get_common_state_getter(
    proxy: &Service,
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

    // Create sub-interface as domain subservice
    let service = Service {
        session: proxy.session,
        own_handle: 0,
        object_id,
        pointer_buffer_size: proxy.pointer_buffer_size,
    };

    Ok(CommonStateGetter(service))
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
pub fn get_self_controller(proxy: &Service) -> Result<SelfController, GetSelfControllerError> {
    let result = proxy
        .dispatch(CMD_GET_SELF_CONTROLLER)
        .out_objects(1)
        .send()
        .map_err(GetSelfControllerError::Dispatch)?;

    if result.objects.is_empty() {
        return Err(GetSelfControllerError::MissingObject);
    }

    let object_id = result.objects[0];

    // Create sub-interface as domain subservice
    let service = Service {
        session: proxy.session,
        own_handle: 0,
        object_id,
        pointer_buffer_size: proxy.pointer_buffer_size,
    };

    Ok(SelfController(service))
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
    proxy: &Service,
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

    // Create sub-interface as domain subservice
    let service = Service {
        session: proxy.session,
        own_handle: 0,
        object_id,
        pointer_buffer_size: proxy.pointer_buffer_size,
    };

    Ok(WindowController(service))
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
    window_controller: &Service,
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
/// This translates the high-level mode into the three boolean parameters
/// expected by the service.
pub fn set_focus_handling_mode(
    self_controller: &Service,
    mode: AppletFocusHandlingMode,
) -> Result<(), SetFocusHandlingModeError> {
    // Translate mode to (in_focus, out_of_focus, background) parameters
    // Based on libnx appletSetFocusHandlingMode implementation
    let (notify_in_focus, notify_out_of_focus, suspend_on_background) = match mode {
        AppletFocusHandlingMode::SuspendHomeSleep => (false, false, true),
        AppletFocusHandlingMode::NoSuspend => (true, true, false),
        AppletFocusHandlingMode::SuspendHomeSleepNotify => (true, false, true),
        AppletFocusHandlingMode::AlwaysSuspend => (false, false, true),
    };

    // Input: 3 bools as u8 array
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

    Ok(())
}

/// Error returned by [`set_focus_handling_mode`].
#[derive(Debug, thiserror::Error)]
pub enum SetFocusHandlingModeError {
    /// Failed to dispatch the request.
    #[error("failed to dispatch request")]
    Dispatch(#[source] DispatchError),
}

/// Sets whether to suspend when out of focus (ISelfController, 2.0.0+).
pub fn set_out_of_focus_suspending_enabled(
    self_controller: &Service,
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
    ConvertToDomain(#[source] ServiceConvertToDomainError),
}

/// Enables operation mode change notifications (ISelfController, cmd 11).
///
/// When enabled, the applet receives `OperationModeChanged` messages
/// when the console transitions between handheld and docked modes.
pub fn set_operation_mode_changed_notification(
    self_controller: &Service,
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
    self_controller: &Service,
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
    window_controller: &Service,
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
    proxy: &Service,
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

    // Create sub-interface as domain subservice
    let service = Service {
        session: proxy.session,
        own_handle: 0,
        object_id,
        pointer_buffer_size: proxy.pointer_buffer_size,
    };

    Ok(ApplicationFunctions(service))
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
pub fn notify_running(app_funcs: &Service) -> Result<bool, NotifyRunningError> {
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
    self_controller: &Service,
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
