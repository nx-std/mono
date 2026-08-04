//! Backlight (`lbl`) service implementation.
//!
//! Provides display backlight management, brightness control, dimming,
//! auto-brightness, ambient light sensor access, and VR mode via the
//! `lbl` IPC service.
//!
//! ## Hosversion variants
//!
//! Commands 23–28 (ambient light sensor availability, VR mode brightness,
//! VR mode enable/disable/query) are only available on HOS 3.0.0+.
//! This crate exposes them unconditionally and leaves version selection
//! to the caller.
//!
//! ## Divergence from libnx
//!
//! libnx's `lbl.c` keeps a guarded global singleton managed by
//! `NX_GENERATE_SERVICE_GUARD` and checks `hosversionBefore(3,0,0)`
//! at runtime for the VR-mode and sensor-availability commands. This
//! crate follows the convention of the other `nx-service-*` crates:
//! connect once via [`connect_cmif`], reuse the [`LblService`] across
//! calls, and close the session explicitly with `Drop`.
//! Hosversion gating is the caller's responsibility.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{
    BorrowedSessionHandle,
    Session,
};

mod cmif;
mod proto;
mod types;

pub use self::{
    cmif::{
        DispatchInF32Error,
        DispatchInU64Error,
        DispatchNoIoError,
        DispatchOutBoolError,
        DispatchOutF32Error,
        DispatchOutU32Error,
        GetAmbientLightSensorValueError,
    },
    proto::SERVICE_NAME,
    types::{
        AmbientLightSensorValue,
        BacklightSwitchStatus,
    },
};

/// Backlight service (`lbl`) session wrapper.
#[repr(transparent)]
pub struct LblService(Session);

impl LblService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> BorrowedSessionHandle<'_> {
        self.0.handle()
    }
}

/// Settings persistence.
impl LblService {
    /// Saves the current backlight settings.
    #[inline]
    pub fn save_current_setting(&self) -> Result<(), DispatchNoIoError> {
        cmif::save_current_setting(self.0.handle())
    }

    /// Loads the current backlight settings.
    #[inline]
    pub fn load_current_setting(&self) -> Result<(), DispatchNoIoError> {
        cmif::load_current_setting(self.0.handle())
    }
}

/// Brightness control.
impl LblService {
    /// Sets the current brightness setting (0.0–1.0).
    #[inline]
    pub fn set_current_brightness_setting(
        &self,
        brightness: f32,
    ) -> Result<(), DispatchInF32Error> {
        cmif::set_current_brightness_setting(self.0.handle(), brightness)
    }

    /// Gets the current brightness setting.
    #[inline]
    pub fn get_current_brightness_setting(&self) -> Result<f32, DispatchOutF32Error> {
        cmif::get_current_brightness_setting(self.0.handle())
    }

    /// Applies the current brightness setting to the backlight hardware.
    #[inline]
    pub fn apply_current_brightness_setting_to_backlight(&self) -> Result<(), DispatchNoIoError> {
        cmif::apply_current_brightness_setting_to_backlight(self.0.handle())
    }

    /// Gets the brightness setting currently applied to the backlight
    /// hardware.
    #[inline]
    pub fn get_brightness_setting_applied_to_backlight(&self) -> Result<f32, DispatchOutF32Error> {
        cmif::get_brightness_setting_applied_to_backlight(self.0.handle())
    }
}

/// Backlight power control.
impl LblService {
    /// Switches the backlight on with a fade duration (nanoseconds).
    #[inline]
    pub fn switch_backlight_on(&self, fade_time: u64) -> Result<(), DispatchInU64Error> {
        cmif::switch_backlight_on(self.0.handle(), fade_time)
    }

    /// Switches the backlight off with a fade duration (nanoseconds).
    #[inline]
    pub fn switch_backlight_off(&self, fade_time: u64) -> Result<(), DispatchInU64Error> {
        cmif::switch_backlight_off(self.0.handle(), fade_time)
    }

