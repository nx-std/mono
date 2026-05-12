//! Fan control (`fan`) service implementation.
//!
//! Provides fan speed management via the `IFanManager` / `IController`
//! interface pair. The manager opens per-device controllers that can
//! read and write the rotation speed level.
//!
//! ## Divergence from libnx
//!
//! libnx's `fan.c` keeps a guarded global singleton (`g_fanSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`FanService`] across calls, and close
//! the session explicitly with `Drop`.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;

pub use self::{
    cmif::{GetRotationSpeedLevelError, OpenControllerError, SetRotationSpeedLevelError},
    proto::SERVICE_NAME,
};

/// Fan manager (`IFanManager`) session wrapper.
#[repr(transparent)]
pub struct FanService(Session);

impl FanService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl FanService {
    /// Opens an `IController` session for the given device code.
    #[inline]
    pub fn open_controller(&self, device_code: u32) -> Result<FanController, OpenControllerError> {
        let handle = cmif::open_controller(self.0.handle(), device_code)?;
        let service = Session::from_handle(handle, 0);
        Ok(FanController(service))
    }
}

/// Fan controller (`IController`) session wrapper.
///
/// Obtained via [`FanService::open_controller`]. Controls the fan
/// rotation speed for a specific device.
#[repr(transparent)]
pub struct FanController(Session);

impl FanController {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl FanController {
    /// Sets the fan rotation speed level.
    ///
    /// # Warning
    ///
    /// Disabling the fan (setting level to 0.0) can damage the system.
    #[inline]
    pub fn set_rotation_speed_level(&self, level: f32) -> Result<(), SetRotationSpeedLevelError> {
        cmif::set_rotation_speed_level(self.0.handle(), level)
    }

    /// Gets the current fan rotation speed level.
    #[inline]
    pub fn get_rotation_speed_level(&self) -> Result<f32, GetRotationSpeedLevelError> {
        cmif::get_rotation_speed_level(self.0.handle())
    }
}

/// Connects to the `fan` (Fan Control) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<FanService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(FanService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get fan service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
