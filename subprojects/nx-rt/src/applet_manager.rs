//! Applet Manager (AM) state and singleton API
//!
//! This module manages the applet service sessions and sub-interfaces.
//! Since applet has multiple sub-interfaces that aren't identified by ServiceName,
//! they are stored in a dedicated structure rather than the generic service registry.

use nx_service_applet::{
    AppletFocusHandlingMode, AppletFocusState, AppletMessage, AppletProxyService, AppletService,
    AppletType, ApplicationFunctions, CommonStateGetter, SelfController, WindowController,
    aruid::Aruid,
};
use nx_std_sync::{once_lock::OnceLock, rwlock::RwLock};
use nx_svc::process::Handle as ProcessHandle;

use crate::service_manager;

/// Global applet state, lazily initialized.
static APPLET_STATE: OnceLock<RwLock<Option<AppletState>>> = OnceLock::new();

/// Returns a reference to the applet state lock, initializing it if needed.
fn state() -> &'static RwLock<Option<AppletState>> {
    APPLET_STATE.get_or_init(|| RwLock::new(None))
}

/// Initializes the applet service.
///
/// Connects to appletOE or appletAE based on applet type,
/// and gets the required sub-interfaces.
///
/// # Panics
///
/// Panics if SM is not initialized.
pub fn init(applet_type: AppletType, process_handle: ProcessHandle) -> Result<(), ConnectError> {
    // Don't initialize if AppletType::None
    if matches!(applet_type, AppletType::None) {
        return Ok(());
    }

    let sm_guard = service_manager::sm_session();
    let sm = sm_guard.as_ref().expect("SM not initialized");

    // Connect to appletOE or appletAE
    let service =
        match nx_service_applet::connect(sm, applet_type).map_err(ConnectError::Connect)? {
            Some(service) => service,
            None => return Ok(()), // AppletType::None
        };

    // Open proxy session
    let proxy = service
        .open_proxy(applet_type, process_handle)
        .map_err(ConnectError::OpenProxy)?;

    // Get sub-interfaces
    let common_state_getter = proxy
        .get_common_state_getter()
        .map_err(ConnectError::GetCommonStateGetter)?;

    let self_controller = proxy
        .get_self_controller()
        .map_err(ConnectError::GetSelfController)?;

    // WindowController and ApplicationFunctions handling depends on applet type
    let is_application = matches!(applet_type, AppletType::Application);

    // WindowController is mandatory for Application type, optional for others
    let window_controller = if is_application {
        Some(
            proxy
                .get_window_controller()
                .map_err(ConnectError::GetWindowController)?,
        )
    } else {
        proxy.get_window_controller().ok()
    };

    // ApplicationFunctions is only available for Application type
    let application_functions = if is_application {
        Some(
            proxy
                .get_application_functions()
                .map_err(ConnectError::GetApplicationFunctions)?,
        )
    } else {
        None
    };

    // Application-specific initialization handshake
    if is_application {
        // SAFETY: For Application type, we ensured window_controller and application_functions are Some
        let wc = window_controller.as_ref().unwrap();
        let app_funcs = application_functions.as_ref().unwrap();

        // 1. Get message event handle
        let event_handle = common_state_getter
            .get_event_handle()
            .map_err(ConnectError::GetEventHandle)?;

        // 2. Get initial focus state
        let mut focus_state = common_state_getter
            .get_current_focus_state()
            .map_err(ConnectError::GetFocusState)?;

        // 3. Wait for InFocus state (blocking loop)
        // This is critical - the application must wait for the system to grant focus
        // before it can render or acquire foreground rights
        while focus_state != AppletFocusState::InFocus {
            // Wait on message event
            // SAFETY: event_handle is a valid kernel handle obtained from get_event_handle.
            // The handle remains valid for the duration of this wait.
            unsafe {
                nx_svc::sync::wait_synchronization_single(&event_handle, u64::MAX)
                    .map_err(ConnectError::WaitSynchronization)?;

                // Reset the event signal - the applet message event has autoclear=false.
                // Without resetting, the event remains signaled and the wait returns immediately,
                // causing a busy-loop that results in a black screen.
                let _ = nx_svc::sync::reset_signal(&event_handle);
            }

            // Receive and process message
            if let Ok(Some(msg)) = common_state_getter.receive_message()
                && matches!(msg, AppletMessage::FocusStateChanged)
            {
                focus_state = common_state_getter
                    .get_current_focus_state()
                    .map_err(ConnectError::GetFocusState)?;
            }
        }

        // 4. Acquire foreground rights
        wc.acquire_foreground_rights()
            .map_err(ConnectError::AcquireForegroundRights)?;

        // 5. Set focus handling mode
        self_controller
            .set_focus_handling_mode(AppletFocusHandlingMode::SuspendHomeSleep)
            .map_err(ConnectError::SetFocusHandlingMode)?;

        // 6. Notify the system the application is ready
        app_funcs
            .notify_running()
            .map_err(ConnectError::NotifyRunning)?;

        // 7. Enable mode change notifications
        self_controller
            .set_operation_mode_changed_notification(true)
            .map_err(ConnectError::SetOperationModeNotification)?;

        self_controller
            .set_performance_mode_changed_notification(true)
            .map_err(ConnectError::SetPerformanceModeNotification)?;
    }

    // Fetch and cache the applet resource user ID
    let aruid = window_controller
        .as_ref()
        .and_then(|wc| wc.get_applet_resource_user_id().unwrap_or(None));

    // Store in registry
    let applet_state = AppletState {
        service,
        proxy,
        common_state_getter,
        self_controller,
        window_controller,
        application_functions,
        aruid,
    };

    let mut guard = state().write();
    *guard = Some(applet_state);

    Ok(())
}

