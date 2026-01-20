//! Applet Manager (AM) Service for Horizon OS.
//!
//! This crate provides access to the Nintendo Switch's Applet Manager service,
//! the core system component responsible for application lifecycle management,
//! focus coordination, and inter-applet communication.
//!
//! # Overview
//!
//! The Applet Manager (AM) is Horizon OS's **application supervisor**. It doesn't
//! just launch applications—it orchestrates the entire user experience:
//!
//! - **Screen ownership**: Only one applet can have the display at a time
//! - **Focus management**: Coordinates between apps, HOME menu, and system overlays
//! - **Lifecycle control**: Handles suspend/resume based on focus and system state
//! - **Mode changes**: Notifies apps of docking, performance, and display changes
//! - **Inter-applet communication**: Enables launching library applets (keyboard,
//!   error dialogs, etc.) and exchanging data with them
//!
//! # The Two Services
//!
//! AM exposes two service endpoints based on applet type:
//!
//! ## `appletOE` — Application Exclusive
//!
//! Used exclusively by regular applications (games, homebrew). Key characteristics:
//!
//! - **Single session**: Only one application can be connected at a time
//! - **Service name**: `"appletOE"` (`IApplicationProxyService`)
//! - **Used by**: [`AppletType::Application`]
//!
//! ## `appletAE` — Applet Environment
//!
//! Used by all other applet types. Key characteristics:
//!
//! - **Multiple sessions**: Several system components connect simultaneously
//! - **Service name**: `"appletAE"` (`IAllSystemAppletProxiesService`)
//! - **Used by**: [`AppletType::SystemApplet`], [`AppletType::LibraryApplet`],
//!   [`AppletType::OverlayApplet`], [`AppletType::SystemApplication`]
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │                    Applet Manager                          │
//! ├─────────────────────────┬──────────────────────────────────┤
//! │      appletOE           │           appletAE               │
//! │  (Application Proxy     │  (All System Applet Proxies      │
//! │     Service)            │     Service)                     │
//! ├─────────────────────────┼──────────────────────────────────┤
//! │  • One session only     │  • Multiple sessions allowed     │
//! │  • Games & homebrew     │  • qlaunch (HOME menu)           │
//! │                         │  • Library applets (swkbd, etc.) │
//! │                         │  • Overlay applet                │
//! │                         │  • System applications           │
//! └─────────────────────────┴──────────────────────────────────┘
//! ```
//!
//! # Proxy Session Pattern
//!
//! You don't interact with appletOE/appletAE directly. Instead, you request a
//! **proxy session** that provides access to multiple sub-interfaces:
//!
//! ```text
//! appletOE/appletAE
//!     │
//!     └─> OpenXxxProxy(process_handle)  ←── Returns IXxxProxy
//!             │
//!             ├─> GetCommonStateGetter()     → ICommonStateGetter (cmd 0)
//!             ├─> GetSelfController()        → ISelfController (cmd 1)
//!             ├─> GetWindowController()      → IWindowController (cmd 2)
//!             ├─> GetAudioController()       → IAudioController (cmd 3)
//!             ├─> GetDisplayController()     → IDisplayController (cmd 4)
//!             ├─> GetLibraryAppletCreator()  → ILibraryAppletCreator (cmd 11)
//!             └─> GetXxxFunctions()          → Type-specific interface (cmd 20)
//! ```
//!
//! ## Proxy Commands by Applet Type
//!
//! | Type | Service | Command ID |
//! |------|---------|------------|
//! | [`AppletType::Application`] | `appletOE` | 0 |
//! | [`AppletType::SystemApplet`] | `appletAE` | 100 |
//! | [`AppletType::LibraryApplet`] | `appletAE` | 200 (or 201 on HOS 3.0.0+) |
//! | [`AppletType::OverlayApplet`] | `appletAE` | 300 |
//! | [`AppletType::SystemApplication`] | `appletAE` | 350 |
//!
//! # Sub-Interfaces
//!
//! Each proxy provides access to specialized sub-interfaces:
//!
//! ## [`CommonStateGetter`] — "What's happening in the system?"
//!
//! Queries system and applet state:
//!
//! | Command | Name | Purpose |
//! |---------|------|---------|
//! | 0 | `GetEventHandle` | Event signaled when messages are available |
//! | 1 | `ReceiveMessage` | Dequeue an [`AppletMessage`] (error 0x680 if empty) |
//! | 5 | `GetOperationMode` | Handheld vs docked ([`AppletOperationMode`]) |
//! | 6 | `GetPerformanceMode` | Normal vs boost CPU/GPU clocks |
//! | 9 | `GetCurrentFocusState` | Current focus state ([`AppletFocusState`]) |
//!
//! ## [`SelfController`] — "Control my own applet"
//!
//! Manages the applet's own behavior:
//!
//! | Command | Name | Status | Purpose |
//! |---------|------|--------|---------|
//! | 0 | `Exit` | | Clean exit from the applet |
//! | 1-2 | `LockExit`/`UnlockExit` | | Prevent forced closure |
//! | 10 | `SetScreenShotPermission` | | Control screenshot capability |
//! | 11 | `SetOperationModeChangedNotification` | ✅ | Enable handheld/docked notifications |
//! | 12 | `SetPerformanceModeChangedNotification` | ✅ | Enable CPU/GPU clock notifications |
//! | 13 | `SetFocusHandlingMode` | ✅ | Configure suspension behavior |
//! | 16 | `SetOutOfFocusSuspendingEnabled` | ✅ | Enable/disable out-of-focus suspension |
//! | 40 | `CreateManagedDisplayLayer` | | Create a display layer |
//!
//! ## [`WindowController`] — "Manage my display"
//!
//! Display and foreground management:
//!
//! | Command | Name | Status | Purpose |
//! |---------|------|--------|---------|
//! | 1 | `GetAppletResourceUserId` | ✅ | Get the applet resource user ID |
//! | 10 | `AcquireForegroundRights` | ✅ | Claim the foreground display |
//!
//! ## ILibraryAppletCreator — "Launch system dialogs"
//!
//! Create and manage library applets:
//!
//! | Command | Name | Purpose |
//! |---------|------|---------|
//! | 0 | `CreateLibraryApplet` | Launch a library applet by ID |
//! | 1 | `TerminateAllLibraryApplets` | Terminate all created applets |
//! | 10 | `CreateStorage` | Allocate storage for data transfer |
//! | 11 | `CreateTransferMemoryStorage` | Create storage from TransferMemory |
//!
//! # Message System
//!
//! AM communicates with applets through an **asynchronous message queue**. The
//! system posts messages to indicate state changes, and applets poll for them:
//!
//! ```text
//!                     ┌─────────────────┐
//!                     │  Horizon OS     │
//!                     └────────┬────────┘
//!                              │ Posts messages
//!                              ▼
//!                     ┌─────────────────┐
//!                     │  Event Handle   │ ←── From GetEventHandle (cmd 0)
//!                     └────────┬────────┘
//!                              │ Signals when message available
//!                              ▼
//!                     ┌─────────────────┐
//!                     │ ReceiveMessage  │ ←── Returns AppletMessage
//!                     └────────┬────────┘
//!                              │
//!          ┌───────────────────┼───────────────────┐
//!          ▼                   ▼                   ▼
//!     ExitRequest(4)    FocusStateChanged(15)   Resume(16)
//! ```
//!
//! ## Message Types
//!
//! | Value | Name | Meaning |
//! |-------|------|---------|
//! | 4 | `ExitRequest` | System wants the applet to exit |
//! | 15 | `FocusStateChanged` | Focus state changed |
//! | 16 | `Resume` | Applet was suspended, now resuming |
//! | 30 | `OperationModeChanged` | Docked ↔ Handheld transition |
//! | 31 | `PerformanceModeChanged` | CPU/GPU clock changes |
//! | 51 | `RequestToDisplay` | Another applet wants the screen |
//! | 90 | `CaptureButtonShortPressed` | Screenshot button pressed |
//! | 92 | `AlbumScreenShotTaken` | Screenshot was captured |
//!
//! # Focus States and Suspension
//!
//! The [`AppletFocusState`] indicates the applet's visibility and activity:
//!
//! ```text
//!                     ┌─────────────┐
//!          ┌─────────>│  InFocus(1) │<─────────┐
//!          │          └──────┬──────┘          │
//!          │                 │                 │
//!     AcquireForeground      │ LibraryApplet   │ Resume
//!          │                 │ launched        │
//!          │                 ▼                 │
//!          │          ┌─────────────┐          │
//!          │          │OutOfFocus(2)│          │
//!          │          └──────┬──────┘          │
//!          │                 │                 │
//!          │            HOME pressed           │
//!          │            or sleep               │
//!          │                 ▼                 │
//!          │          ┌─────────────┐          │
//!          └──────────│Background(3)│──────────┘
//!                     └─────────────┘
//! ```
//!
//! ## Focus Handling Modes
//!
//! [`AppletFocusHandlingMode`] controls suspension behavior when focus is lost:
//!
//! | Mode | Value | Behavior |
//! |------|-------|----------|
//! | `SuspendHomeSleep` | 0 | Suspend only for HOME menu/sleep (default) |
//! | `NoSuspend` | 1 | Never suspend (useful for audio/background apps) |
//! | `SuspendHomeSleepNotify` | 2 | Suspend for HOME/sleep but receive notifications |
//! | `AlwaysSuspend` | 3 | Suspend whenever out of focus |
//!
//! # Library Applets
//!
//! Library applets are pre-built system UI components that applications can launch
//! for common tasks. They run as separate processes, exchanging data via IStorage.
//!
//! ## Common Library Applets
//!
//! | ID | Name | Purpose |
//! |----|------|---------|
//! | 0x0A | `auth` | Authentication dialogs |
//! | 0x0B | `cabinet` | Amiibo management |
//! | 0x0C | `controller` | Controller pairing/configuration |
//! | 0x0E | `error` | Error display dialogs |
//! | 0x0F | `netConnect` | Network connection wizard |
//! | 0x10 | `playerSelect` | User profile selection |
//! | 0x11 | `swkbd` | Software keyboard |
//! | 0x12 | `miiEdit` | Mii editor |
//! | 0x13 | `web` | Web browser |
//! | 0x14 | `shop` | eShop |
//!
//! ## Library Applet Data Flow
//!
//! Data flows between applets via **IStorage** objects:
//!
//! ```text
//! ┌─────────────┐                      ┌─────────────┐
//! │  Your App   │                      │  LibApplet  │
//! └──────┬──────┘                      └──────┬──────┘
//!        │                                    │
//!        │  CreateStorage(size)               │
//!        │  ──────────────────>               │
//!        │       IStorage                     │
//!        │                                    │
//!        │  Write data to storage             │
//!        │                                    │
//!        │  PushInData(storage)               │
//!        │  ─────────────────────────────────>│
//!        │                                    │
//!        │  Start()                           │
//!        │  ─────────────────────────────────>│
//!        │                                    │
//!        │         [Applet runs]              │
//!        │                                    │
//!        │  Join() / wait for state change    │
//!        │  <─────────────────────────────────│
//!        │                                    │
//!        │  PopOutData()                      │
//!        │  <─────────────────────────────────│
//!        │       IStorage with results        │
//! ```
//!
//! Every library applet receives a **CommonArguments** header (0x20 bytes) containing
//! version info and system tick, followed by applet-specific configuration data.
//!
//! # Application Lifecycle
//!
//! A typical application goes through these phases:
//!
//! ## 1. Initialization
//!
//! ```text
//! crt0 → runtime init → applet init
//!     │
//!     ├─ Connect to appletOE service
//!     ├─ OpenApplicationProxy(process_handle)
//!     ├─ Get sub-interfaces (CommonStateGetter, SelfController, etc.)
//!     ├─ Wait for InFocus state (blocking)
//!     ├─ AcquireForegroundRights()
//!     ├─ SetFocusHandlingMode(SuspendHomeSleep)
//!     └─ NotifyRunning()
//! ```
//!
//! ## 2. Main Loop
//!
//! ```text
//! loop {
//!     // Check for messages via event handle
//!     // Process messages (may trigger exit)
//!     // Poll input
//!     // Update game state
//!     // Render frame
//! }
//! ```
//!
//! The main loop should poll for messages and handle:
//! - `ExitRequest` → clean shutdown
//! - `FocusStateChanged` → update state, possibly pause
//! - `OperationModeChanged` → adjust for dock/undock
//!
//! ## 3. Shutdown
//!
//! ```text
//! exit requested → cleanup → service cleanup
//!     │
//!     ├─ User cleanup code
//!     ├─ SetFocusHandlingMode(NoSuspend)
//!     ├─ Reset CPU boost if used
//!     └─ Close applet service
//! ```
//!
//! # References
//!
//! - [Switchbrew Wiki: Applet Manager services](https://switchbrew.org/wiki/Applet_Manager_services)
//! - [libnx applet.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/services/applet.h)

