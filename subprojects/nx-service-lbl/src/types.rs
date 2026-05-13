//! Backlight (`lbl`) wire-layout types.

/// Backlight switch status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BacklightSwitchStatus {
    Disabled = 0,
    Enabled = 1,
    Enabling = 2,
    Disabling = 3,
}

impl BacklightSwitchStatus {
    /// Converts a raw `u32` wire value to a [`BacklightSwitchStatus`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Disabled),
            1 => Some(Self::Enabled),
            2 => Some(Self::Enabling),
            3 => Some(Self::Disabling),
            _ => None,
        }
    }
}

/// Ambient light sensor reading returned by
/// [`LblService::get_ambient_light_sensor_value`](crate::LblService::get_ambient_light_sensor_value).
#[derive(Debug, Clone, Copy)]
pub struct AmbientLightSensorValue {
    /// Whether the sensor reading is over the limit.
    pub over_limit: bool,
    /// Ambient light level in lux.
    pub lux: f32,
}
