//! Temperature measurement (`ts`) service implementation.
//!
//! Provides access to temperature sensor readings on the Switch.
//!
//! ## Interfaces
//!
//! Two API generations exist:
//!
//! - **Legacy** (1.0.0–16.1.0): Location-based queries via
//!   [`TsService::get_temperature_range`], [`TsService::get_temperature`],
//!   and [`TsService::get_temperature_milli_c`].
//! - **Session-based** (8.0.0+): Open a [`TsSession`] for a specific device
//!   code and read the temperature as a float.
//!
//! Per IC-4, this crate is hosversion-unaware — callers choose which
//! methods to call based on the target firmware version.
//!
//! ## Divergence from libnx
//!
//! libnx's `ts.c` keeps a guarded global singleton (`g_tsSrv`) managed by
//! `NX_GENERATE_SERVICE_GUARD`. This crate follows the convention of the
//! other `nx-service-*` crates: connect once via [`connect_cmif`], reuse
//! the service wrapper across calls, and close explicitly.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetTemperatureError, GetTemperatureMilliCError, GetTemperatureRangeError, OpenSessionError,
        SessionGetTemperatureError,
    },
    proto::SERVICE_NAME,
    types::{TemperatureRange, TsDeviceCode, TsLocation},
};

/// Temperature measurement service wrapper.
///
/// Provides both legacy location-based and session-based temperature reading.
#[repr(transparent)]
pub struct TsService(Session);

impl TsService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// Legacy CMIF protocol methods (1.0.0–16.1.0).
impl TsService {
    /// Gets the temperature range for a sensor location.
    ///
    /// Returns the minimum and maximum temperature in Celsius.
    /// Available on firmware 1.0.0–16.1.0.
    #[inline]
    pub fn get_temperature_range(
        &self,
        location: TsLocation,
    ) -> Result<TemperatureRange, GetTemperatureRangeError> {
        let (min, max) = cmif::get_temperature_range(self.0.handle(), location as u8)?;
        Ok(TemperatureRange { min, max })
    }

    /// Gets the temperature for a sensor location, in Celsius.
    ///
    /// Available on firmware 1.0.0–16.1.0.
    #[inline]
    pub fn get_temperature(&self, location: TsLocation) -> Result<i32, GetTemperatureError> {
        cmif::get_temperature(self.0.handle(), location as u8)
    }

    /// Gets the temperature for a sensor location, in millicelsius.
    ///
    /// Available on firmware 1.0.0–13.2.1.
    #[inline]
    pub fn get_temperature_milli_c(
        &self,
        location: TsLocation,
    ) -> Result<i32, GetTemperatureMilliCError> {
        cmif::get_temperature_milli_c(self.0.handle(), location as u8)
    }
}

/// Session-based CMIF protocol methods (8.0.0+).
impl TsService {
    /// Opens a temperature session for a specific device code.
    ///
    /// The returned [`TsSession`] can be used to read the temperature
    /// as a float. Available on firmware 8.0.0+.
    #[inline]
    pub fn open_session(&self, device_code: TsDeviceCode) -> Result<TsSession, OpenSessionError> {
        let handle = cmif::open_session(self.0.handle(), device_code as u32)?;

        let service = Session::from_handle(handle, 0);

        Ok(TsSession(service))
    }
}

/// Temperature device session wrapper.
///
/// Represents an open session to a specific temperature sensor device.
/// Provides floating-point temperature readings.
pub struct TsSession(Session);

impl TsSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods for the temperature session.
impl TsSession {
    /// Gets the temperature in Celsius as a float.
    ///
    /// Available on firmware 10.0.0+.
    #[inline]
    pub fn get_temperature(&self) -> Result<f32, SessionGetTemperatureError> {
        cmif::session_get_temperature(self.0.handle())
    }
}

/// Connects to the temperature measurement service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<TsService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(TsService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get ts service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
