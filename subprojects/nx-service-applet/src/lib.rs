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
//! | 93 | `AlbumRecordingSaved` | Album recording was saved |
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

use core::mem::ManuallyDrop;

use nx_service_sm::SmService;
use nx_sf::service::{Domain, DomainObject, Session};
use nx_svc::{ipc::Handle as SessionHandle, process::Handle as ProcessHandle, sync::EventHandle};

use crate::aruid::Aruid;

pub mod aruid;
mod cmif;
mod common_state;
mod proto;
pub mod proxy;
pub mod role;

pub use self::{
    cmif::{
        AcquireForegroundRightsError, ConnectError, CreateManagedDisplayLayerError,
        GetAppletResourceUserIdError, GetApplicationFunctionsError, GetCommonStateGetterError,
        GetSelfControllerError, GetSubInterfaceError, GetWindowControllerError, NotifyRunningError,
        OpenProxyError, SetFocusHandlingModeError, SetOperationModeChangedNotificationError,
        SetOutOfFocusSuspendingEnabledError, SetPerformanceModeChangedNotificationError,
    },
    common_state::{
        GetCurrentFocusStateError, GetEventHandleError, GetOperationModeError,
        GetPerformanceModeError, ReceiveMessageError,
    },
    proto::{
        AppletAttribute, AppletFocusHandlingMode, AppletFocusState, AppletMessage,
        AppletOperationMode, AppletPerformanceMode, AppletType, SERVICE_NAME_AE, SERVICE_NAME_OE,
    },
};

/// Builds a `ManuallyDrop<Domain>` aliasing the kernel handle and pointer-buffer
/// size of `parent`. The returned domain shares the parent's kernel handle and
/// must NOT be dropped — its [`Drop`] is suppressed by [`ManuallyDrop`] so the
/// underlying session is closed exactly once by the owning root [`AppletService`].
#[inline]
fn alias_domain(parent: &Domain) -> ManuallyDrop<Domain> {
    // SAFETY: We are creating an alias of an already-converted domain handle.
    // The alias' [`Drop`] is suppressed via [`ManuallyDrop`]; the real owner
    // (the root [`AppletService`]) closes the kernel handle.
    ManuallyDrop::new(unsafe {
        Domain::from_handle_unchecked(parent.handle(), parent.pointer_buffer_size())
    })
}

/// Defines a sub-interface wrapper that aliases the root domain's kernel handle
/// and addresses a specific domain object id within it.
///
/// All "stub" sub-interfaces added to provide access to libnx sub-services follow
/// the same shape — they share the root domain's session and dispatch against
/// their `object_id`. The wrappers do not own the kernel handle: the server-side
/// objects are released when the owning [`AppletService`] (which owns the root
/// [`Domain`]) is dropped and the kernel cascades object close on its side.
/// Per-command methods can be added later as consumers need them.
macro_rules! sub_interface_stub {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub struct $name {
            /// Alias of the root domain (close suppressed via [`ManuallyDrop`]).
            domain: ManuallyDrop<Domain>,
            /// Domain object id this wrapper addresses.
            object_id: u32,
        }

        impl $name {
            /// Returns the underlying session handle.
            #[inline]
            pub fn session(&self) -> SessionHandle {
                self.domain.handle()
            }

            /// Returns the domain object ID.
            #[inline]
            pub fn object_id(&self) -> u32 {
                self.object_id
            }

            /// Borrowed view onto this wrapper's domain object.
            ///
            /// The per-object close is suppressed via [`ManuallyDrop`]; the
            /// underlying server-side object is closed implicitly when the
            /// owning root [`AppletService`] is dropped.
            #[inline]
            #[allow(dead_code)]
            fn as_object(&self) -> ManuallyDrop<DomainObject<'_>> {
                // SAFETY: `object_id` was returned by the server within this
                // sub-interface's parent domain; the `ManuallyDrop` view is
                // the only live `DomainObject` for this id at a time.
                let object = unsafe { self.domain.open_object_raw(self.object_id) }
                    .expect("sub-interface holds a non-zero domain object id");
                ManuallyDrop::new(object)
            }
        }
    };
}