    /// Gets the backlight switch status.
    ///
    /// Returns `None` if the service returns an unrecognised status
    /// value.
    #[inline]
    pub fn get_backlight_switch_status(
        &self,
    ) -> Result<Option<BacklightSwitchStatus>, DispatchOutU32Error> {
        let raw = cmif::get_backlight_switch_status(self.0.handle())?;
        Ok(BacklightSwitchStatus::from_raw(raw))
    }
}

/// Dimming control.
impl LblService {
    /// Enables display dimming.
    #[inline]
    pub fn enable_dimming(&self) -> Result<(), DispatchNoIoError> {
        cmif::enable_dimming(self.0.handle())
    }

    /// Disables display dimming.
    #[inline]
    pub fn disable_dimming(&self) -> Result<(), DispatchNoIoError> {
        cmif::disable_dimming(self.0.handle())
    }

    /// Returns whether dimming is enabled.
    #[inline]
    pub fn is_dimming_enabled(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::is_dimming_enabled(self.0.handle())
    }
}

/// Auto-brightness control.
impl LblService {
    /// Enables automatic brightness control.
    #[inline]
    pub fn enable_auto_brightness_control(&self) -> Result<(), DispatchNoIoError> {
        cmif::enable_auto_brightness_control(self.0.handle())
    }

    /// Disables automatic brightness control.
    #[inline]
    pub fn disable_auto_brightness_control(&self) -> Result<(), DispatchNoIoError> {
        cmif::disable_auto_brightness_control(self.0.handle())
    }

    /// Returns whether automatic brightness control is enabled.
    #[inline]
    pub fn is_auto_brightness_control_enabled(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::is_auto_brightness_control_enabled(self.0.handle())
    }
}

/// Ambient light sensor.
impl LblService {
    /// Sets the ambient light sensor value.
    #[inline]
    pub fn set_ambient_light_sensor_value(&self, value: f32) -> Result<(), DispatchInF32Error> {
        cmif::set_ambient_light_sensor_value(self.0.handle(), value)
    }

    /// Gets the ambient light sensor value.
    #[inline]
    pub fn get_ambient_light_sensor_value(
        &self,
    ) -> Result<AmbientLightSensorValue, GetAmbientLightSensorValueError> {
        let out = cmif::get_ambient_light_sensor_value(self.0.handle())?;
        Ok(AmbientLightSensorValue {
            over_limit: out.over_limit,
            lux: out.lux,
        })
    }

    /// Returns whether the ambient light sensor is available (3.0.0+).
    #[inline]
    pub fn is_ambient_light_sensor_available(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::is_ambient_light_sensor_available(self.0.handle())
    }
}

/// VR mode (3.0.0+).
impl LblService {
    /// Sets the current brightness setting for VR mode (3.0.0+).
    #[inline]
    pub fn set_current_brightness_setting_for_vr_mode(
        &self,
        brightness: f32,
    ) -> Result<(), DispatchInF32Error> {
        cmif::set_current_brightness_setting_for_vr_mode(self.0.handle(), brightness)
    }

    /// Gets the current brightness setting for VR mode (3.0.0+).
    #[inline]
    pub fn get_current_brightness_setting_for_vr_mode(&self) -> Result<f32, DispatchOutF32Error> {
        cmif::get_current_brightness_setting_for_vr_mode(self.0.handle())
    }

    /// Enables VR mode (3.0.0+).
    #[inline]
    pub fn enable_vr_mode(&self) -> Result<(), DispatchNoIoError> {
        cmif::enable_vr_mode(self.0.handle())
    }

    /// Disables VR mode (3.0.0+).
    #[inline]
    pub fn disable_vr_mode(&self) -> Result<(), DispatchNoIoError> {
        cmif::disable_vr_mode(self.0.handle())
    }

    /// Returns whether VR mode is enabled (3.0.0+).
    #[inline]
    pub fn is_vr_mode_enabled(&self) -> Result<bool, DispatchOutBoolError> {
        cmif::is_vr_mode_enabled(self.0.handle())
    }
}

/// Connects to the `lbl` (Backlight) service using CMIF.
///
/// The caller must close the returned [`LblService`] when done.
pub fn connect_cmif(sm: &SmService) -> Result<LblService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::new(handle, 0);

    Ok(LblService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get lbl service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
