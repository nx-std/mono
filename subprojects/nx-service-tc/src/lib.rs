//! Temperature control (`tc`) service implementation.
//!
//! Provides fan control management and skin temperature reading via
//! the `tc` service interface. Fan control can be enabled/disabled,
//! queried for its current state, and the skin temperature can be
//! read in milli-degrees Celsius.
//!
//! ## Divergence from libnx
//!
//! libnx's `tc.c` keeps a guarded global singleton (`g_tcSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD`. This crate follows the
//! convention of the other `nx-service-*` crates: connect once via
//! [`connect_cmif`], reuse the [`TcService`] across calls, and close
//! the session explicitly with `Drop`.
//!
//! libnx gates `tcGetSkinTemperatureMilliC` behind a hosversion
//! check (`hosversionBefore(5,0,0)`). Per IC-4 this crate is
//! hosversion-unaware: [`TcService::get_skin_temperature_milli_c`]
//! is always available and the caller selects based on system version.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;

pub use self::{
    cmif::{
        DisableFanControlError, EnableFanControlError, GetSkinTemperatureMilliCError,
        IsFanControlEnabledError,
    },
    proto::SERVICE_NAME,
};

/// Temperature control (`tc`) session wrapper.
#[repr(transparent)]
pub struct TcService(Session);

impl TcService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl TcService {
    /// Enables fan control.
    #[inline]
    pub fn enable_fan_control(&self) -> Result<(), EnableFanControlError> {
        cmif::enable_fan_control(self.0.handle())
    }

    /// Disables fan control.
    ///
    /// # Warning
    ///
    /// Disabling the fan can damage the system.
    #[inline]
    pub fn disable_fan_control(&self) -> Result<(), DisableFanControlError> {
        cmif::disable_fan_control(self.0.handle())
    }

    /// Queries whether fan control is enabled.
    #[inline]
    pub fn is_fan_control_enabled(&self) -> Result<bool, IsFanControlEnabledError> {
        cmif::is_fan_control_enabled(self.0.handle())
    }

    /// Gets the skin temperature in milli-degrees Celsius.
    ///
    /// Available on \[5.0.0+\]. The caller must check the system
    /// version before calling this method.
    #[inline]
    pub fn get_skin_temperature_milli_c(&self) -> Result<i32, GetSkinTemperatureMilliCError> {
        cmif::get_skin_temperature_milli_c(self.0.handle())
    }
}

/// Connects to the `tc` (Temperature Control) service using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<TcService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(TcService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get tc service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