sub_interface_stub! {
    /// IAudioController sub-interface (proxy cmd 3).
    ///
    /// Available for all applet types. Methods (volume control, transparent volume rate)
    /// are added on demand; see libnx `appletSetExpectedMasterVolume`/`appletGetExpectedMasterVolume`.
    AudioController
}

sub_interface_stub! {
    /// IDisplayController sub-interface (proxy cmd 4).
    ///
    /// Capture-buffer and screenshot operations live behind this interface; see libnx
    /// `appletTakeScreenShotOfOwnLayer`/`appletAcquireLastForegroundCaptureSharedBuffer` etc.
    DisplayController
}

sub_interface_stub! {
    /// IProcessWindingController sub-interface (proxy cmd 10).
    ///
    /// LibraryApplet only. Used by `appletPushContext`/`appletPopContext`.
    ProcessWindingController
}

sub_interface_stub! {
    /// ILibraryAppletCreator sub-interface (proxy cmd 11).
    ///
    /// Available for all applet types. Used to launch library applets (swkbd, error
    /// dialog, …) and create IStorage objects for inter-applet data transfer.
    LibraryAppletCreator
}

sub_interface_stub! {
    /// ILibraryAppletSelfAccessor sub-interface (proxy cmd 20, LibraryApplet only,
    /// pre-15.0.0).
    ///
    /// Provides storage exchange, identity info, and exit-to-self for library applets.
    LibraryAppletSelfAccessor
}

sub_interface_stub! {
    /// IAppletCommonFunctions sub-interface (proxy cmd 21 or 23, HOS 7.0.0+).
    ///
    /// Available for SystemApplet/LibraryApplet/OverlayApplet. The proxy command differs
    /// by applet type — see [`AppletProxyService::get_applet_common_functions`].
    AppletCommonFunctions
}

sub_interface_stub! {
    /// IGlobalStateController sub-interface (proxy cmd 21 or 23).
    ///
    /// Available for SystemApplet (always, cmd 21) and LibraryApplet/OverlayApplet
    /// (HOS 15.0.0+, cmd 23). Sleep/shutdown/reboot sequencing lives here.
    GlobalStateController
}

sub_interface_stub! {
    /// IApplicationCreator sub-interface (proxy cmd 22, SystemApplet only).
    ///
    /// Used by `qlaunch` and similar system applets to spawn/launch applications.
    ApplicationCreator
}

sub_interface_stub! {
    /// IHomeMenuFunctions sub-interface (proxy cmd 22, LibraryApplet on HOS 15.0.0+).
    ///
    /// Replaces `IFunctions` for LibraryApplet starting in 15.0.0.
    HomeMenuFunctions
}

sub_interface_stub! {
    /// IDebugFunctions sub-interface (proxy cmd 1000).
    ///
    /// Available for all applet types. Debug-only commands (system-button injection,
    /// general-storage probing, etc.).
    DebugFunctions
}

/// Applet main service session (appletOE or appletAE).
///
/// This is the root service session, converted to domain mode for efficient
/// sub-object management. Use [`open_proxy`] to get a proxy for your applet type.
/// Dropping the service closes the underlying kernel session; the server cascades
/// object close on its side for every sub-object opened against this domain.
#[repr(transparent)]
pub struct AppletService(Domain);

impl AppletService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
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
///
/// The proxy is a domain object of the root [`AppletService`] and is closed
/// implicitly when the owning [`AppletService`] is dropped.
pub struct AppletProxyService {
    /// Alias of the root domain (close suppressed via [`ManuallyDrop`]).
    domain: ManuallyDrop<Domain>,
    /// Domain object id of the proxy.
    object_id: u32,
}