#![no_std]

extern crate nx_panic_handler; // Provide #![panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Service;
use nx_svc::{ipc::Handle as SessionHandle, process::Handle as ProcessHandle, sync::EventHandle};

use crate::aruid::Aruid;

pub mod aruid;
mod cmif;
mod common_state;
mod proto;

pub use self::{
    cmif::{
        AcquireForegroundRightsError, ConnectError, CreateManagedDisplayLayerError,
        GetAppletResourceUserIdError, GetApplicationFunctionsError, GetCommonStateGetterError,
        GetSelfControllerError, GetWindowControllerError, NotifyRunningError, OpenProxyError,
        SetFocusHandlingModeError, SetOperationModeChangedNotificationError,
        SetOutOfFocusSuspendingEnabledError, SetPerformanceModeChangedNotificationError,
    },
    common_state::{
        GetCurrentFocusStateError, GetEventHandleError, GetOperationModeError,
        GetPerformanceModeError, ReceiveMessageError,
    },
    proto::{
        AppletAttribute, AppletFocusHandlingMode, AppletFocusState, AppletMessage,
        AppletOperationMode, AppletType, SERVICE_NAME_AE, SERVICE_NAME_OE,
    },
};

/// Applet main service session (appletOE or appletAE).
///
/// This is the root service session, converted to domain mode for efficient
/// sub-object management. Use [`open_proxy`] to get a proxy for your applet type.
#[repr(transparent)]
pub struct AppletService(Service);

