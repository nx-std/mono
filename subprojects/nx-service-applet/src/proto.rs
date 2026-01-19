//! Protocol constants and types for the applet service.
//!
//! This module defines the service names, command IDs, and data types used
//! for communicating with the Horizon OS Applet Manager (AM) service.

use nx_sf::ServiceName;
use static_assertions::const_assert_eq;

/// Service name for application applets (`appletOE`).
///
/// Used by `AppletType::Application`.
pub const SERVICE_NAME_OE: ServiceName = ServiceName::new_truncate("appletOE");

/// Service name for other applet types (`appletAE`).
///
/// Used by `AppletType::SystemApplet`, `AppletType::LibraryApplet`,
/// `AppletType::OverlayApplet`, and `AppletType::SystemApplication`.
pub const SERVICE_NAME_AE: ServiceName = ServiceName::new_truncate("appletAE");

/// Command ID for OpenApplicationProxy (AppletType::Application)
pub const CMD_OPEN_APPLICATION_PROXY: u32 = 0;

/// Command ID for OpenSystemAppletProxy (AppletType::SystemApplet)
pub const CMD_OPEN_SYSTEM_APPLET_PROXY: u32 = 100;

/// Command ID for OpenLibraryAppletProxy (AppletType::LibraryApplet, pre-3.0.0)
pub const CMD_OPEN_LIBRARY_APPLET_PROXY: u32 = 200;

/// Command ID for OpenLibraryAppletProxyOld (AppletType::LibraryApplet, 3.0.0+)
///
/// This version accepts an AppletAttribute buffer.
pub const CMD_OPEN_LIBRARY_APPLET_PROXY_OLD: u32 = 201;

/// Command ID for OpenOverlayAppletProxy (AppletType::OverlayApplet)
pub const CMD_OPEN_OVERLAY_APPLET_PROXY: u32 = 300;

/// Command ID for OpenSystemApplicationProxy (AppletType::SystemApplication)
pub const CMD_OPEN_SYSTEM_APPLICATION_PROXY: u32 = 350;

/// Command ID for GetCommonStateGetter
pub const CMD_GET_COMMON_STATE_GETTER: u32 = 0;

/// Command ID for GetSelfController
pub const CMD_GET_SELF_CONTROLLER: u32 = 1;

/// Command ID for GetWindowController
pub const CMD_GET_WINDOW_CONTROLLER: u32 = 2;

// The following constants are reserved for future implementation phases.
// They are defined here for documentation purposes.

/// Command ID for GetAudioController
#[allow(dead_code)]
pub const CMD_GET_AUDIO_CONTROLLER: u32 = 3;

/// Command ID for GetDisplayController
#[allow(dead_code)]
pub const CMD_GET_DISPLAY_CONTROLLER: u32 = 4;

/// Command ID for GetProcessWindingController (LibraryApplet only)
#[allow(dead_code)]
pub const CMD_GET_PROCESS_WINDING_CONTROLLER: u32 = 10;

/// Command ID for GetLibraryAppletCreator
#[allow(dead_code)]
pub const CMD_GET_LIBRARY_APPLET_CREATOR: u32 = 11;

/// Command ID for GetLibraryAppletSelfAccessor or IFunctions (type-dependent)
#[allow(dead_code)]
pub const CMD_GET_FUNCTIONS_OR_SELF_ACCESSOR: u32 = 20;

/// Command ID for GetAppletCommonFunctions (7.0.0+) or IGlobalStateController
#[allow(dead_code)]
pub const CMD_GET_APPLET_COMMON_FUNCTIONS: u32 = 21;

/// Command ID for GetApplicationCreator (SystemApplet) or GetHomeMenuFunctions (15.0.0+)
#[allow(dead_code)]
pub const CMD_GET_APPLICATION_CREATOR: u32 = 22;

/// Command ID for GetAppletCommonFunctions (SystemApplet, 7.0.0+)
#[allow(dead_code)]
pub const CMD_GET_APPLET_COMMON_FUNCTIONS_SYSTEM: u32 = 23;

/// Command ID for GetDebugFunctions
#[allow(dead_code)]
pub const CMD_GET_DEBUG_FUNCTIONS: u32 = 1000;