impl AppletProxyService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.domain.handle()
    }

    /// Returns the domain object ID of this proxy.
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    /// Borrowed view onto the proxy domain object. Per-object close is
    /// suppressed; the proxy is closed when the parent [`AppletService`] drops.
    #[inline]
    fn as_object(&self) -> ManuallyDrop<DomainObject<'_>> {
        // SAFETY: `object_id` was returned by the server within this proxy's
        // parent domain; the `ManuallyDrop` view is the only live
        // `DomainObject` for this id at a time.
        let object = unsafe { self.domain.open_object_raw(self.object_id) }
            .expect("AppletProxyService holds a non-zero domain object id");
        ManuallyDrop::new(object)
    }

    /// Gets the ICommonStateGetter sub-interface.
    ///
    /// Provides access to focus state, operation mode, and message events.
    #[inline]
    pub fn get_common_state_getter(&self) -> Result<CommonStateGetter, GetCommonStateGetterError> {
        cmif::get_common_state_getter(&self.as_object())
    }

    /// Gets the ISelfController sub-interface.
    ///
    /// Provides control over focus handling, screenshots, and more.
    #[inline]
    pub fn get_self_controller(&self) -> Result<SelfController, GetSelfControllerError> {
        cmif::get_self_controller(&self.as_object())
    }

    /// Gets the IWindowController sub-interface.
    ///
    /// Provides control over foreground display rights.
    #[inline]
    pub fn get_window_controller(&self) -> Result<WindowController, GetWindowControllerError> {
        cmif::get_window_controller(&self.as_object())
    }

    /// Gets the IApplicationFunctions sub-interface (Application type only).
    ///
    /// Provides application-specific functionality like NotifyRunning.
    /// Only available for `AppletType::Application` via appletOE.
    #[inline]
    pub fn get_application_functions(
        &self,
    ) -> Result<ApplicationFunctions, GetApplicationFunctionsError> {
        cmif::get_application_functions(&self.as_object())
    }

    /// Gets the IAudioController sub-interface (cmd 3, all applet types).
    #[inline]
    pub fn get_audio_controller(&self) -> Result<AudioController, GetSubInterfaceError> {
        cmif::get_audio_controller(&self.as_object())
    }

    /// Gets the IDisplayController sub-interface (cmd 4, all applet types).
    #[inline]
    pub fn get_display_controller(&self) -> Result<DisplayController, GetSubInterfaceError> {
        cmif::get_display_controller(&self.as_object())
    }

    /// Gets the IProcessWindingController sub-interface (cmd 10, LibraryApplet only).
    #[inline]
    pub fn get_process_winding_controller(
        &self,
    ) -> Result<ProcessWindingController, GetSubInterfaceError> {
        cmif::get_process_winding_controller(&self.as_object())
    }

    /// Gets the ILibraryAppletCreator sub-interface (cmd 11, all applet types).
    #[inline]
    pub fn get_library_applet_creator(&self) -> Result<LibraryAppletCreator, GetSubInterfaceError> {
        cmif::get_library_applet_creator(&self.as_object())
    }

    /// Gets the ILibraryAppletSelfAccessor sub-interface (cmd 20, LibraryApplet only,
    /// pre-15.0.0).
    #[inline]
    pub fn get_library_applet_self_accessor(
        &self,
    ) -> Result<LibraryAppletSelfAccessor, GetSubInterfaceError> {
        cmif::get_library_applet_self_accessor(&self.as_object())
    }

    /// Gets the IAppletCommonFunctions sub-interface (HOS 7.0.0+, non-Application).
    ///
    /// `applet_type` selects the right proxy command: SystemApplet uses cmd 23,
    /// other types use cmd 21. Returns [`GetSubInterfaceError::Dispatch`] if the
    /// HOS is older than 7.0.0 or the applet type is not supported.
    #[inline]
    pub fn get_applet_common_functions(
        &self,
        applet_type: AppletType,
    ) -> Result<AppletCommonFunctions, GetSubInterfaceError> {
        cmif::get_applet_common_functions(&self.as_object(), applet_type)
    }

    /// Gets the IGlobalStateController sub-interface.
    ///
    /// `applet_type` selects the proxy command: SystemApplet uses cmd 21,
    /// LibraryApplet/OverlayApplet use cmd 23 (HOS 15.0.0+).
    #[inline]
    pub fn get_global_state_controller(
        &self,
        applet_type: AppletType,
    ) -> Result<GlobalStateController, GetSubInterfaceError> {
        cmif::get_global_state_controller(&self.as_object(), applet_type)
    }

    /// Gets the IApplicationCreator sub-interface (cmd 22, SystemApplet only).
    #[inline]
    pub fn get_application_creator(&self) -> Result<ApplicationCreator, GetSubInterfaceError> {
        cmif::get_application_creator(&self.as_object())
    }

    /// Gets the IHomeMenuFunctions sub-interface (cmd 22, LibraryApplet on 15.0.0+).
    #[inline]
    pub fn get_home_menu_functions(&self) -> Result<HomeMenuFunctions, GetSubInterfaceError> {
        cmif::get_home_menu_functions(&self.as_object())
    }

    /// Gets the IDebugFunctions sub-interface (cmd 1000, all applet types).
    #[inline]
    pub fn get_debug_functions(&self) -> Result<DebugFunctions, GetSubInterfaceError> {
        cmif::get_debug_functions(&self.as_object())
    }
}

