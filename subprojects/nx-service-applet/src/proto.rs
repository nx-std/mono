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

/// Command ID for OpenLibraryAppletProxyOld (AppletType::LibraryApplet, pre-3.0.0)
///
/// Legacy proxy command without an `AppletAttribute` buffer.
pub const CMD_OPEN_LIBRARY_APPLET_PROXY_OLD: u32 = 200;

/// Command ID for OpenLibraryAppletProxy (AppletType::LibraryApplet, 3.0.0+)
///
/// This version accepts an `AppletAttribute` buffer.
pub const CMD_OPEN_LIBRARY_APPLET_PROXY: u32 = 201;

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
pub const CMD_GET_AUDIO_CONTROLLER: u32 = 3;

/// Command ID for GetDisplayController
pub const CMD_GET_DISPLAY_CONTROLLER: u32 = 4;

/// Command ID for GetProcessWindingController (LibraryApplet only)
pub const CMD_GET_PROCESS_WINDING_CONTROLLER: u32 = 10;

/// Command ID for GetLibraryAppletCreator
pub const CMD_GET_LIBRARY_APPLET_CREATOR: u32 = 11;

/// Command ID for `Get*Functions` (cmd 20) — type-dependent.
///
/// For Application this returns IApplicationFunctions (see [`CMD_GET_APPLICATION_FUNCTIONS`]);
/// for SystemApplet/OverlayApplet/SystemApplication it returns the type-specific
/// `IFunctions`; for LibraryApplet it returns ILibraryAppletSelfAccessor; on HOS 15.0.0+
/// for LibraryApplet it returns IHomeMenuFunctions.
pub const CMD_GET_FUNCTIONS_OR_SELF_ACCESSOR: u32 = 20;

/// Command ID for GetAppletCommonFunctions (7.0.0+, non-SystemApplet) or
/// GetGlobalStateController (SystemApplet, always).
pub const CMD_GET_APPLET_COMMON_FUNCTIONS: u32 = 21;

/// Command ID for GetApplicationCreator (SystemApplet) or GetHomeMenuFunctions
/// (LibraryApplet, 15.0.0+).
pub const CMD_GET_APPLICATION_CREATOR: u32 = 22;

/// Command ID for GetAppletCommonFunctions (SystemApplet, 7.0.0+) or
/// GetGlobalStateController (LibraryApplet/OverlayApplet, 15.0.0+).
pub const CMD_GET_APPLET_COMMON_FUNCTIONS_SYSTEM: u32 = 23;

/// Command ID for GetDebugFunctions
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
/// Command ID for GetLibraryAppletLaunchableEvent (ISelfController).
///
/// The system signals this event when it is willing to launch a library applet.
/// Creating one before it is signalled races the system, so a launch waits here
/// first.
pub const CMD_SC_GET_LIBRARY_APPLET_LAUNCHABLE_EVENT: u32 = 9;

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

/// Command ID for CreateLibraryApplet (ILibraryAppletCreator).
pub const CMD_LAC_CREATE_LIBRARY_APPLET: u32 = 0;

/// Command ID for CreateStorage (ILibraryAppletCreator).
pub const CMD_LAC_CREATE_STORAGE: u32 = 10;

/// Command ID for GetAppletStateChangedEvent (ILibraryAppletAccessor).
pub const CMD_LAA_GET_APPLET_STATE_CHANGED_EVENT: u32 = 0;

/// Command ID for Start (ILibraryAppletAccessor).
pub const CMD_LAA_START: u32 = 10;

/// Command ID for GetResult (ILibraryAppletAccessor).
///
/// Carries the applet's own exit status: a plain success reply means the applet
/// exited normally, and a service error is the applet's result verbatim.
pub const CMD_LAA_GET_RESULT: u32 = 30;

/// Command ID for PushInData (ILibraryAppletAccessor).
pub const CMD_LAA_PUSH_IN_DATA: u32 = 100;

/// Command ID for PopOutData (ILibraryAppletAccessor).
pub const CMD_LAA_POP_OUT_DATA: u32 = 101;

/// Command ID for Open (IStorage).
pub const CMD_STORAGE_OPEN: u32 = 0;

/// Command ID for GetSize (IStorageAccessor).
pub const CMD_STORAGE_ACCESSOR_GET_SIZE: u32 = 0;

/// Command ID for Write (IStorageAccessor).
pub const CMD_STORAGE_ACCESSOR_WRITE: u32 = 10;

/// Command ID for Read (IStorageAccessor).
pub const CMD_STORAGE_ACCESSOR_READ: u32 = 11;

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

/// Performance mode reported by `ICommonStateGetter::GetPerformanceMode`.
///
/// Matches libnx `AppletPerformanceMode` (signed enum read as `u32` over IPC).
/// The `Invalid` variant corresponds to libnx's `AppletPerformanceMode_Invalid`
/// (`-1`), which the server may return when the mode is not yet determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum AppletPerformanceMode {
    /// Performance mode has not been determined yet.
    Invalid = u32::MAX,
    /// Normal performance (default clocks).
    #[default]
    Normal = 0,
    /// Boost performance (higher CPU/GPU clocks).
    Boost = 1,
}

impl AppletPerformanceMode {
    /// Creates an `AppletPerformanceMode` from a raw u32 value.
    ///
    /// Returns `None` if the value doesn't correspond to a known mode.
    #[inline]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            u32::MAX => Some(Self::Invalid),
            0 => Some(Self::Normal),
            1 => Some(Self::Boost),
            _ => None,
        }
    }
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
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
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

