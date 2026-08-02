//! Error types returned by applet initialization at the runtime layer.
//!
//! Wraps `nx_service_applet::proxy::OpenError` (the service-layer aggregate
//! covering connect/open/drain) and adds the runtime-policy steps the
//! libnx-faithful Application path performs after the proxy is open:
//! waiting for the InFocus event, acquiring foreground rights, setting the
//! focus handling mode, calling NotifyRunning, and enabling mode-change
//! notifications.

#[cfg(feature = "ffi")]
use nx_sf::error::ToResultCode as _;
#[cfg(feature = "ffi")]
use nx_svc::error::{ResultCode, ToResultCode as _};

#[cfg(feature = "ffi")]
use crate::error::ToResultCode;

/// Error returned by [`super::init`].
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Failed to open the applet proxy (service-layer: connect + open + drain
    /// extras + obtain the core seven sub-interfaces).
    #[error("failed to open applet proxy")]
    Open(#[source] nx_service_applet::proxy::OpenError),
    /// Failed to obtain the message event handle from `ICommonStateGetter`.
    #[error("failed to get message event handle")]
    GetEventHandle(#[source] nx_service_applet::GetEventHandleError),
    /// Failed to read the current focus state.
    #[error("failed to get current focus state")]
    GetFocusState(#[source] nx_service_applet::GetCurrentFocusStateError),
    /// Failed to read the current operation mode for the initial cache.
    #[error("failed to get current operation mode")]
    GetOperationMode(#[source] nx_service_applet::GetOperationModeError),
    /// Failed to read the current performance mode for the initial cache.
    #[error("failed to get current performance mode")]
    GetPerformanceMode(#[source] nx_service_applet::GetPerformanceModeError),
    /// Wait on the message event during the InFocus handshake failed.
    #[error("failed to wait for synchronization")]
    WaitSynchronization(#[source] nx_svc::sync::WaitSyncError),
    /// `IWindowController::AcquireForegroundRights` failed.
    #[error("failed to acquire foreground rights")]
    AcquireForegroundRights(#[source] nx_service_applet::AcquireForegroundRightsError),
    /// `ISelfController::SetFocusHandlingMode` failed.
    #[error("failed to set focus handling mode")]
    SetFocusHandlingMode(#[source] nx_service_applet::SetFocusHandlingModeError),
    /// `IApplicationFunctions::NotifyRunning` failed.
    #[error("failed to notify running")]
    NotifyRunning(#[source] nx_service_applet::NotifyRunningError),
    /// Enabling operation-mode-change notifications failed.
    #[error("failed to set operation mode notification")]
    SetOperationModeNotification(
        #[source] nx_service_applet::SetOperationModeChangedNotificationError,
    ),
    /// Enabling performance-mode-change notifications failed.
    #[error("failed to set performance mode notification")]
    SetPerformanceModeNotification(
        #[source] nx_service_applet::SetPerformanceModeChangedNotificationError,
    ),
}

#[cfg(feature = "ffi")]
impl ToResultCode for ConnectError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Open(err) => err.to_rc(),
            Self::GetEventHandle(err) => err.to_rc(),
            Self::GetFocusState(err) => err.to_rc(),
            Self::GetOperationMode(err) => err.to_rc(),
            Self::GetPerformanceMode(err) => err.to_rc(),
            Self::WaitSynchronization(err) => err.to_rc(),
            Self::AcquireForegroundRights(err) => err.to_rc(),
            Self::SetFocusHandlingMode(err) => err.to_rc(),
            Self::NotifyRunning(err) => err.to_rc(),
            Self::SetOperationModeNotification(err) => err.to_rc(),
            Self::SetPerformanceModeNotification(err) => err.to_rc(),
        }
    }
}
