//! Board Power Control (`bpc`) service implementation.
//!
//! Provides system power management — shutdown, reboot, sleep-button
//! state, and power-button polling.
//!
//! ## Service names
//!
//! libnx selects `"bpc"` on HOS 2.0.0+ and `"bpc:c"` on earlier
//! versions. This crate exposes both as [`SERVICE_NAME`] and
//! [`SERVICE_NAME_LEGACY`], letting the caller pick. See
//! [`connect_cmif`] (2.0.0+) and [`connect_cmif_legacy`] (< 2.0.0).
//!
//! ## Divergence from libnx
//!
//! libnx's `bpc.c` keeps a guarded global singleton (`g_bpcSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD` and includes hosversion
//! checks. This crate follows the convention of the other
//! `nx-service-*` crates: connect once via [`connect_cmif`] /
//! [`connect_cmif_legacy`], reuse the [`BpcService`] across calls, and let
//! `Drop` close the session. Hosversion gating is the caller's
//! responsibility.

#![no_std]

extern crate nx_panic_handler;

use nx_service_sm::SmService;
use nx_sf::service::Session;
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{GetPowerButtonError, GetSleepButtonStateError, RebootSystemError, ShutdownSystemError},
    proto::{SERVICE_NAME, SERVICE_NAME_LEGACY},
    types::SleepButtonState,
};

/// Board Power Control (`bpc`) session wrapper.
#[repr(transparent)]
pub struct BpcService(Session);

impl BpcService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl BpcService {
    /// Initiates a full system shutdown.
    #[inline]
    pub fn shutdown_system(&self) -> Result<(), ShutdownSystemError> {
        cmif::shutdown_system(self.0.handle())
    }

    /// Initiates a full system reboot.
    #[inline]
    pub fn reboot_system(&self) -> Result<(), RebootSystemError> {
        cmif::reboot_system(self.0.handle())
    }

    /// Gets the current sleep button state.
    ///
    /// Only available on HOS [2.0.0–13.2.1].
    #[inline]
    pub fn get_sleep_button_state(&self) -> Result<SleepButtonState, GetSleepButtonStateError> {
        cmif::get_sleep_button_state(self.0.handle())
    }

    /// Gets whether the power button is currently pushed.
    ///
    /// Only available on HOS [6.0.0+].
    #[inline]
    pub fn get_power_button(&self) -> Result<bool, GetPowerButtonError> {
        cmif::get_power_button(self.0.handle())
    }
}

/// Connects to the `bpc` (Board Power Control) service using CMIF.
///
/// Uses the `"bpc"` service name, available on HOS 2.0.0+. For
/// earlier versions, use [`connect_cmif_legacy`].
pub fn connect_cmif(sm: &SmService) -> Result<BpcService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    Ok(BpcService(Session::from_handle(handle, 0)))
}

/// Connects to the `bpc:c` (Board Power Control) service using CMIF.
///
/// Uses the legacy `"bpc:c"` service name for HOS < 2.0.0. For 2.0.0+,
/// use [`connect_cmif`].
pub fn connect_cmif_legacy(sm: &SmService) -> Result<BpcService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME_LEGACY)
        .map_err(ConnectCmifError)?;

    Ok(BpcService(Session::from_handle(handle, 0)))
}

/// Error returned by [`connect_cmif`] and [`connect_cmif_legacy`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get bpc service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