impl AppletService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Consumes and closes the applet service.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Opens a proxy session for the specified applet type.
    ///
    /// The proxy provides access to sub-interfaces like ICommonStateGetter,
    /// ISelfController, etc.
    ///
    /// # Arguments
    ///
    /// * `applet_type` - The type of applet (must not be `None` or `Default`)
    /// * `process_handle` - The current process handle (usually `CUR_PROCESS_HANDLE`)
    #[inline]
    pub fn open_proxy(
        &self,
        applet_type: AppletType,
        process_handle: ProcessHandle,
    ) -> Result<AppletProxyService, OpenProxyError> {
        cmif::open_proxy(&self.0, applet_type, process_handle, None)
    }

    /// Opens a library applet proxy with attributes (3.0.0+).
    ///
    /// Use this for `LibraryApplet` type on HOS 3.0.0 or later.
    #[inline]
    pub fn open_library_applet_proxy(
        &self,
        process_handle: ProcessHandle,
        attr: &AppletAttribute,
    ) -> Result<AppletProxyService, OpenProxyError> {
        cmif::open_proxy(
            &self.0,
            AppletType::LibraryApplet,
            process_handle,
            Some(attr),
        )
    }
}

/// Applet proxy session.
///
/// The proxy provides access to all the sub-interfaces for managing the applet:
/// - `ICommonStateGetter` - Focus state, operation mode, messages
/// - `ISelfController` - Focus handling, screenshots, etc.
/// - `IWindowController` - Foreground rights
/// - And more depending on applet type
#[repr(transparent)]
pub struct AppletProxyService(Service);

