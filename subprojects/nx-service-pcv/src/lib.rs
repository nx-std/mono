//! Power/Clock/Voltage (`pcv`) service implementation.
//!
//! Provides clock rate management, voltage control, and module-ID
//! mapping via the `pcv` IPC service.
//!
//! ## Hosversion variants
//!
//! All IPC commands in this crate are only available on HOS 1.0.0–7.0.1.
//! On 8.0.0+ the `pcv` service was replaced by `clkrst` and `pmc`.
//! This crate exposes commands unconditionally and leaves version
//! selection to the caller.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    DispatchError,
    Session,
};

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    proto::SERVICE_NAME,
    types::{
        PcvClockRatesListType,
        PcvModule,
        PcvModuleId,
        PossibleClockRates,
    },
};

/// PCV service session wrapper.
#[repr(transparent)]
pub struct PcvService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PcvService {}
unsafe impl Sync for PcvService {}

impl PcvService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Clock rate management (pre-8.0.0).
impl PcvService {
    /// Sets the clock rate for a module.
    #[inline]
    pub fn set_clock_rate(&self, module: PcvModule, hz: u32) -> Result<(), DispatchError> {
        cmif::set_clock_rate(&self.0, module as u32, hz)
    }

    /// Gets the clock rate for a module.
    #[inline]
    pub fn get_clock_rate(&self, module: PcvModule) -> Result<u32, DispatchError> {
        cmif::get_clock_rate(&self.0, module as u32)
    }

    /// Gets the possible clock rates for a module.
    ///
    /// Writes the rate values into `rates` and returns metadata about
    /// the rate list type and the number of entries actually written.
    #[inline]
    pub fn get_possible_clock_rates(
        &self,
        module: PcvModule,
        rates: &mut [u32],
    ) -> Result<PossibleClockRates, DispatchError> {
        let out = cmif::get_possible_clock_rates(&self.0, module as u32, rates)?;
        Ok(PossibleClockRates {
            list_type: PcvClockRatesListType::from_raw(out.list_type),
            count: out.count,
        })
    }
}

/// Voltage control (pre-8.0.0).
impl PcvService {
    /// Sets the voltage-enabled state for a power domain.
    #[inline]
    pub fn set_voltage_enabled(&self, power_domain: u32, state: bool) -> Result<(), DispatchError> {
        cmif::set_voltage_enabled(&self.0, power_domain, state)
    }

    /// Gets the voltage-enabled state for a power domain.
    #[inline]
    pub fn get_voltage_enabled(&self, power_domain: u32) -> Result<bool, DispatchError> {
        cmif::get_voltage_enabled(&self.0, power_domain)
    }
}

/// Connects to the `pcv` (Power/Clock/Voltage) service using CMIF.
///
/// The caller must close the returned [`PcvService`] when done.
pub fn connect_cmif(sm: &SmService) -> Result<PcvService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(PcvService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pcv service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
