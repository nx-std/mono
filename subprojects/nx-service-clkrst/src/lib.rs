//! Clock/Reset (`clkrst`) service implementation.
//!
//! Provides clock rate management via the `IClkrstManager` /
//! `IClkrstSession` interface pair. The manager opens per-module
//! sessions that can read, write, and query possible clock rates.
//!
//! ## Availability
//!
//! Only available on HOS [8.0.0+].
//!
//! ## Divergence from libnx
//!
//! libnx's `clkrst.c` keeps a guarded global singleton (`g_clkrstSrv`)
//! managed by `NX_GENERATE_SERVICE_GUARD` and includes hosversion
//! checks. This crate follows the convention of the other
//! `nx-service-*` crates: connect once via [`connect_cmif`], reuse the
//! [`ClkrstService`] across calls, and close the session explicitly
//! with `Drop`. Hosversion gating is the caller's
//! responsibility.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        GetClockRateError, GetPossibleClockRatesError, OpenSessionError, PossibleClockRates,
        SetClockRateError,
    },
    proto::SERVICE_NAME,
    types::{ClockRatesListType, PcvModuleId},
};

/// Clock/Reset manager (`IClkrstManager`) session wrapper.
#[repr(transparent)]
pub struct ClkrstService(Session);

impl ClkrstService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl ClkrstService {
    /// Opens an `IClkrstSession` for the given PCV module.
    ///
    /// `unk` is set to `3` in official sysmodules.
    #[inline]
    pub fn open_session(
        &self,
        module_id: PcvModuleId,
        unk: u32,
    ) -> Result<ClkrstSession, OpenSessionError> {
        let handle = cmif::open_session(self.0.handle(), module_id, unk)?;
        let service = Session::new(handle, 0);
        Ok(ClkrstSession(service))
    }
}

/// Clock/Reset session (`IClkrstSession`) wrapper.
///
/// Obtained via [`ClkrstService::open_session`]. Controls the clock
/// rate for a specific PCV module.
#[repr(transparent)]
pub struct ClkrstSession(Session);

impl ClkrstSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// CMIF protocol methods.
impl ClkrstSession {
    /// Sets the clock rate in Hz.
    #[inline]
    pub fn set_clock_rate(&self, hz: u32) -> Result<(), SetClockRateError> {
        cmif::set_clock_rate(self.0.handle(), hz)
    }

    /// Gets the current clock rate in Hz.
    #[inline]
    pub fn get_clock_rate(&self) -> Result<u32, GetClockRateError> {
        cmif::get_clock_rate(self.0.handle())
    }

    /// Gets the list of possible clock rates for this session.
    ///
    /// Fills `rates` with up to `rates.len()` entries and returns the
    /// list type and actual count.
    #[inline]
    pub fn get_possible_clock_rates(
        &self,
        rates: &mut [u32],
    ) -> Result<PossibleClockRates, GetPossibleClockRatesError> {
        cmif::get_possible_clock_rates(self.0.handle(), rates)
    }
}

/// Connects to the `clkrst` (Clock/Reset) service using CMIF.
///
/// Only available on HOS [8.0.0+]. The caller must ensure the correct
/// HOS version before invoking this function.
pub fn connect_cmif(sm: &SmService) -> Result<ClkrstService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(ClkrstService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get clkrst service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