/// Command ID for GetEventHandle (ICommonStateGetter)
pub const CMD_CSG_GET_EVENT_HANDLE: u32 = 0;

/// Command ID for ReceiveMessage (ICommonStateGetter)
pub const CMD_CSG_RECEIVE_MESSAGE: u32 = 1;

/// Command ID for GetOperationMode (ICommonStateGetter)
pub const CMD_CSG_GET_OPERATION_MODE: u32 = 5;

/// Command ID for GetPerformanceMode (ICommonStateGetter)
pub const CMD_CSG_GET_PERFORMANCE_MODE: u32 = 6;

/// Command ID for GetCradleStatus (ICommonStateGetter)
#[allow(dead_code)]
pub const CMD_CSG_GET_CRADLE_STATUS: u32 = 7;

/// Command ID for GetBootMode (ICommonStateGetter)
#[allow(dead_code)]
pub const CMD_CSG_GET_BOOT_MODE: u32 = 8;

/// Command ID for GetCurrentFocusState (ICommonStateGetter)
pub const CMD_CSG_GET_CURRENT_FOCUS_STATE: u32 = 9;

/// Command ID for SetOperationModeChangedNotification (ISelfController)
pub const CMD_SC_SET_OPERATION_MODE_CHANGED_NOTIFICATION: u32 = 11;

/// Command ID for SetPerformanceModeChangedNotification (ISelfController)
pub const CMD_SC_SET_PERFORMANCE_MODE_CHANGED_NOTIFICATION: u32 = 12;

/// Command ID for SetFocusHandlingMode (ISelfController)
pub const CMD_SC_SET_FOCUS_HANDLING_MODE: u32 = 13;

/// Command ID for SetOutOfFocusSuspendingEnabled (ISelfController, 2.0.0+)
pub const CMD_SC_SET_OUT_OF_FOCUS_SUSPENDING_ENABLED: u32 = 16;

/// Command ID for CreateManagedDisplayLayer (ISelfController)
pub const CMD_SC_CREATE_MANAGED_DISPLAY_LAYER: u32 = 40;

/// Command ID for GetAppletResourceUserId (IWindowController)
pub const CMD_WC_GET_APPLET_RESOURCE_USER_ID: u32 = 1;

/// Command ID for AcquireForegroundRights (IWindowController)
pub const CMD_WC_ACQUIRE_FOREGROUND_RIGHTS: u32 = 10;

/// Command ID for GetApplicationFunctions (IApplicationProxy, AppletType::Application only)
///
/// Returns IApplicationFunctions interface (cmd 20).
/// Only available for Application type applets via appletOE.
pub const CMD_GET_APPLICATION_FUNCTIONS: u32 = 20;

/// Command ID for NotifyRunning (IApplicationFunctions)
///
/// Notifies the system that the application has completed initialization
/// and is ready to run. This should be called after:
/// - Waiting for InFocus state
/// - Acquiring foreground rights
/// - Setting up focus handling mode
pub const CMD_AF_NOTIFY_RUNNING: u32 = 40;

/// Applet type determining which service and proxy to use.
///
/// This value controls whether the applet connects to `appletOE` or `appletAE`,
/// and which proxy command is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AppletType {
    /// No applet service (applet functions are no-ops).
    None = -2,
    /// Default type (auto-detects to Application).
    #[default]
    Default = -1,
    /// Main application applet. Uses `appletOE` service.
    Application = 0,
    /// System applet (e.g., qlaunch). Uses `appletAE` service.
    SystemApplet = 1,
    /// Library applet. Uses `appletAE` service.
    LibraryApplet = 2,
    /// Overlay applet. Uses `appletAE` service.
    OverlayApplet = 3,
    /// System application. Uses `appletAE` service.
    SystemApplication = 4,
}

impl AppletType {
    /// Returns the raw i32 value of this applet type.
    #[inline]
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    /// Creates an `AppletType` from a raw i32 value.
    ///
    /// Returns `None` if the value doesn't correspond to a valid applet type.
    #[inline]
    pub const fn from_raw(value: i32) -> Option<Self> {
        match value {
            -2 => Some(Self::None),
            -1 => Some(Self::Default),
            0 => Some(Self::Application),
            1 => Some(Self::SystemApplet),
            2 => Some(Self::LibraryApplet),
            3 => Some(Self::OverlayApplet),
            4 => Some(Self::SystemApplication),
            _ => None,
        }
    }

