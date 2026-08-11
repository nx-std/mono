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
#[cfg(feature = "ffi")]
use nx_sf::error::ToResultCode as _;
#[cfg(feature = "ffi")]
use nx_svc::error::ToResultCode as _;
use nx_svc::process::Handle as ProcessHandle;

use super::state::{
    AppletCache,
    OwnedEventHandle,
    Slot,
};

/// Brings up an [`Application`]-role applet.
///
/// Performs the service-layer open via
/// [`nx_service_applet::proxy::open_application`], then the foreground
/// handshake this role alone performs in full: wait for focus, acquire
/// foreground rights, set the focus handling mode, announce that the process
/// is running. The shared snapshot and notification steps follow.
pub(crate) fn open_application(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<Application>, OpenApplicationError> {
    let proxy = proxy::open_application(sm, ph).map_err(OpenApplicationError::Open)?;

    wait_in_focus(proxy.common_state_getter()).map_err(OpenApplicationError::WaitInFocus)?;

    proxy
        .acquire_foreground_rights()
        .map_err(OpenApplicationError::AcquireForegroundRights)?;
    proxy
        .set_focus_handling_mode(AppletFocusHandlingMode::SuspendHomeSleep)
        .map_err(OpenApplicationError::SetFocusHandlingMode)?;
    proxy
        .notify_running()
        .map_err(OpenApplicationError::NotifyRunning)?;

    let cache = fetch_initial_cache(&proxy).map_err(OpenApplicationError::Cache)?;
    enable_mode_notifications(proxy.self_controller())
        .map_err(OpenApplicationError::Notifications)?;

    Ok(Slot { proxy, cache })
}

/// Error returned by [`open_application`].
#[derive(Debug, thiserror::Error)]
pub enum OpenApplicationError {
    /// The proxy could not be opened.
    ///
    /// Occurs when the Application Manager refused the session or one of the
    /// sub-interfaces behind it. Nothing was opened.
    #[error("failed to open the applet proxy")]
    Open(#[source] nx_service_applet::proxy::OpenError),
    /// The wait for foreground focus failed.
    ///
    /// Occurs when the message event could not be obtained or waited on. The
    /// proxy is open but the process never reached the foreground.
    #[error("failed to wait for foreground focus")]
    WaitInFocus(#[source] WaitInFocusError),
    /// Acquiring foreground rights was refused.
    #[error("failed to acquire foreground rights")]
    AcquireForegroundRights(#[source] nx_service_applet::AcquireForegroundRightsError),
    /// Setting the focus handling mode was refused.
    ///
    /// Only this role may set it; the Application Manager rejects the command
    /// from any other.
    #[error("failed to set the focus handling mode")]
    SetFocusHandlingMode(#[source] nx_service_applet::SetFocusHandlingModeError),
    /// Announcing that the process is running was refused.
    #[error("failed to announce that the process is running")]
    NotifyRunning(#[source] nx_service_applet::NotifyRunningError),
    /// The initial state snapshot failed.
    #[error("failed to read the initial applet state")]
    Cache(#[source] CacheError),
    /// Enabling mode-change notifications failed.
    #[error("failed to enable mode-change notifications")]
    Notifications(#[source] NotificationError),
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for OpenApplicationError {
    fn to_rc(self) -> crate::error::ResultCode {
        match self {
            Self::Open(err) => err.to_rc(),
            Self::WaitInFocus(err) => err.to_rc(),
            Self::AcquireForegroundRights(err) => err.to_rc(),
            Self::SetFocusHandlingMode(err) => err.to_rc(),
            Self::NotifyRunning(err) => err.to_rc(),
            Self::Cache(err) => err.to_rc(),
            Self::Notifications(err) => err.to_rc(),
        }
    }
}

/// Brings up a [`SystemApplication`]-role applet.
///
/// The same foreground handshake as [`open_application`], without the focus
/// handling mode: the Application Manager accepts that command from the
/// `Application` role alone.
pub(crate) fn open_system_application(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<SystemApplication>, OpenSystemApplicationError> {
    let proxy = proxy::open_system_application(sm, ph).map_err(OpenSystemApplicationError::Open)?;

    wait_in_focus(proxy.common_state_getter()).map_err(OpenSystemApplicationError::WaitInFocus)?;

    proxy
        .acquire_foreground_rights()
        .map_err(OpenSystemApplicationError::AcquireForegroundRights)?;
    proxy
        .notify_running()
        .map_err(OpenSystemApplicationError::NotifyRunning)?;

    let cache = fetch_initial_cache(&proxy).map_err(OpenSystemApplicationError::Cache)?;
    enable_mode_notifications(proxy.self_controller())
        .map_err(OpenSystemApplicationError::Notifications)?;

    Ok(Slot { proxy, cache })
}

/// Error returned by [`open_system_application`].
///
/// The same set as [`OpenApplicationError`] without `SetFocusHandlingMode`,
/// which this role cannot reach: a shared type would give every caller a
/// variant this function never returns.
#[derive(Debug, thiserror::Error)]
pub enum OpenSystemApplicationError {
    /// The proxy could not be opened.
    #[error("failed to open the applet proxy")]
    Open(#[source] nx_service_applet::proxy::OpenError),
    /// The wait for foreground focus failed.
    #[error("failed to wait for foreground focus")]
    WaitInFocus(#[source] WaitInFocusError),
    /// Acquiring foreground rights was refused.
    #[error("failed to acquire foreground rights")]
    AcquireForegroundRights(#[source] nx_service_applet::AcquireForegroundRightsError),
    /// Announcing that the process is running was refused.
    #[error("failed to announce that the process is running")]
    NotifyRunning(#[source] nx_service_applet::NotifyRunningError),
    /// The initial state snapshot failed.
    #[error("failed to read the initial applet state")]
    Cache(#[source] CacheError),
    /// Enabling mode-change notifications failed.
    #[error("failed to enable mode-change notifications")]
    Notifications(#[source] NotificationError),
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for OpenSystemApplicationError {
    fn to_rc(self) -> crate::error::ResultCode {
        match self {
            Self::Open(err) => err.to_rc(),
            Self::WaitInFocus(err) => err.to_rc(),
            Self::AcquireForegroundRights(err) => err.to_rc(),
            Self::NotifyRunning(err) => err.to_rc(),
            Self::Cache(err) => err.to_rc(),
            Self::Notifications(err) => err.to_rc(),
        }
    }
}

/// Brings up a [`LibraryApplet`]-role applet.
pub(crate) fn open_library_applet(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<LibraryApplet>, OpenAppletError> {
    let proxy = proxy::open_library_applet(sm, ph).map_err(OpenAppletError::Open)?;
    let cache = fetch_initial_cache(&proxy).map_err(OpenAppletError::Cache)?;
    enable_mode_notifications(proxy.self_controller()).map_err(OpenAppletError::Notifications)?;
    Ok(Slot { proxy, cache })
}

/// Brings up a [`SystemApplet`]-role applet.
pub(crate) fn open_system_applet(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<SystemApplet>, OpenAppletError> {
    let proxy = proxy::open_system_applet(sm, ph).map_err(OpenAppletError::Open)?;
    let cache = fetch_initial_cache(&proxy).map_err(OpenAppletError::Cache)?;
    enable_mode_notifications(proxy.self_controller()).map_err(OpenAppletError::Notifications)?;
    Ok(Slot { proxy, cache })
}

/// Brings up an [`OverlayApplet`]-role applet.
pub(crate) fn open_overlay_applet(
    sm: &SmService,
    ph: ProcessHandle,
) -> Result<Slot<OverlayApplet>, OpenAppletError> {
    let proxy = proxy::open_overlay_applet(sm, ph).map_err(OpenAppletError::Open)?;
    let cache = fetch_initial_cache(&proxy).map_err(OpenAppletError::Cache)?;
    enable_mode_notifications(proxy.self_controller()).map_err(OpenAppletError::Notifications)?;
    Ok(Slot { proxy, cache })
}

/// Error returned by [`open_library_applet`], [`open_system_applet`] and
/// [`open_overlay_applet`].
///
/// The three share it because they perform the same three steps: none of them
/// runs the foreground handshake, which the Application Manager accepts only
/// from a role that owns the screen.
#[derive(Debug, thiserror::Error)]
pub enum OpenAppletError {
    /// The proxy could not be opened.
    #[error("failed to open the applet proxy")]
    Open(#[source] nx_service_applet::proxy::OpenError),
    /// The initial state snapshot failed.
    #[error("failed to read the initial applet state")]
    Cache(#[source] CacheError),
    /// Enabling mode-change notifications failed.
    #[error("failed to enable mode-change notifications")]
    Notifications(#[source] NotificationError),
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for OpenAppletError {
    fn to_rc(self) -> crate::error::ResultCode {
        match self {
            Self::Open(err) => err.to_rc(),
            Self::Cache(err) => err.to_rc(),
            Self::Notifications(err) => err.to_rc(),
        }
    }
}

/// Reads the initial operation/performance/focus state and ARUID from a
/// freshly-opened proxy, packaged for storage in [`AppletCache`].
fn fetch_initial_cache<R: Role>(proxy: &Proxy<R>) -> Result<AppletCache, CacheError> {
    let operation_mode = proxy
        .common_state_getter()
        .get_operation_mode()
        .map_err(CacheError::GetOperationMode)?;
    let performance_mode = proxy
        .common_state_getter()
        .get_performance_mode()
        .map_err(CacheError::GetPerformanceMode)?;
    let focus_state = proxy
        .common_state_getter()
        .get_current_focus_state()
        .map_err(CacheError::GetFocusState)?;

    // ARUID failures are non-fatal: non-Application roles legitimately receive
    // ARUID=0 or an IPC error here, and every caller treats a missing one as
    // "this role has none".
    let aruid = proxy.get_applet_resource_user_id().unwrap_or_default();

    // Asked for once, here, because each ask mints a fresh kernel handle that
    // this process then owns. Every later reader borrows this one.
    let message_event = proxy
        .common_state_getter()
        .get_event_handle()
        .map(OwnedEventHandle::new)
        .map_err(CacheError::GetEventHandle)?;

    Ok(AppletCache {
        aruid,
        focus_state: AtomicU8::new(focus_state as u8),
        operation_mode: AtomicU8::new(operation_mode as u8),
        performance_mode: AtomicU32::new(performance_mode as u32),
        message_event,
    })
}

/// Error returned by [`fetch_initial_cache`].
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// The current operation mode could not be read.
    #[error("failed to read the current operation mode")]
    GetOperationMode(#[source] nx_service_applet::GetOperationModeError),
    /// The current performance mode could not be read.
    #[error("failed to read the current performance mode")]
    GetPerformanceMode(#[source] nx_service_applet::GetPerformanceModeError),
    /// The current focus state could not be read.
    #[error("failed to read the current focus state")]
    GetFocusState(#[source] nx_service_applet::GetCurrentFocusStateError),
    /// The message event could not be obtained.
    ///
    /// Occurs when the Application Manager refused to issue the handle the
    /// session waits on for messages. No handle was issued, so none leaks.
    #[error("failed to obtain the message event handle")]
    GetEventHandle(#[source] nx_service_applet::GetEventHandleError),
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for CacheError {
    fn to_rc(self) -> crate::error::ResultCode {
        match self {
            Self::GetOperationMode(err) => err.to_rc(),
            Self::GetPerformanceMode(err) => err.to_rc(),
            Self::GetFocusState(err) => err.to_rc(),
            Self::GetEventHandle(err) => err.to_rc(),
        }
    }
}

/// Enables operation- and performance-mode change notifications.
fn enable_mode_notifications(self_controller: SelfController<'_>) -> Result<(), NotificationError> {
    self_controller
        .set_operation_mode_changed_notification(true)
        .map_err(NotificationError::SetOperationMode)?;
    self_controller
        .set_performance_mode_changed_notification(true)
        .map_err(NotificationError::SetPerformanceMode)?;
    Ok(())
}

/// Error returned by [`enable_mode_notifications`].
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    /// Operation-mode change notifications could not be enabled.
    ///
    /// The session stays open; the cached operation mode is then only as
    /// current as the last read.
    #[error("failed to enable operation-mode change notifications")]
    SetOperationMode(#[source] nx_service_applet::SetOperationModeChangedNotificationError),
    /// Performance-mode change notifications could not be enabled.
    ///
    /// As above, for the performance mode.
    #[error("failed to enable performance-mode change notifications")]
    SetPerformanceMode(#[source] nx_service_applet::SetPerformanceModeChangedNotificationError),
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for NotificationError {
    fn to_rc(self) -> crate::error::ResultCode {
        match self {
            Self::SetOperationMode(err) => err.to_rc(),
            Self::SetPerformanceMode(err) => err.to_rc(),
        }
    }
}

/// Blocks until the applet's focus state becomes `InFocus`.
///
/// Takes the message event, reads the current focus state, then waits on the
/// event and re-reads the state on every focus-change message. Only the roles
/// that own the screen run this.
fn wait_in_focus(common_state_getter: CommonStateGetter<'_>) -> Result<(), WaitInFocusError> {
    // This runs before the session's own copy of the event exists, so it takes
    // one of its own and closes it on the way out. Both are handles to the same
    // kernel object; owning this one for the length of the wait is what keeps
    // it from outliving the loop.
    let event = common_state_getter
        .get_event_handle()
        .map(OwnedEventHandle::new)
        .map_err(WaitInFocusError::GetEventHandle)?;

    let mut focus_state = common_state_getter
        .get_current_focus_state()
        .map_err(WaitInFocusError::GetFocusState)?;

    while focus_state != AppletFocusState::InFocus {
        // SAFETY: `event` owns the handle for the whole of this loop, so it
        // names a live kernel event; the Application Manager issues it as a
        // resettable one, which is what `reset_signal` requires.
        unsafe {
            nx_svc::sync::wait_synchronization_single(event.as_handle(), u64::MAX)
                .map_err(WaitInFocusError::WaitSynchronization)?;
            // The event does not clear itself, so the signal is cleared here;
            // leaving it set would make the next wait return at once and spin
            // this loop. A refusal costs only that: the loop re-reads the
            // focus state each time round, so it still terminates on the
            // state itself rather than on the signal.
            let _ = nx_svc::sync::reset_signal(event.as_handle());
        }

        if let Ok(Some(msg)) = common_state_getter.receive_message()
            && matches!(msg, AppletMessage::FocusStateChanged)
        {
            focus_state = common_state_getter
                .get_current_focus_state()
                .map_err(WaitInFocusError::GetFocusState)?;
        }
    }

    Ok(())
}

/// Error returned by [`wait_in_focus`].
#[derive(Debug, thiserror::Error)]
pub enum WaitInFocusError {
    /// The message event could not be obtained.
    #[error("failed to obtain the message event handle")]
    GetEventHandle(#[source] nx_service_applet::GetEventHandleError),
    /// The current focus state could not be read.
    #[error("failed to read the current focus state")]
    GetFocusState(#[source] nx_service_applet::GetCurrentFocusStateError),
    /// Waiting on the message event failed.
    ///
    /// The process never reached the foreground, so the caller must not treat
    /// the session as ready.
    #[error("failed to wait on the message event")]
    WaitSynchronization(#[source] nx_svc::sync::WaitSyncError),
}

#[cfg(feature = "ffi")]
impl crate::error::ToResultCode for WaitInFocusError {
    fn to_rc(self) -> crate::error::ResultCode {
        match self {
            Self::GetEventHandle(err) => err.to_rc(),
            Self::GetFocusState(err) => err.to_rc(),
            Self::WaitSynchronization(err) => err.to_rc(),
        }
    }
}