/// Identifies which applet `CreateLibraryApplet` launches.
///
/// The values below `0x0A` name applets that are not library applets; they are
/// part of the same system-wide enum and are listed for completeness, but only
/// the `LibraryApplet*` variants are valid for `CreateLibraryApplet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AppletId {
    /// No applet.
    None = 0x00,
    /// Application. Not valid for use with library applets.
    Application = 0x01,
    /// `overlayDisp`.
    OverlayApplet = 0x02,
    /// `qlaunch`.
    SystemAppletMenu = 0x03,
    /// `starter`.
    SystemApplication = 0x04,
    /// `auth`.
    LibraryAppletAuth = 0x0A,
    /// `cabinet`.
    LibraryAppletCabinet = 0x0B,
    /// `controller`.
    LibraryAppletController = 0x0C,
    /// `dataErase`.
    LibraryAppletDataErase = 0x0D,
    /// `error`, the system error dialog.
    LibraryAppletError = 0x0E,
    /// `netConnect`.
    LibraryAppletNetConnect = 0x0F,
    /// `playerSelect`.
    LibraryAppletPlayerSelect = 0x10,
    /// `swkbd`, the software keyboard.
    LibraryAppletSwkbd = 0x11,
    /// `miiEdit`.
    LibraryAppletMiiEdit = 0x12,
    /// `LibAppletWeb`.
    LibraryAppletWeb = 0x13,
    /// `LibAppletShop`.
    LibraryAppletShop = 0x14,
    /// `photoViewer`.
    LibraryAppletPhotoViewer = 0x15,
    /// `set`. Not present on retail devices.
    LibraryAppletSet = 0x16,
    /// `LibAppletOff`.
    LibraryAppletOfflineWeb = 0x17,
    /// `LibAppletLns`.
    LibraryAppletLoginShare = 0x18,
    /// `LibAppletAuth`.
    LibraryAppletWifiWebAuth = 0x19,
    /// `myPage`.
    LibraryAppletMyPage = 0x1A,
}

impl AppletId {
    /// Returns the raw u32 value of this applet id.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// Why a library applet stopped running.
///
/// Derived from the result `GetResult` replies with: a success means the applet
/// ran to completion, and every other value is the applet's own result,
/// classified the way libnx's `appletHolderJoin` classifies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryAppletExitReason {
    /// The applet ran to completion.
    Normal,
    /// The user dismissed the applet without completing it.
    Canceled,
    /// The applet terminated abnormally.
    Abnormal,
    /// The applet failed in a way this mapping does not name.
    Unexpected,
}

impl LibraryAppletExitReason {
    /// Classifies the result code `GetResult` replied with.
    ///
    /// Only module 128 carries an exit reason; anything else is a failure the
    /// applet protocol does not describe, and so reads as
    /// [`Unexpected`](Self::Unexpected).
    pub const fn from_result_code(code: u32) -> Self {
        /// Result module that carries library applet exit reasons.
        const MODULE_APPLET: u32 = 128;
        /// Description meaning the user dismissed the applet.
        const DESC_CANCELED: u32 = 22;
        /// Descriptions in `[start, end)` mean an abnormal termination.
        const DESC_ABNORMAL_START: u32 = 0x14;
        const DESC_ABNORMAL_END: u32 = 0x32;

        let module = code & 0x1FF;
        let description = (code >> 9) & 0x1FFF;

        if module != MODULE_APPLET {
            return Self::Unexpected;
        }

        if description == DESC_CANCELED {
            Self::Canceled
        } else if description >= DESC_ABNORMAL_START && description < DESC_ABNORMAL_END {
            Self::Abnormal
        } else {
            Self::Unexpected
        }
    }
}

/// How a library applet is presented once started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum LibraryAppletMode {
    /// Foreground.
    #[default]
    AllForeground = 0,
    /// Background.
    Background = 1,
    /// No UI.
    NoUi = 2,
    /// Background with indirect display.
    BackgroundIndirect = 3,
    /// Foreground, but initially hidden.
    AllForegroundInitiallyHidden = 4,
}

impl LibraryAppletMode {
    /// Returns the raw u32 value of this mode.
    #[inline]
    pub const fn as_raw(self) -> u32 {
        self as u32
    }
}

/// The common arguments every library applet reads as its **first** storage.
///
/// libnx calls this `LibAppletArgs`; switchbrew calls it `CommonArguments`. The
/// applet rejects a `version` other than 1, and reads `size` to decide how much
/// of the struct it can trust.
///
/// Fixed-layout payload written verbatim into an `IStorage`, so it is modelled
/// as a `repr(C)` struct and converted with zerocopy rather than serialised
/// field by field.
#[derive(Debug, Clone, Copy, zerocopy::Immutable, zerocopy::IntoBytes)]
#[repr(C)]
pub struct LibraryAppletArgs {
    /// Struct version. Must be 1; version 0 is not supported.
    pub version: u32,
    /// Size of this struct.
    pub size: u32,
    /// Library applet API version.
    pub la_version: u32,
    /// Theme colour the caller expects the applet to render with.
    pub expected_theme_color: i32,
    /// Whether the applet plays its startup sound.
    pub play_startup_sound: u8,
    _padding: [u8; 7],
    /// System tick at the moment the arguments are pushed.
    pub tick: u64,
}

const_assert_eq!(size_of::<LibraryAppletArgs>(), 0x20);

impl LibraryAppletArgs {
    /// Builds the common arguments for a library applet launched at `tick`.
    ///
    /// `expected_theme_color` is left at zero. libnx sources it from
    /// `appletGetThemeColorType`, which costs another round trip and only
    /// affects the palette the applet renders with, never whether it runs.
    pub const fn new(la_version: u32, tick: u64) -> Self {
        Self {
            version: 1,
            size: size_of::<Self>() as u32,
            la_version,
            expected_theme_color: 0,
            play_startup_sound: 0,
            _padding: [0; 7],
            tick,
        }
    }
}