/// Gets the applet proxy session.
pub fn get_proxy() -> Option<impl core::ops::Deref<Target = AppletProxyService> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(AppletProxyRef(guard))
    } else {
        None
    }
}

/// Gets the ICommonStateGetter sub-interface.
pub fn get_common_state_getter()
-> Option<impl core::ops::Deref<Target = CommonStateGetter> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(AppletCommonStateGetterRef(guard))
    } else {
        None
    }
}

/// Gets the ISelfController sub-interface.
pub fn get_self_controller() -> Option<impl core::ops::Deref<Target = SelfController> + 'static> {
    let guard = state().read();
    if guard.is_some() {
        Some(AppletSelfControllerRef(guard))
    } else {
        None
    }
}

/// Gets the IWindowController sub-interface.
///
/// Returns None if the applet is not initialized or if WindowController is not available.
pub fn get_window_controller() -> Option<impl core::ops::Deref<Target = WindowController> + 'static>
{
    let guard = state().read();
    if guard.as_ref()?.window_controller.is_some() {
        Some(AppletWindowControllerRef(guard))
    } else {
        None
    }
}

/// Gets the cached applet resource user ID.
///
/// Returns `None` if the applet is not initialized or if the ARUID was not
/// available during init.
pub fn get_applet_resource_user_id() -> Option<Aruid> {
    state().read().as_ref().and_then(|s| s.aruid)
}

/// Exits the applet service session.
pub fn exit() {
    let mut guard = state().write();
    if let Some(applet_state) = guard.take() {
        // Close sub-interfaces and sessions in reverse order
        if let Some(application_functions) = applet_state.application_functions {
            application_functions.close();
        }
        if let Some(window_controller) = applet_state.window_controller {
            window_controller.close();
        }
        applet_state.self_controller.close();
        applet_state.common_state_getter.close();
        applet_state.proxy.close();
        applet_state.service.close();
    }
}

/// Internal storage for applet service sessions.
struct AppletState {
    /// Main service session (appletOE or appletAE)
    service: AppletService,
    /// Proxy session
    proxy: AppletProxyService,
    /// ICommonStateGetter sub-interface
    common_state_getter: CommonStateGetter,
    /// ISelfController sub-interface
    self_controller: SelfController,
    /// IWindowController sub-interface (mandatory for Application, optional for others)
    window_controller: Option<WindowController>,
    /// IApplicationFunctions sub-interface (Application type only)
    application_functions: Option<ApplicationFunctions>,
    /// Cached applet resource user ID (fetched once during init)
    aruid: Option<Aruid>,
}