impl AppletProxyService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Returns the domain object ID of this proxy (0 if non-domain).
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.0.object_id
    }

    /// Consumes and closes the proxy service.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Gets the ICommonStateGetter sub-interface.
    ///
    /// Provides access to focus state, operation mode, and message events.
    #[inline]
    pub fn get_common_state_getter(&self) -> Result<CommonStateGetter, GetCommonStateGetterError> {
        cmif::get_common_state_getter(&self.0)
    }

    /// Gets the ISelfController sub-interface.
    ///
    /// Provides control over focus handling, screenshots, and more.
    #[inline]
    pub fn get_self_controller(&self) -> Result<SelfController, GetSelfControllerError> {
        cmif::get_self_controller(&self.0)
    }

    /// Gets the IWindowController sub-interface.
    ///
    /// Provides control over foreground display rights.
    #[inline]
    pub fn get_window_controller(&self) -> Result<WindowController, GetWindowControllerError> {
        cmif::get_window_controller(&self.0)
    }

    /// Gets the IApplicationFunctions sub-interface (Application type only).
    ///
    /// Provides application-specific functionality like NotifyRunning.
    /// Only available for `AppletType::Application` via appletOE.
    #[inline]
    pub fn get_application_functions(
        &self,
    ) -> Result<ApplicationFunctions, GetApplicationFunctionsError> {
        cmif::get_application_functions(&self.0)
    }
}

