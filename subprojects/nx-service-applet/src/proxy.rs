//! Role-typed wrapper over an opened applet proxy session.
//!
//! [`Proxy<R>`] bundles the root [`AppletService`], the [`AppletProxyService`]
//! returned by `Open*Proxy`, and every sub-interface the role admits. The `R`
//! type parameter pins the role; method sets defined per `Proxy<R>` then
//! reflect exactly what AM accepts for that role.
//!
//! Build a `Proxy<R>` with one of the role-specific open functions:
//!
//! * [`open_application`]
//! * [`open_library_applet`]
//! * [`open_system_applet`]
//! * [`open_overlay_applet`]
//! * [`open_system_application`]
//!
//! Each performs `SM → service → proxy → core seven sub-interfaces →
//! role-specific extras` as a single composite IPC choreography, mirroring
//! libnx's `_appletInitialize` plumbing minus the runtime policy steps
//! (InFocus wait, AcquireForegroundRights, NotifyRunning, mode notifications)
//! which are the consumer's responsibility.

use core::marker::PhantomData;

use nx_service_sm::SmService;
use nx_sf::error::{GENERIC_ERROR, ToResultCode};
use nx_svc::{error::ResultCode, process::Handle as ProcessHandle};

use crate::{
    AcquireForegroundRightsError, AppletFocusHandlingMode, AppletProxyService, AppletService,
    AudioController, CommonStateGetter, ConnectError, DebugFunctions, DisplayController,
    GetCommonStateGetterError, GetSelfControllerError, GetSubInterfaceError,
    GetWindowControllerError, LibraryAppletCreator, NotifyRunningError, OpenProxyError,
    SelfController, SetFocusHandlingModeError, SetOutOfFocusSuspendingEnabledError,
    WindowController,
    aruid::Aruid,
    role::{
        Application, DrainExtrasError, LibraryApplet, OverlayApplet, Role, SystemApplet,
        SystemApplication,
    },
};

/// The seven sub-interfaces every AM role exposes (proxy cmds 0–4, 11, 1000).
pub struct CoreSubInterfaces {
    pub common_state_getter: CommonStateGetter,
    pub self_controller: SelfController,
    pub window_controller: WindowController,
    pub audio_controller: AudioController,
    pub display_controller: DisplayController,
    pub library_applet_creator: LibraryAppletCreator,
    pub debug_functions: DebugFunctions,
}

impl CoreSubInterfaces {
    /// Opens all seven core sub-interfaces from `proxy`.
    fn open(proxy: &AppletProxyService) -> Result<Self, OpenError> {
        let common_state_getter = proxy
            .get_common_state_getter()
            .map_err(OpenError::GetCommonStateGetter)?;
        let self_controller = proxy
            .get_self_controller()
            .map_err(OpenError::GetSelfController)?;
        let window_controller = proxy
            .get_window_controller()
            .map_err(OpenError::GetWindowController)?;
        let audio_controller = proxy
            .get_audio_controller()
            .map_err(OpenError::GetSubInterface)?;
        let display_controller = proxy
            .get_display_controller()
            .map_err(OpenError::GetSubInterface)?;
        let library_applet_creator = proxy
            .get_library_applet_creator()
            .map_err(OpenError::GetSubInterface)?;
        let debug_functions = proxy
            .get_debug_functions()
            .map_err(OpenError::GetSubInterface)?;
        Ok(Self {
            common_state_getter,
            self_controller,
            window_controller,
            audio_controller,
            display_controller,
            library_applet_creator,
            debug_functions,
        })
    }
}

/// Role-typed proxy bundling every IPC handle AM admits for role `R`.
pub struct Proxy<R: Role> {
    service: AppletService,
    proxy: AppletProxyService,
    core: CoreSubInterfaces,
    extras: R::Extras,
    _role: PhantomData<R>,
}

