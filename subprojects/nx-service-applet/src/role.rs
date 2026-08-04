//! Applet role taxonomy.
//!
//! The Applet Manager admits clients in one of five roles. Each role is
//! reachable via a different proxy command (and through either `appletOE` or
//! `appletAE`), and each receives a different set of sub-interfaces. This
//! module models the taxonomy at the type level:
//!
//! * Five zero-sized marker types — [`Application`], [`LibraryApplet`],
//!   [`SystemApplet`], [`OverlayApplet`], [`SystemApplication`] — implement
//!   the sealed [`Role`] trait.
//! * Each role's `ExtraIds` associated type lists the object ids of the
//!   sub-interfaces that exist *only* for that role.
//! * `Role::drain_extras` performs the role-specific IPC choreography to open
//!   those sub-interfaces from a freshly-opened [`AppletProxyService`].
//!
//! Combined with the typed [`Proxy<R>`](crate::Proxy) wrapper, this lets
//! callers obtain a value whose method set reflects exactly the sub-interface
//! menu and convenience operations AM admits for the role — illegal
//! cross-role calls become compile errors instead of runtime
//! `LibnxError_NotInitialized` results.
//!
//! HOS-version-gated sub-interfaces are stored as `Option<T>`; role-mandatory
//! ones are non-optional, so the type system guarantees they exist whenever a
//! [`Proxy<R>`](crate::Proxy) is held.

use nx_sf::error::ToResultCode;
use nx_svc::error::ResultCode;

use crate::{
    AppletProxyService,
    AppletType,
    GetApplicationFunctionsError,
    GetSubInterfaceError,
};

mod sealed {
    pub trait Sealed {}
}

/// Sealed marker trait identifying an AM applet role.
///
/// Each implementor is a zero-sized type whose `Extras` associated type lists
/// the sub-interfaces unique to that role. The trait is sealed: only the
/// markers in this module may implement it.
pub trait Role: sealed::Sealed + 'static {
    /// The [`AppletType`] discriminant this role corresponds to.
    const APPLET_TYPE: AppletType;

    /// Object ids of the sub-interfaces stored alongside the core seven.
    type ExtraIds;

    /// Performs the IPC choreography to open the role's sub-interfaces from a
    /// freshly-opened proxy.
    ///
    /// Mandatory extras propagate IPC failures; HOS-version-gated extras
    /// swallow `Err` into `None` (firmware that doesn't expose the
    /// sub-interface is not an error).
    fn drain_extras(proxy: AppletProxyService<'_>) -> Result<Self::ExtraIds, DrainExtrasError>;
}

/// Application role (appletOE, proxy cmd 0, single session).
pub struct Application;
/// LibraryApplet role (appletAE, proxy cmd 200 pre-3.0.0 / cmd 201 since
/// 3.0.0).
pub struct LibraryApplet;
/// SystemApplet role (appletAE, proxy cmd 100).
pub struct SystemApplet;
/// OverlayApplet role (appletAE, proxy cmd 300).
pub struct OverlayApplet;
/// SystemApplication role (appletAE, proxy cmd 350). Receives an
/// `IApplicationProxy` — identical sub-interface menu to [`Application`],
/// distinct AM-side gating.
pub struct SystemApplication;

impl sealed::Sealed for Application {}
impl sealed::Sealed for LibraryApplet {}
impl sealed::Sealed for SystemApplet {}
impl sealed::Sealed for OverlayApplet {}
impl sealed::Sealed for SystemApplication {}

/// Object ids of the sub-interfaces unique to [`Application`] / [`SystemApplication`]
/// (`IApplicationProxy` class).
pub struct ApplicationExtraIds {
    pub application_functions: u32,
}

/// Object ids of the sub-interfaces unique to [`LibraryApplet`].
pub struct LibraryAppletExtraIds {
    pub process_winding_controller: u32,
    /// `ILibraryAppletSelfAccessor` (proxy cmd 20). Absent on HOS 15.0.0+
    /// where the equivalent surface moves to [`Self::home_menu_functions`].
    pub library_applet_self_accessor: Option<u32>,
    /// `IHomeMenuFunctions` (proxy cmd 22, HOS 15.0.0+) replaces
    /// `IFunctions`/`ILibraryAppletSelfAccessor`.
    pub home_menu_functions: Option<u32>,
    /// `IAppletCommonFunctions` (proxy cmd 21, HOS 7.0.0+).
    pub applet_common_functions: Option<u32>,
    /// `IGlobalStateController` (proxy cmd 23, HOS 15.0.0+).
    pub global_state_controller: Option<u32>,
}

/// Object ids of the sub-interfaces unique to [`SystemApplet`].
pub struct SystemAppletExtraIds {
    pub global_state_controller: u32,
    pub application_creator: u32,
    /// `IAppletCommonFunctions` (proxy cmd 23 for SystemApplet, HOS 7.0.0+).
    pub applet_common_functions: Option<u32>,
}

