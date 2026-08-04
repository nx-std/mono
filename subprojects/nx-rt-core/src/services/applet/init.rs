//! Per-role applet initialization at the runtime layer.
//!
//! `nx-service-applet`'s `open_<role>` functions do the IPC plumbing (connect
//! → open proxy → drain core + extras). This module layers the libnx-faithful
//! runtime policy on top: the InFocus wait, AcquireForegroundRights,
//! SetFocusHandlingMode, NotifyRunning, initial cache snapshot, and enabling
//! mode-change notifications. Each `open_<role>` here returns a fully-cooked
//! [`Slot<R>`] ready to drop into the [`AppletSingleton`].

use core::sync::atomic::{
    AtomicU8,
    AtomicU32,
};

use nx_service_applet::{
    AppletFocusHandlingMode,
    AppletFocusState,
    AppletMessage,
    CommonStateGetter,
    SelfController,
    proxy::{
        self,
        Proxy,
    },
    role::{
        Application,
        LibraryApplet,
        OverlayApplet,
        Role,
        SystemApplet,
        SystemApplication,
    },
};
use nx_service_sm::SmService;
use nx_svc::process::Handle as ProcessHandle;

use super::{
    error::ConnectError,
    state::{
        AppletCache,
        Slot,
    },
};

/// Brings up an [`Application`]-role applet.
///
/// Performs the service-layer open via
/// [`nx_service_applet::proxy::open_application`], then runs the
/// Application-only runtime handshake: InFocus wait → AcquireForegroundRights
/// → SetFocusHandlingMode(SuspendHomeSleep) → NotifyRunning → cache snapshot
/// → enable mode notifications.
pub(super) fn open_application(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<Application>, ConnectError> {
    let proxy = proxy::open_application(sm, ph).map_err(ConnectError::Open)?;

    wait_in_focus(proxy.common_state_getter())?;

    proxy
        .acquire_foreground_rights()
        .map_err(ConnectError::AcquireForegroundRights)?;
    proxy
        .set_focus_handling_mode(AppletFocusHandlingMode::SuspendHomeSleep)
        .map_err(ConnectError::SetFocusHandlingMode)?;
    proxy
        .notify_running()
        .map_err(ConnectError::NotifyRunning)?;

    let cache = fetch_initial_cache(&proxy)?;
    enable_mode_notifications(proxy.self_controller())?;

    Ok(Slot { proxy, cache })
}

/// Brings up a [`SystemApplication`]-role applet.
///
/// Same as [`open_application`] minus `SetFocusHandlingMode` (AM rejects it
/// for any role other than `Application`).
pub(super) fn open_system_application(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<SystemApplication>, ConnectError> {
    let proxy = proxy::open_system_application(sm, ph).map_err(ConnectError::Open)?;

    wait_in_focus(proxy.common_state_getter())?;

    proxy
        .acquire_foreground_rights()
        .map_err(ConnectError::AcquireForegroundRights)?;
    // NOTE: libnx restricts `SetFocusHandlingMode` to Application
    // (`applet.c:566`); skip for SystemApplication.
    proxy
        .notify_running()
        .map_err(ConnectError::NotifyRunning)?;

    let cache = fetch_initial_cache(&proxy)?;
    enable_mode_notifications(proxy.self_controller())?;

    Ok(Slot { proxy, cache })
}

/// Brings up a [`LibraryApplet`]-role applet.
///
/// No InFocus wait, no foreground rights, no `NotifyRunning` — those are
/// Application-only in libnx. Just snapshot initial state and enable mode
/// notifications.
pub(super) fn open_library_applet(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<LibraryApplet>, ConnectError> {
    let proxy = proxy::open_library_applet(sm, ph).map_err(ConnectError::Open)?;
    let cache = fetch_initial_cache(&proxy)?;
    enable_mode_notifications(proxy.self_controller())?;
    Ok(Slot { proxy, cache })
}

/// Brings up a [`SystemApplet`]-role applet.
pub(super) fn open_system_applet(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<SystemApplet>, ConnectError> {
    let proxy = proxy::open_system_applet(sm, ph).map_err(ConnectError::Open)?;
    let cache = fetch_initial_cache(&proxy)?;
    enable_mode_notifications(proxy.self_controller())?;
    Ok(Slot { proxy, cache })
}

/// Brings up an [`OverlayApplet`]-role applet.
pub(super) fn open_overlay_applet(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<OverlayApplet>, ConnectError> {
    let proxy = proxy::open_overlay_applet(sm, ph).map_err(ConnectError::Open)?;
    let cache = fetch_initial_cache(&proxy)?;
    enable_mode_notifications(proxy.self_controller())?;
    Ok(Slot { proxy, cache })
}

/// Reads the initial operation/performance/focus state and ARUID from a
/// freshly-opened proxy, packaged for storage in [`AppletCache`].
fn fetch_initial_cache<R: Role>(proxy: &Proxy<R>) -> Result<AppletCache, ConnectError> {
    let operation_mode = proxy
        .common_state_getter()
        .get_operation_mode()
        .map_err(ConnectError::GetOperationMode)?;
    let performance_mode = proxy
        .common_state_getter()
        .get_performance_mode()
        .map_err(ConnectError::GetPerformanceMode)?;
    let focus_state = proxy
        .common_state_getter()
        .get_current_focus_state()
        .map_err(ConnectError::GetFocusState)?;

    // ARUID failures are non-fatal — non-Application roles legitimately
    // receive ARUID=0/IPC errors here. Mirror the prior FFI behavior of
    // treating any failure as "no aruid available".
    let aruid = proxy.get_applet_resource_user_id().unwrap_or_default();

    Ok(AppletCache {
        aruid,
        focus_state: AtomicU8::new(focus_state as u8),
        operation_mode: AtomicU8::new(operation_mode as u8),
        performance_mode: AtomicU32::new(performance_mode as u32),
    })
}

/// Enables operation- and performance-mode change notifications.
fn enable_mode_notifications(self_controller: SelfController<'_>) -> Result<(), ConnectError> {
    self_controller
        .set_operation_mode_changed_notification(true)
        .map_err(ConnectError::SetOperationModeNotification)?;
    self_controller
        .set_performance_mode_changed_notification(true)
        .map_err(ConnectError::SetPerformanceModeNotification)?;
    Ok(())
}

/// Blocks until the applet's focus state becomes `InFocus`.
///
/// Mirrors libnx `applet.c:272-301`: get the message event, get the current
/// focus state, then loop waiting on the event and refreshing focus on
/// `FocusStateChanged` messages. Application / SystemApplication path only.
fn wait_in_focus(common_state_getter: CommonStateGetter<'_>) -> Result<(), ConnectError> {
    let event_handle = common_state_getter
        .get_event_handle()
        .map_err(ConnectError::GetEventHandle)?;

    let mut focus_state = common_state_getter
        .get_current_focus_state()
        .map_err(ConnectError::GetFocusState)?;

    while focus_state != AppletFocusState::InFocus {
        // SAFETY: event_handle is a valid kernel handle owned by
        // CommonStateGetter for the duration of init. Waiting on / resetting
        // its signal is sound.
        unsafe {
            nx_svc::sync::wait_synchronization_single(&event_handle, u64::MAX)
                .map_err(ConnectError::WaitSynchronization)?;
            // The applet message event has autoclear=false; clear the signal
            // manually to avoid the wait returning immediately on the next
            // iteration.
            let _ = nx_svc::sync::reset_signal(&event_handle);
        }

        if let Ok(Some(msg)) = common_state_getter.receive_message()
            && matches!(msg, AppletMessage::FocusStateChanged)
        {
            focus_state = common_state_getter
                .get_current_focus_state()
                .map_err(ConnectError::GetFocusState)?;
        }
    }

    Ok(())
}