/// ICommonStateGetter sub-interface.
///
/// Provides access to:
/// - Message event handle for notifications
/// - Current focus state
/// - Operation mode (handheld/docked)
/// - Performance mode
#[repr(transparent)]
pub struct CommonStateGetter(Service);

impl CommonStateGetter {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Returns the domain object ID (0 if non-domain).
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.0.object_id
    }

    /// Consumes and closes the interface.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Gets the message event handle.
    ///
    /// This event is signaled when the applet receives a message.
    /// Use with `ReceiveMessage` to get the actual message.
    #[inline]
    pub fn get_event_handle(&self) -> Result<EventHandle, GetEventHandleError> {
        common_state::get_event_handle(&self.0)
    }

    /// Receives a pending message.
    ///
    /// Returns `Ok(None)` if no message is pending.
    #[inline]
    pub fn receive_message(&self) -> Result<Option<AppletMessage>, ReceiveMessageError> {
        common_state::receive_message(&self.0)
    }

    /// Gets the current operation mode (handheld/docked).
    #[inline]
    pub fn get_operation_mode(&self) -> Result<AppletOperationMode, GetOperationModeError> {
        common_state::get_operation_mode(&self.0)
    }

    /// Gets the current performance mode.
    #[inline]
    pub fn get_performance_mode(&self) -> Result<u32, GetPerformanceModeError> {
        common_state::get_performance_mode(&self.0)
    }

    /// Gets the current focus state.
    #[inline]
    pub fn get_current_focus_state(&self) -> Result<AppletFocusState, GetCurrentFocusStateError> {
        common_state::get_current_focus_state(&self.0)
    }
}

/// ISelfController sub-interface.
///
/// Provides control over:
/// - Focus handling mode
/// - Out-of-focus suspending
/// - Screenshots
/// - And more
#[repr(transparent)]
pub struct SelfController(Service);

impl SelfController {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Returns the domain object ID (0 if non-domain).
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.0.object_id
    }

    /// Consumes and closes the interface.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Sets the focus handling mode.
    ///
    /// This controls when the applet suspends based on focus state.
    /// Only valid for Application applet type.
    #[inline]
    pub fn set_focus_handling_mode(
        &self,
        mode: AppletFocusHandlingMode,
    ) -> Result<(), SetFocusHandlingModeError> {
        cmif::set_focus_handling_mode(&self.0, mode)
    }

    /// Sets whether to suspend when out of focus (2.0.0+).
    ///
    /// Only valid for Application applet type.
    #[inline]
    pub fn set_out_of_focus_suspending_enabled(
        &self,
        enabled: bool,
    ) -> Result<(), SetOutOfFocusSuspendingEnabledError> {
        cmif::set_out_of_focus_suspending_enabled(&self.0, enabled)
    }

    /// Enables or disables operation mode change notifications.
    ///
    /// When enabled, the applet receives `OperationModeChanged` messages
    /// when the console transitions between handheld and docked modes.
    ///
    /// Called during applet initialization (typically with `true`).
    #[inline]
    pub fn set_operation_mode_changed_notification(
        &self,
        enabled: bool,
    ) -> Result<(), SetOperationModeChangedNotificationError> {
        cmif::set_operation_mode_changed_notification(&self.0, enabled)
    }

    /// Enables or disables performance mode change notifications.
    ///
    /// When enabled, the applet receives `PerformanceModeChanged` messages
    /// when CPU/GPU clock speeds change.
    ///
    /// Called during applet initialization (typically with `true`).
    #[inline]
    pub fn set_performance_mode_changed_notification(
        &self,
        enabled: bool,
    ) -> Result<(), SetPerformanceModeChangedNotificationError> {
        cmif::set_performance_mode_changed_notification(&self.0, enabled)
    }

    /// Creates a managed display layer.
    ///
    /// Returns the layer ID on success.
    #[inline]
    pub fn create_managed_display_layer(&self) -> Result<u64, CreateManagedDisplayLayerError> {
        cmif::create_managed_display_layer(&self.0)
    }
}

