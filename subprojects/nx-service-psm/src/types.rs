//! PSM (`psm`) wire-layout types.

use static_assertions::const_assert_eq;

/// Charger type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ChargerType {
    Unconnected = 0,
    EnoughPower = 1,
    LowPower = 2,
    NotSupported = 3,
}

impl ChargerType {
    /// Converts a raw `u32` wire value to a [`ChargerType`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Unconnected),
            1 => Some(Self::EnoughPower),
            2 => Some(Self::LowPower),
            3 => Some(Self::NotSupported),
            _ => None,
        }
    }
}

/// VDD 5.0V rail state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Vdd50State {
    Unknown = 0,
    Vdd50AOffVdd50BOff = 1,
    Vdd50AOnVdd50BOff = 2,
    Vdd50AOffVdd50BOn = 3,
}

impl Vdd50State {
    /// Converts a raw `u32` wire value to a [`Vdd50State`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::Unknown),
            1 => Some(Self::Vdd50AOffVdd50BOff),
            2 => Some(Self::Vdd50AOnVdd50BOff),
            3 => Some(Self::Vdd50AOffVdd50BOn),
            _ => None,
        }
    }
}

/// Battery voltage state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BatteryVoltageState {
    NeedsShutdown = 0,
    NeedsSleep = 1,
    NoPerformanceBoost = 2,
    Normal = 3,
}

impl BatteryVoltageState {
    /// Converts a raw `u32` wire value to a [`BatteryVoltageState`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            0 => Some(Self::NeedsShutdown),
            1 => Some(Self::NeedsSleep),
            2 => Some(Self::NoPerformanceBoost),
            3 => Some(Self::Normal),
            _ => None,
        }
    }
}

/// Battery charge info fields (pre-17.0.0 wire layout).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct BatteryChargeInfoFieldsLegacy {
    pub input_current_limit: u32,
    pub boost_mode_current_limit: u32,
    pub fast_charge_current_limit: u32,
    pub charge_voltage_limit: u32,
    pub charger_type: u32,
    pub hi_z_mode: u8,
    pub battery_charging: u8,
    pub _pad: [u8; 2],
    pub vdd50_state: u32,
    pub temperature_celsius: u32,
    pub battery_charge_percentage: u32,
    pub battery_charge_milli_voltage: u32,
    pub battery_age_percentage: u32,
    pub usb_power_role: u32,
    pub usb_charger_type: u32,
    pub charger_input_voltage_limit: u32,
    pub charger_input_current_limit: u32,
    pub fast_battery_charging: u8,
    pub controller_power_supply: u8,
    pub otg_request: u8,
    pub _reserved: u8,
}

const_assert_eq!(size_of::<BatteryChargeInfoFieldsLegacy>(), 0x40);

/// Battery charge info fields (17.0.0+ wire layout).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct BatteryChargeInfoFields {
    pub input_current_limit: u32,
    pub boost_mode_current_limit: u32,
    pub fast_charge_current_limit: u32,
    pub charge_voltage_limit: u32,
    pub charger_type: u32,
    pub hi_z_mode: u8,
    pub battery_charging: u8,
    pub _pad: [u8; 2],
    pub vdd50_state: u32,
    pub temperature_celsius: u32,
    pub battery_charge_percentage: u32,
    pub battery_charge_milli_voltage: u32,
    pub battery_age_percentage: u32,
    pub usb_power_role: u32,
    pub usb_charger_type: u32,
    pub charger_input_voltage_limit: u32,
    pub charger_input_current_limit: u32,
    pub fast_battery_charging: u8,
    pub controller_power_supply: u8,
    pub otg_request: u8,
    pub _reserved: u8,
    pub _unk_x40: [u8; 0x14],
}

const_assert_eq!(size_of::<BatteryChargeInfoFields>(), 0x54);