/// ICommonStateGetter sub-interface.
///
/// Provides access to:
/// - Message event handle for notifications
/// - Current focus state
/// - Operation mode (handheld/docked)
/// - Performance mode
///
/// Closed implicitly when the owning [`AppletService`] is dropped.
pub struct CommonStateGetter {
    domain: ManuallyDrop<Domain>,
    object_id: u32,
}

impl CommonStateGetter {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.domain.handle()
    }

    /// Returns the domain object ID.
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    #[inline]
    fn as_object(&self) -> ManuallyDrop<DomainObject<'_>> {
        // SAFETY: see `AppletProxyService::as_object`.
        let object = unsafe { self.domain.open_object_raw(self.object_id) }
            .expect("CommonStateGetter holds a non-zero domain object id");
        ManuallyDrop::new(object)
    }

    /// Gets the message event handle.
    ///
    /// This event is signaled when the applet receives a message.
    /// Use with `ReceiveMessage` to get the actual message.
    #[inline]
    pub fn get_event_handle(&self) -> Result<EventHandle, GetEventHandleError> {
        common_state::get_event_handle(&self.as_object())
    }

    /// Receives a pending message.
    ///
    /// Returns `Ok(None)` if no message is pending.
    #[inline]
    pub fn receive_message(&self) -> Result<Option<AppletMessage>, ReceiveMessageError> {
        common_state::receive_message(&self.as_object())
    }

    /// Gets the current operation mode (handheld/docked).
    #[inline]
    pub fn get_operation_mode(&self) -> Result<AppletOperationMode, GetOperationModeError> {
        common_state::get_operation_mode(&self.as_object())
    }

    /// Gets the current performance mode.
    #[inline]
    pub fn get_performance_mode(&self) -> Result<AppletPerformanceMode, GetPerformanceModeError> {
        common_state::get_performance_mode(&self.as_object())
    }

    /// Gets the current focus state.
    #[inline]
    pub fn get_current_focus_state(&self) -> Result<AppletFocusState, GetCurrentFocusStateError> {
        common_state::get_current_focus_state(&self.as_object())
    }
}

/// ISelfController sub-interface.
///
/// Provides control over:
/// - Focus handling mode
/// - Out-of-focus suspending
/// - Screenshots
/// - And more
///
/// Closed implicitly when the owning [`AppletService`] is dropped.
pub struct SelfController {
    domain: ManuallyDrop<Domain>,
    object_id: u32,
}

impl SelfController {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.domain.handle()
    }

    /// Returns the domain object ID.
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    #[inline]
    fn as_object(&self) -> ManuallyDrop<DomainObject<'_>> {
        // SAFETY: see `AppletProxyService::as_object`.
        let object = unsafe { self.domain.open_object_raw(self.object_id) }
            .expect("SelfController holds a non-zero domain object id");
        ManuallyDrop::new(object)
    }

    /// Sets the focus handling mode.
    ///
    /// This controls when the applet suspends based on focus state.
    ///
    /// # Applet-type contract
    ///
    /// libnx rejects this call for any applet type other than `Application`
    /// (`appletSetFocusHandlingMode` returns `LibnxError_NotInitialized`).
    /// `nx-service-applet` does not enforce that here; callers must
    /// gate this on `AppletType::Application` (the `nx-rt` wrapper does so).
    ///
    /// Internally issues two IPC dispatches (cmd 13 and, on HOS 2.0.0+,
    /// cmd 16 via the companion `SetOutOfFocusSuspendingEnabled`), mirroring
    /// libnx `appletSetFocusHandlingMode`.
    #[inline]
    pub fn set_focus_handling_mode(
        &self,
        mode: AppletFocusHandlingMode,
    ) -> Result<(), SetFocusHandlingModeError> {
        cmif::set_focus_handling_mode(&self.as_object(), mode)
    }

    /// Sets whether to suspend when out of focus (HOS 2.0.0+).
    ///
    /// # Version contract
    ///
    /// The underlying command (cmd 16) was introduced in HOS 2.0.0. Callers are
    /// responsible for HOS version-gating; this primitive does not check.
    ///
    /// # Applet-type contract
    ///
    /// Only valid for `AppletType::Application`. `nx-service-applet` does not
    /// enforce that here; the `nx-rt` wrapper does.
    #[inline]
    pub fn set_out_of_focus_suspending_enabled(
        &self,
        enabled: bool,
    ) -> Result<(), SetOutOfFocusSuspendingEnabledError> {
        cmif::set_out_of_focus_suspending_enabled(&self.as_object(), enabled)
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
        cmif::set_operation_mode_changed_notification(&self.as_object(), enabled)
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
        cmif::set_performance_mode_changed_notification(&self.as_object(), enabled)
    }

    /// Creates a managed display layer.
    ///
    /// Returns the layer ID on success.
    #[inline]
    pub fn create_managed_display_layer(&self) -> Result<u64, CreateManagedDisplayLayerError> {
        cmif::create_managed_display_layer(&self.as_object())
    }
}