/// IWindowController sub-interface.
///
/// Provides control over foreground display rights.
#[repr(transparent)]
pub struct WindowController(Service);

impl WindowController {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Returns the domain object ID (0 if non-domain).
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.0.object_id
    }

    /// Consumes and closes the interface.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Gets the applet resource user ID.
    ///
    /// This ID is used by various system services (HID, audio, NV, etc.) to
    /// identify the applet. It's obtained during applet initialization and
    /// typically stored globally for later use.
    ///
    /// Returns `Ok(None)` if the system returns ARUID 0 (invalid).
    #[inline]
    pub fn get_applet_resource_user_id(
        &self,
    ) -> Result<Option<Aruid>, GetAppletResourceUserIdError> {
        cmif::get_applet_resource_user_id(&self.0)
    }

    /// Acquires foreground display rights.
    ///
    /// Must be called after waiting for `InFocus` state during initialization.
    #[inline]
    pub fn acquire_foreground_rights(&self) -> Result<(), AcquireForegroundRightsError> {
        cmif::acquire_foreground_rights(&self.0)
    }
}

/// IApplicationFunctions interface (Application type only).
///
/// Provides application-specific functionality like NotifyRunning.
/// Only available for `AppletType::Application` via appletOE.
#[repr(transparent)]
pub struct ApplicationFunctions(Service);

impl ApplicationFunctions {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.session
    }

    /// Returns the domain object ID (0 if non-domain).
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.0.object_id
    }

    /// Consumes and closes the interface.
    #[inline]
    pub fn close(self) {
        self.0.close();
    }

    /// Notifies the system that the application has completed initialization
    /// and is ready to run.
    ///
    /// This should be called after waiting for InFocus, acquiring foreground rights,
    /// and setting up focus handling mode.
    #[inline]
    pub fn notify_running(&self) -> Result<bool, NotifyRunningError> {
        cmif::notify_running(&self.0)
    }
}

/// Connects to the applet service (appletOE or appletAE) based on applet type.
///
/// The service is automatically converted to domain mode for efficient
/// sub-object management.
///
/// # Arguments
///
/// * `sm` - Service Manager session
/// * `applet_type` - The type of applet (determines which service to connect to)
///
/// # Returns
///
/// Returns `Ok(None)` if `applet_type` is `AppletType::None`.
pub fn connect(
    sm: &SmService,
    applet_type: AppletType,
) -> Result<Option<AppletService>, ConnectError> {
    if matches!(applet_type, AppletType::None) {
        return Ok(None);
    }

    // Determine which service to connect to
    let service_name = if applet_type.uses_applet_oe() {
        SERVICE_NAME_OE
    } else {
        SERVICE_NAME_AE
    };

    // Get service handle from SM
    let handle = sm
        .get_service_handle_cmif(service_name)
        .map_err(ConnectError::GetService)?;

    // Create service and convert to domain
    let mut service = Service {
        session: handle,
        own_handle: 1,
        object_id: 0,
        pointer_buffer_size: 0,
    };

    service
        .convert_to_domain()
        .map_err(ConnectError::ConvertToDomain)?;

    Ok(Some(AppletService(service)))
}
