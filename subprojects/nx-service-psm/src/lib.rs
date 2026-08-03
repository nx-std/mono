//! Power supply monitor (`psm`) service implementation.
//!
//! Provides battery charge percentage, charger type, voltage state, raw
//! charge/age percentages, charge info fields, and state-change event
//! sessions via the `psm` IPC service.
//!
//! ## Hosversion variants
//!
//! - `get_battery_charge_info_fields`: pre-17.0.0 returns
//!   [`BatteryChargeInfoFieldsLegacy`] (0x40 bytes), 17.0.0+ returns
//!   [`BatteryChargeInfoFields`] (0x54 bytes). Paired method variants
//!   are exposed and the caller selects the correct one.
//! - `get_battery_charge_calibrated_event`: 3.0.0+ only.
//!
//! This crate exposes all commands unconditionally and leaves version
//! selection to the caller.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{BorrowedSessionHandle, Session};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        DispatchEventError, DispatchInBoolError, DispatchNoIoError, DispatchOutBoolError,
        DispatchOutF64Error, DispatchOutStructError, DispatchOutU32Error, OpenSessionError,
    },
    proto::SERVICE_NAME,
    types::{
        BatteryChargeInfoFields, BatteryChargeInfoFieldsLegacy, BatteryVoltageState, ChargerType,
        Vdd50State,
    },
};

/// Power supply monitor service (`psm`) session wrapper.
#[repr(transparent)]
pub struct PsmService(Session);

impl PsmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Battery charge queries.
impl PsmService {
    /// Gets the current battery charge percentage (0–100).
    #[inline]
    pub fn get_battery_charge_percentage(&self) -> Result<u32, DispatchOutU32Error> {
        cmif::get_battery_charge_percentage(self.0.handle())
    }

    /// Gets the charger type.
    ///
    /// Returns `None` if the service returns an unrecognised charger type value.
    #[inline]
    pub fn get_charger_type(&self) -> Result<Option<ChargerType>, DispatchOutU32Error> {
        let raw = cmif::get_charger_type(self.0.handle())?;
        Ok(ChargerType::from_raw(raw))
    }

    /// Gets the battery voltage state.
    ///
    /// Returns `None` if the service returns an unrecognised state value.
    #[inline]
    pub fn get_battery_voltage_state(
        &self,
    ) -> Result<Option<BatteryVoltageState>, DispatchOutU32Error> {
        let raw = cmif::get_battery_voltage_state(self.0.handle())?;
        Ok(BatteryVoltageState::from_raw(raw))
    }

    /// Gets the raw battery charge percentage as a floating-point value.
    #[inline]
    pub fn get_raw_battery_charge_percentage(&self) -> Result<f64, DispatchOutF64Error> {
        cmif::get_raw_battery_charge_percentage(self.0.handle())
    }

    /// Returns whether enough power is supplied.
    #[inline]
    pub fn is_enough_power_supplied(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::is_enough_power_supplied(self.0.handle())
    }

    /// Gets the battery age percentage as a floating-point value.
    #[inline]
    pub fn get_battery_age_percentage(&self) -> Result<f64, DispatchOutF64Error> {
        cmif::get_battery_age_percentage(self.0.handle())
    }
}

/// Battery charging control.
impl PsmService {
    /// Enables battery charging.
    #[inline]
    pub fn enable_battery_charging(&self) -> Result<(), DispatchNoIoError> {
        cmif::enable_battery_charging(self.0.handle())
    }

    /// Disables battery charging.
    #[inline]
    pub fn disable_battery_charging(&self) -> Result<(), DispatchNoIoError> {
        cmif::disable_battery_charging(self.0.handle())
    }

    /// Returns whether battery charging is currently enabled.
    #[inline]
    pub fn is_battery_charging_enabled(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::is_battery_charging_enabled(self.0.handle())
    }

    /// Enables fast battery charging.
    #[inline]
    pub fn enable_fast_battery_charging(&self) -> Result<(), DispatchNoIoError> {
        cmif::enable_fast_battery_charging(self.0.handle())
    }

    /// Disables fast battery charging.
    #[inline]
    pub fn disable_fast_battery_charging(&self) -> Result<(), DispatchNoIoError> {
        cmif::disable_fast_battery_charging(self.0.handle())
    }
}

/// Controller power supply.
impl PsmService {
    /// Acquires controller power supply.
    #[inline]
    pub fn acquire_controller_power_supply(&self) -> Result<(), DispatchNoIoError> {
        cmif::acquire_controller_power_supply(self.0.handle())
    }