/// Object ids of the sub-interfaces unique to [`OverlayApplet`].
pub struct OverlayAppletExtraIds {
    /// `IAppletCommonFunctions` (proxy cmd 21, HOS 7.0.0+).
    pub applet_common_functions: Option<u32>,
    /// `IGlobalStateController` (proxy cmd 23, HOS 15.0.0+).
    pub global_state_controller: Option<u32>,
}

impl Role for Application {
    const APPLET_TYPE: AppletType = AppletType::Application;
    type ExtraIds = ApplicationExtraIds;

    fn drain_extras(proxy: AppletProxyService<'_>) -> Result<Self::ExtraIds, DrainExtrasError> {
        let application_functions = proxy
            .get_application_functions()
            .map_err(DrainExtrasError::GetApplicationFunctions)?
            .object_id();
        Ok(ApplicationExtraIds {
            application_functions,
        })
    }
}

impl Role for SystemApplication {
    const APPLET_TYPE: AppletType = AppletType::SystemApplication;
    // Same proxy class as Application.
    type ExtraIds = ApplicationExtraIds;

    fn drain_extras(proxy: AppletProxyService<'_>) -> Result<Self::ExtraIds, DrainExtrasError> {
        let application_functions = proxy
            .get_application_functions()
            .map_err(DrainExtrasError::GetApplicationFunctions)?
            .object_id();
        Ok(ApplicationExtraIds {
            application_functions,
        })
    }
}

impl Role for LibraryApplet {
    const APPLET_TYPE: AppletType = AppletType::LibraryApplet;
    type ExtraIds = LibraryAppletExtraIds;

    fn drain_extras(proxy: AppletProxyService<'_>) -> Result<Self::ExtraIds, DrainExtrasError> {
        let process_winding_controller = proxy
            .get_process_winding_controller()
            .map_err(DrainExtrasError::GetSubInterface)?
            .object_id();
        // Version-gated: accept whichever the firmware admits.
        let library_applet_self_accessor = proxy
            .get_library_applet_self_accessor()
            .ok()
            .map(|v| v.object_id());
        let home_menu_functions = proxy.get_home_menu_functions().ok().map(|v| v.object_id());
        let applet_common_functions = proxy
            .get_applet_common_functions(AppletType::LibraryApplet)
            .ok()
            .map(|v| v.object_id());
        let global_state_controller = proxy
            .get_global_state_controller(AppletType::LibraryApplet)
            .ok()
            .map(|v| v.object_id());
        Ok(LibraryAppletExtraIds {
            process_winding_controller,
            library_applet_self_accessor,
            home_menu_functions,
            applet_common_functions,
            global_state_controller,
        })
    }
}

impl Role for SystemApplet {
    const APPLET_TYPE: AppletType = AppletType::SystemApplet;
    type ExtraIds = SystemAppletExtraIds;

    fn drain_extras(proxy: AppletProxyService<'_>) -> Result<Self::ExtraIds, DrainExtrasError> {
        let global_state_controller = proxy
            .get_global_state_controller(AppletType::SystemApplet)
            .map_err(DrainExtrasError::GetSubInterface)?
            .object_id();
        let application_creator = proxy
            .get_application_creator()
            .map_err(DrainExtrasError::GetSubInterface)?
            .object_id();
        let applet_common_functions = proxy
            .get_applet_common_functions(AppletType::SystemApplet)
            .ok()
            .map(|v| v.object_id());
        Ok(SystemAppletExtraIds {
            global_state_controller,
            application_creator,
            applet_common_functions,
        })
    }
}

impl Role for OverlayApplet {
    const APPLET_TYPE: AppletType = AppletType::OverlayApplet;
    type ExtraIds = OverlayAppletExtraIds;

    fn drain_extras(proxy: AppletProxyService<'_>) -> Result<Self::ExtraIds, DrainExtrasError> {
        let applet_common_functions = proxy
            .get_applet_common_functions(AppletType::OverlayApplet)
            .ok()
            .map(|v| v.object_id());
        let global_state_controller = proxy
            .get_global_state_controller(AppletType::OverlayApplet)
            .ok()
            .map(|v| v.object_id());
        Ok(OverlayAppletExtraIds {
            applet_common_functions,
            global_state_controller,
        })
    }
}

/// Error returned by [`Role::drain_extras`].
#[derive(Debug, thiserror::Error)]
pub enum DrainExtrasError {
    /// Failed to obtain `IApplicationFunctions`.
    #[error("failed to get IApplicationFunctions")]
    GetApplicationFunctions(#[source] GetApplicationFunctionsError),
    /// Failed to obtain a role-specific sub-interface.
    #[error("failed to get sub-interface")]
    GetSubInterface(#[source] GetSubInterfaceError),
}

impl ToResultCode for DrainExtrasError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::GetApplicationFunctions(err) => err.to_rc(),
            Self::GetSubInterface(err) => err.to_rc(),
        }
    }
}