impl<R: Role> Proxy<R> {
    fn from_session(service: AppletService, proxy: AppletProxyService) -> Result<Self, OpenError> {
        let core = CoreSubInterfaces::open(&proxy)?;
        let extras = R::drain_extras(&proxy).map_err(OpenError::DrainExtras)?;
        Ok(Self {
            service,
            proxy,
            core,
            extras,
            _role: PhantomData,
        })
    }

    /// Returns the underlying root [`AppletService`] (appletOE/appletAE).
    #[inline]
    pub fn service(&self) -> &AppletService {
        &self.service
    }

    /// Returns the underlying [`AppletProxyService`].
    #[inline]
    pub fn proxy_service(&self) -> &AppletProxyService {
        &self.proxy
    }

    /// Returns the role-specific extras (read-only).
    #[inline]
    pub fn extras(&self) -> &R::Extras {
        &self.extras
    }

    #[inline]
    pub fn common_state_getter(&self) -> &CommonStateGetter {
        &self.core.common_state_getter
    }

    #[inline]
    pub fn self_controller(&self) -> &SelfController {
        &self.core.self_controller
    }

    #[inline]
    pub fn window_controller(&self) -> &WindowController {
        &self.core.window_controller
    }

    #[inline]
    pub fn audio_controller(&self) -> &AudioController {
        &self.core.audio_controller
    }

    #[inline]
    pub fn display_controller(&self) -> &DisplayController {
        &self.core.display_controller
    }

    #[inline]
    pub fn library_applet_creator(&self) -> &LibraryAppletCreator {
        &self.core.library_applet_creator
    }

    #[inline]
    pub fn debug_functions(&self) -> &DebugFunctions {
        &self.core.debug_functions
    }

    /// `IWindowController::AcquireForegroundRights` (cmd 10).
    ///
    /// AM only acts on this for the foreground-eligible roles; non-Application
    /// callers will receive an AM-side error.
    #[inline]
    pub fn acquire_foreground_rights(&self) -> Result<(), AcquireForegroundRightsError> {
        self.window_controller().acquire_foreground_rights()
    }
}

impl Proxy<Application> {
    /// `IApplicationFunctions` (proxy cmd 20).
    #[inline]
    pub fn application_functions(&self) -> &crate::ApplicationFunctions {
        &self.extras.application_functions
    }

    /// `IApplicationFunctions::NotifyRunning` — Application-only.
    #[inline]
    pub fn notify_running(&self) -> Result<bool, NotifyRunningError> {
        self.application_functions().notify_running()
    }

    /// `ISelfController::SetFocusHandlingMode` (cmd 13 + cmd 16 on HOS 2.0.0+).
    ///
    /// AM rejects this for any role other than Application
    /// (`LibnxError_NotInitialized` in libnx); the typestate enforces the
    /// same restriction at compile time on the Rust side.
    #[inline]
    pub fn set_focus_handling_mode(
        &self,
        mode: AppletFocusHandlingMode,
    ) -> Result<(), SetFocusHandlingModeError> {
        self.self_controller().set_focus_handling_mode(mode)
    }

    /// `ISelfController::SetOutOfFocusSuspendingEnabled` (cmd 16, HOS 2.0.0+).
    /// Application-only.
    #[inline]
    pub fn set_out_of_focus_suspending_enabled(
        &self,
        enabled: bool,
    ) -> Result<(), SetOutOfFocusSuspendingEnabledError> {
        self.self_controller()
            .set_out_of_focus_suspending_enabled(enabled)
    }
}

//
// Shares ApplicationExtras (same `IApplicationProxy` class) but is rejected by
// AM's runtime gating for SetFocusHandlingMode / SetOutOfFocusSuspendingEnabled.
// NotifyRunning is allowed.

impl Proxy<SystemApplication> {
    /// `IApplicationFunctions` (proxy cmd 20).
    #[inline]
    pub fn application_functions(&self) -> &crate::ApplicationFunctions {
        &self.extras.application_functions
    }

    /// `IApplicationFunctions::NotifyRunning`.
    #[inline]
    pub fn notify_running(&self) -> Result<bool, NotifyRunningError> {
        self.application_functions().notify_running()
    }
}