    /// Releases controller power supply.
    #[inline]
    pub fn release_controller_power_supply(&self) -> Result<(), DispatchNoIoError> {
        cmif::release_controller_power_supply(self.0.handle())
    }
}

/// Power charge emulation.
impl PsmService {
    /// Enables enough-power charge emulation.
    #[inline]
    pub fn enable_enough_power_charge_emulation(&self) -> Result<(), DispatchNoIoError> {
        cmif::enable_enough_power_charge_emulation(self.0.handle())
    }

    /// Disables enough-power charge emulation.
    #[inline]
    pub fn disable_enough_power_charge_emulation(&self) -> Result<(), DispatchNoIoError> {
        cmif::disable_enough_power_charge_emulation(self.0.handle())
    }
}

/// Battery charge info and events.
impl PsmService {
    /// Acquires the battery charge info event.
    ///
    /// Returns the raw copy handle for the event.
    #[inline]
    pub fn get_battery_charge_info_event(&self) -> Result<u32, DispatchEventError> {
        cmif::get_battery_charge_info_event(self.0.handle())
    }

    /// Gets battery charge info fields (pre-17.0.0 wire layout).
    #[inline]
    pub fn get_battery_charge_info_fields_legacy(
        &self,
    ) -> Result<BatteryChargeInfoFieldsLegacy, DispatchOutStructError> {
        cmif::get_battery_charge_info_fields_legacy(self.0.handle())
    }

    /// Gets battery charge info fields (17.0.0+ wire layout).
    #[inline]
    pub fn get_battery_charge_info_fields(
        &self,
    ) -> Result<BatteryChargeInfoFields, DispatchOutStructError> {
        cmif::get_battery_charge_info_fields(self.0.handle())
    }

    /// Acquires the battery charge calibrated event (3.0.0+).
    ///
    /// Returns the raw copy handle for the event.
    #[inline]
    pub fn get_battery_charge_calibrated_event(&self) -> Result<u32, DispatchEventError> {
        cmif::get_battery_charge_calibrated_event(self.0.handle())
    }
}

/// Session management.
impl PsmService {
    /// Opens an [`IPsmSession`](PsmSession) sub-object for state-change event
    /// monitoring.
    #[inline]
    pub fn open_session(&self) -> Result<PsmSession, OpenSessionError> {
        let handle = cmif::open_session(self.0.handle())?;
        Ok(PsmSession(Session::new(handle, 0)))
    }
}

/// PSM session sub-object (`IPsmSession`) for state-change event monitoring.
///
/// Opened via [`PsmService::open_session`]. The session allows enabling or
/// disabling notifications for charger type, power supply, and battery
/// voltage state changes, and binding/unbinding the composite state-change
/// event.
#[repr(transparent)]
pub struct PsmSession(Session);

impl PsmSession {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }

    /// Binds the state-change event.
    ///
    /// Returns the raw copy handle for the event.
    #[inline]
    pub fn bind_state_change_event(&self) -> Result<u32, DispatchEventError> {
        cmif::session_bind_state_change_event(self.0.handle())
    }

    /// Unbinds the state-change event.
    #[inline]
    pub fn unbind_state_change_event(&self) -> Result<(), DispatchNoIoError> {
        cmif::session_unbind_state_change_event(self.0.handle())
    }

    /// Sets whether charger type change events are enabled.
    #[inline]
    pub fn set_charger_type_change_event_enabled(
        &self,
        enabled: bool,
    ) -> Result<(), DispatchInBoolError> {
        cmif::session_set_charger_type_change_event_enabled(self.0.handle(), enabled)
    }

    /// Sets whether power supply change events are enabled.
    #[inline]
    pub fn set_power_supply_change_event_enabled(
        &self,
        enabled: bool,
    ) -> Result<(), DispatchInBoolError> {
        cmif::session_set_power_supply_change_event_enabled(self.0.handle(), enabled)
    }

    /// Sets whether battery voltage state change events are enabled.
    #[inline]
    pub fn set_battery_voltage_state_change_event_enabled(
        &self,
        enabled: bool,
    ) -> Result<(), DispatchInBoolError> {
        cmif::session_set_battery_voltage_state_change_event_enabled(self.0.handle(), enabled)
    }
}

/// Connects to the `psm` (Power Supply Monitor) service using CMIF.
///
/// The returned [`PsmService`] closes its session when dropped.
pub fn connect_cmif(sm: &SmService) -> Result<PsmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(PsmService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get psm service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