/// Wrapper for accessing AppletProxyService through RwLockReadGuard
struct AppletProxyRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<AppletState>>);

impl core::ops::Deref for AppletProxyRef {
    type Target = AppletProxyService;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create AppletProxyRef when the option is Some
        &self.0.as_ref().unwrap().proxy
    }
}

/// Wrapper for accessing CommonStateGetter through RwLockReadGuard
struct AppletCommonStateGetterRef(
    nx_std_sync::rwlock::RwLockReadGuard<'static, Option<AppletState>>,
);

impl core::ops::Deref for AppletCommonStateGetterRef {
    type Target = CommonStateGetter;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create this ref when the option is Some
        &self.0.as_ref().unwrap().common_state_getter
    }
}

/// Wrapper for accessing SelfController through RwLockReadGuard
struct AppletSelfControllerRef(nx_std_sync::rwlock::RwLockReadGuard<'static, Option<AppletState>>);

impl core::ops::Deref for AppletSelfControllerRef {
    type Target = SelfController;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create this ref when the option is Some
        &self.0.as_ref().unwrap().self_controller
    }
}

/// Wrapper for accessing WindowController through RwLockReadGuard
struct AppletWindowControllerRef(
    nx_std_sync::rwlock::RwLockReadGuard<'static, Option<AppletState>>,
);

impl core::ops::Deref for AppletWindowControllerRef {
    type Target = WindowController;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We only create this ref when both AppletState and window_controller are Some
        self.0.as_ref().unwrap().window_controller.as_ref().unwrap()
    }
}

/// Error returned by [`init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to connect to applet service.
    #[error("failed to connect to applet service")]
    Connect(#[source] nx_service_applet::ConnectError),
    /// Failed to open proxy session.
    #[error("failed to open applet proxy")]
    OpenProxy(#[source] nx_service_applet::OpenProxyError),
    /// Failed to get ICommonStateGetter.
    #[error("failed to get ICommonStateGetter")]
    GetCommonStateGetter(#[source] nx_service_applet::GetCommonStateGetterError),
    /// Failed to get ISelfController.
    #[error("failed to get ISelfController")]
    GetSelfController(#[source] nx_service_applet::GetSelfControllerError),
    /// Failed to get IWindowController.
    #[error("failed to get IWindowController")]
    GetWindowController(#[source] nx_service_applet::GetWindowControllerError),
    /// Failed to get IApplicationFunctions.
    #[error("failed to get IApplicationFunctions")]
    GetApplicationFunctions(#[source] nx_service_applet::GetApplicationFunctionsError),
    /// Failed to get message event handle.
    #[error("failed to get message event handle")]
    GetEventHandle(#[source] nx_service_applet::GetEventHandleError),
    /// Failed to get current focus state.
    #[error("failed to get current focus state")]
    GetFocusState(#[source] nx_service_applet::GetCurrentFocusStateError),
    /// Failed to wait for synchronization.
    #[error("failed to wait for synchronization")]
    WaitSynchronization(#[source] nx_svc::sync::WaitSyncError),
    /// Failed to acquire foreground rights.
    #[error("failed to acquire foreground rights")]
    AcquireForegroundRights(#[source] nx_service_applet::AcquireForegroundRightsError),
    /// Failed to set focus handling mode.
    #[error("failed to set focus handling mode")]
    SetFocusHandlingMode(#[source] nx_service_applet::SetFocusHandlingModeError),
    /// Failed to notify running.
    #[error("failed to notify running")]
    NotifyRunning(#[source] nx_service_applet::NotifyRunningError),
    /// Failed to set operation mode notification.
    #[error("failed to set operation mode notification")]
    SetOperationModeNotification(
        #[source] nx_service_applet::SetOperationModeChangedNotificationError,
    ),
    /// Failed to set performance mode notification.
    #[error("failed to set performance mode notification")]
    SetPerformanceModeNotification(
        #[source] nx_service_applet::SetPerformanceModeChangedNotificationError,
    ),
}