impl Proxy<LibraryApplet> {
    /// `IProcessWindingController` (proxy cmd 10).
    #[inline]
    pub fn process_winding_controller(&self) -> &crate::ProcessWindingController {
        &self.extras.process_winding_controller
    }

    /// `ILibraryAppletSelfAccessor` (proxy cmd 20). Absent on HOS 15.0.0+.
    #[inline]
    pub fn library_applet_self_accessor(&self) -> Option<&crate::LibraryAppletSelfAccessor> {
        self.extras.library_applet_self_accessor.as_ref()
    }

    /// `IHomeMenuFunctions` (proxy cmd 22, HOS 15.0.0+).
    #[inline]
    pub fn home_menu_functions(&self) -> Option<&crate::HomeMenuFunctions> {
        self.extras.home_menu_functions.as_ref()
    }

    /// `IAppletCommonFunctions` (proxy cmd 21, HOS 7.0.0+).
    #[inline]
    pub fn applet_common_functions(&self) -> Option<&crate::AppletCommonFunctions> {
        self.extras.applet_common_functions.as_ref()
    }

    /// `IGlobalStateController` (proxy cmd 23, HOS 15.0.0+).
    #[inline]
    pub fn global_state_controller(&self) -> Option<&crate::GlobalStateController> {
        self.extras.global_state_controller.as_ref()
    }
}

impl Proxy<SystemApplet> {
    /// `IGlobalStateController` (proxy cmd 21).
    #[inline]
    pub fn global_state_controller(&self) -> &crate::GlobalStateController {
        &self.extras.global_state_controller
    }

    /// `IApplicationCreator` (proxy cmd 22).
    #[inline]
    pub fn application_creator(&self) -> &crate::ApplicationCreator {
        &self.extras.application_creator
    }

    /// `IAppletCommonFunctions` (proxy cmd 23, HOS 7.0.0+).
    #[inline]
    pub fn applet_common_functions(&self) -> Option<&crate::AppletCommonFunctions> {
        self.extras.applet_common_functions.as_ref()
    }
}

impl Proxy<OverlayApplet> {
    /// `IAppletCommonFunctions` (proxy cmd 21, HOS 7.0.0+).
    #[inline]
    pub fn applet_common_functions(&self) -> Option<&crate::AppletCommonFunctions> {
        self.extras.applet_common_functions.as_ref()
    }

    /// `IGlobalStateController` (proxy cmd 23, HOS 15.0.0+).
    #[inline]
    pub fn global_state_controller(&self) -> Option<&crate::GlobalStateController> {
        self.extras.global_state_controller.as_ref()
    }
}

/// Opens an Application proxy on `appletOE` and drains every sub-interface.
///
/// Performs SM → service (appletOE, domain) → `OpenApplicationProxy` (cmd 0)
/// → core seven sub-interfaces → `IApplicationFunctions` (cmd 20). Does **not**
/// perform the InFocus wait, AcquireForegroundRights, SetFocusHandlingMode,
/// or NotifyRunning steps — those are runtime policy and live in the caller.
pub fn open_application(
    sm: &SmService,
    process_handle: ProcessHandle,
) -> Result<Proxy<Application>, OpenError> {
    open::<Application>(sm, process_handle)
}

/// Opens a LibraryApplet proxy on `appletAE` and drains every sub-interface.
///
/// Uses the pre-3.0.0 path (`OpenLibraryAppletProxyOld`, cmd 200). The
/// 3.0.0+ path with `AppletAttribute` lives behind
/// [`open_library_applet_with_attribute`] (TODO; not yet implemented).
pub fn open_library_applet(
    sm: &SmService,
    process_handle: ProcessHandle,
) -> Result<Proxy<LibraryApplet>, OpenError> {
    open::<LibraryApplet>(sm, process_handle)
}