    /// Returns true if this applet type uses `appletOE` service.
    #[inline]
    pub const fn uses_applet_oe(self) -> bool {
        matches!(self, Self::Application)
    }

    /// Returns true if this is an application type (Application or SystemApplication).
    #[inline]
    pub const fn is_application(self) -> bool {
        matches!(self, Self::Application | Self::SystemApplication)
    }

    /// Returns true if this is specifically a regular application.
    #[inline]
    pub const fn is_regular_application(self) -> bool {
        matches!(self, Self::Application)
    }
}

/// Focus state of the applet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AppletFocusState {
    /// Applet is in focus and active.
    InFocus = 1,
    /// Out of focus due to a LibraryApplet being open.
    OutOfFocus = 2,
    /// Out of focus due to HOME menu being open or console sleeping.
    Background = 3,
}

impl AppletFocusState {
    /// Creates an `AppletFocusState` from a raw u8 value.
    #[inline]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::InFocus),
            2 => Some(Self::OutOfFocus),
            3 => Some(Self::Background),
            _ => None,
        }
    }
}

/// Operation mode of the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AppletOperationMode {
    /// Handheld mode (undocked).
    #[default]
    Handheld = 0,
    /// Console mode (docked / TV-mode).
    Console = 1,
}

impl AppletOperationMode {
    /// Creates an `AppletOperationMode` from a raw u8 value.
    #[inline]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Handheld),
            1 => Some(Self::Console),
            _ => None,
        }
    }
}

/// Messages received from the applet event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AppletMessage {
    /// Exit request from the system.
    ExitRequest = 4,
    /// Focus state changed.
    FocusStateChanged = 15,
    /// Applet execution was resumed.
    Resume = 16,
    /// Operation mode changed (handheld/docked).
    OperationModeChanged = 30,
    /// Performance mode changed.
    PerformanceModeChanged = 31,
    /// Display requested (see `appletApproveToDisplay`).
    RequestToDisplay = 51,
    /// Capture button was short-pressed.
    CaptureButtonShortPressed = 90,
    /// Screenshot was taken.
    AlbumScreenShotTaken = 92,
    /// Album recording was saved.
    AlbumRecordingSaved = 93,
}

impl AppletMessage {
    /// Creates an `AppletMessage` from a raw u32 value.
    #[inline]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            4 => Some(Self::ExitRequest),
            15 => Some(Self::FocusStateChanged),
            16 => Some(Self::Resume),
            30 => Some(Self::OperationModeChanged),
            31 => Some(Self::PerformanceModeChanged),
            51 => Some(Self::RequestToDisplay),
            90 => Some(Self::CaptureButtonShortPressed),
            92 => Some(Self::AlbumScreenShotTaken),
            93 => Some(Self::AlbumRecordingSaved),
            _ => None,
        }
    }
}

/// Focus handling mode for applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum AppletFocusHandlingMode {
    /// Suspend only when HOME menu is open or console is sleeping (default).
    #[default]
    SuspendHomeSleep = 0,
    /// Don't suspend when out of focus.
    NoSuspend = 1,
    /// Suspend when HOME/sleep but still receive OnFocusState hook.
    SuspendHomeSleepNotify = 2,
    /// Always suspend when out of focus, regardless of reason.
    AlwaysSuspend = 3,
}

/// Applet attribute for LibraryApplet proxy (3.0.0+).
///
/// Used with `OpenLibraryAppletProxyOld` (cmd 201).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct AppletAttribute {
    /// Flag. When non-zero, two state fields are set to 1.
    pub flag: u8,
    /// Reserved/unused.
    _reserved: [u8; 0x7F],
}

const_assert_eq!(size_of::<AppletAttribute>(), 0x80);

impl AppletAttribute {
    /// Creates a new zeroed `AppletAttribute`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            flag: 0,
            _reserved: [0; 0x7F],
        }
    }

    /// Creates a new `AppletAttribute` with the specified flag.
    #[inline]
    pub const fn with_flag(flag: u8) -> Self {
        Self {
            flag,
            _reserved: [0; 0x7F],
        }
    }
}

impl Default for AppletAttribute {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