/// IWindowController sub-interface.
///
/// Provides control over foreground display rights.
///
/// Closed implicitly when the owning [`AppletService`] is dropped.
pub struct WindowController {
    domain: ManuallyDrop<Domain>,
    object_id: u32,
}

impl WindowController {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.domain.handle()
    }

    /// Returns the domain object ID.
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    #[inline]
    fn as_object(&self) -> ManuallyDrop<DomainObject<'_>> {
        // SAFETY: see `AppletProxyService::as_object`.
        let object = unsafe { self.domain.open_object_raw(self.object_id) }
            .expect("WindowController holds a non-zero domain object id");
        ManuallyDrop::new(object)
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
        cmif::get_applet_resource_user_id(&self.as_object())
    }

    /// Acquires foreground display rights.
    ///
    /// Must be called after waiting for `InFocus` state during initialization.
    #[inline]
    pub fn acquire_foreground_rights(&self) -> Result<(), AcquireForegroundRightsError> {
        cmif::acquire_foreground_rights(&self.as_object())
    }
}

/// IApplicationFunctions interface (Application type only).
///
/// Provides application-specific functionality like NotifyRunning.
/// Only available for `AppletType::Application` via appletOE.
///
/// Closed implicitly when the owning [`AppletService`] is dropped.
pub struct ApplicationFunctions {
    domain: ManuallyDrop<Domain>,
    object_id: u32,
}

impl ApplicationFunctions {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.domain.handle()
    }

    /// Returns the domain object ID.
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    #[inline]
    fn as_object(&self) -> ManuallyDrop<DomainObject<'_>> {
        // SAFETY: see `AppletProxyService::as_object`.
        let object = unsafe { self.domain.open_object_raw(self.object_id) }
            .expect("ApplicationFunctions holds a non-zero domain object id");
        ManuallyDrop::new(object)
    }

    /// Notifies the system that the application has completed initialization
    /// and is ready to run.
    ///
    /// This should be called after waiting for InFocus, acquiring foreground rights,
    /// and setting up focus handling mode. Only valid for `AppletType::Application`.
    #[inline]
    pub fn notify_running(&self) -> Result<bool, NotifyRunningError> {
        cmif::notify_running(&self.as_object())
    }
}

/// Connects to the applet service (appletOE or appletAE) based on applet type.
///
/// The service is automatically converted to domain mode for efficient
/// sub-object management.
///
/// `AppletType::Default` is coerced to `AppletType::Application`, matching
/// libnx's `_appletInitialize` behavior.
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

    // Coerce Default → Application, matching libnx _appletInitialize behavior.
    let applet_type = if matches!(applet_type, AppletType::Default) {
        AppletType::Application
    } else {
        applet_type
    };

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

    // Build an owned session (the pointer-buffer-size query is internal) and
    // promote it to a domain. On failure the inner [`Session`] is dropped,
    // closing the kernel handle.
    let session = Session::new(handle);
    let domain = session
        .convert_to_domain()
        .map_err(|(_session, err)| ConnectError::ConvertToDomain(err))?;

    Ok(Some(AppletService(domain)))
}