/// Opens a SystemApplet proxy on `appletAE` and drains every sub-interface.
pub fn open_system_applet(
    sm: &SmService,
    process_handle: ProcessHandle,
) -> Result<Proxy<SystemApplet>, OpenError> {
    open::<SystemApplet>(sm, process_handle)
}

/// Opens an OverlayApplet proxy on `appletAE` and drains every sub-interface.
pub fn open_overlay_applet(
    sm: &SmService,
    process_handle: ProcessHandle,
) -> Result<Proxy<OverlayApplet>, OpenError> {
    open::<OverlayApplet>(sm, process_handle)
}

/// Opens a SystemApplication proxy on `appletAE` and drains every sub-interface.
pub fn open_system_application(
    sm: &SmService,
    process_handle: ProcessHandle,
) -> Result<Proxy<SystemApplication>, OpenError> {
    open::<SystemApplication>(sm, process_handle)
}

/// Generic open: connects to the right service for `R`, opens the proxy, and
/// builds a `Proxy<R>`.
fn open<R: Role>(sm: &SmService, process_handle: ProcessHandle) -> Result<Proxy<R>, OpenError> {
    let service = match crate::connect(sm, R::APPLET_TYPE).map_err(OpenError::Connect)? {
        Some(s) => s,
        // `connect` only returns `None` for AppletType::None, which the role
        // markers never select.
        None => return Err(OpenError::NoneAppletType),
    };
    let proxy = service
        .open_proxy(R::APPLET_TYPE, process_handle)
        .map_err(OpenError::OpenProxy)?;
    Proxy::<R>::from_session(service, proxy)
}

/// Aggregate error returned by the per-role `open_*` functions.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// Failed to connect to applet service (`appletOE`/`appletAE`).
    #[error("failed to connect to applet service")]
    Connect(#[source] ConnectError),
    /// `connect` returned `None` for an `AppletType` that should always
    /// produce a service. This should be unreachable since role markers never
    /// select `AppletType::None`.
    #[error("applet type unexpectedly returned None from connect")]
    NoneAppletType,
    /// Failed to open the role's proxy session.
    #[error("failed to open applet proxy")]
    OpenProxy(#[source] OpenProxyError),
    /// Failed to obtain `ICommonStateGetter`.
    #[error("failed to get ICommonStateGetter")]
    GetCommonStateGetter(#[source] GetCommonStateGetterError),
    /// Failed to obtain `ISelfController`.
    #[error("failed to get ISelfController")]
    GetSelfController(#[source] GetSelfControllerError),
    /// Failed to obtain `IWindowController`.
    #[error("failed to get IWindowController")]
    GetWindowController(#[source] GetWindowControllerError),
    /// Failed to obtain a non-version-gated sub-interface (audio / display /
    /// library applet creator / debug functions).
    #[error("failed to get sub-interface")]
    GetSubInterface(#[source] GetSubInterfaceError),
    /// Failed to drain role-specific extras.
    #[error("failed to drain role-specific extras")]
    DrainExtras(#[source] DrainExtrasError),
}

impl ToResultCode for OpenError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::Connect(err) => err.to_rc(),
            Self::OpenProxy(err) => err.to_rc(),
            Self::GetCommonStateGetter(err) => err.to_rc(),
            Self::GetSelfController(err) => err.to_rc(),
            Self::GetWindowController(err) => err.to_rc(),
            Self::GetSubInterface(err) => err.to_rc(),
            Self::DrainExtras(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::NoneAppletType => GENERIC_ERROR,
        }
    }
}

impl<R: Role> Proxy<R> {
    /// Returns the applet resource user ID, fetched via `IWindowController`.
    ///
    /// Returns `Ok(None)` when AM reports ARUID = 0 (no aruid assigned, common
    /// for non-foreground roles). Returns `Err` on IPC failure.
    pub fn get_applet_resource_user_id(
        &self,
    ) -> Result<Option<Aruid>, crate::GetAppletResourceUserIdError> {
        self.window_controller().get_applet_resource_user_id()
    }
}
