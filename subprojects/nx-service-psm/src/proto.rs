//! PSM (`psm`) protocol constants.

use nx_sf::ServiceName;

/// Service name for the power supply monitor service.
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("psm");

// ---------------------------------------------------------------------------
// IPsmServer commands
// ---------------------------------------------------------------------------

/// GetBatteryChargePercentage
pub const GET_BATTERY_CHARGE_PERCENTAGE: u32 = 0;

/// GetChargerType
pub const GET_CHARGER_TYPE: u32 = 1;

/// EnableBatteryCharging
pub const ENABLE_BATTERY_CHARGING: u32 = 2;

/// DisableBatteryCharging
pub const DISABLE_BATTERY_CHARGING: u32 = 3;

/// IsBatteryChargingEnabled
pub const IS_BATTERY_CHARGING_ENABLED: u32 = 4;

/// AcquireControllerPowerSupply
pub const ACQUIRE_CONTROLLER_POWER_SUPPLY: u32 = 5;

/// ReleaseControllerPowerSupply
pub const RELEASE_CONTROLLER_POWER_SUPPLY: u32 = 6;

/// OpenSession (returns IPsmSession sub-object)
pub const OPEN_SESSION: u32 = 7;

/// EnableEnoughPowerChargeEmulation
pub const ENABLE_ENOUGH_POWER_CHARGE_EMULATION: u32 = 8;

/// DisableEnoughPowerChargeEmulation
pub const DISABLE_ENOUGH_POWER_CHARGE_EMULATION: u32 = 9;

/// EnableFastBatteryCharging
pub const ENABLE_FAST_BATTERY_CHARGING: u32 = 10;

/// DisableFastBatteryCharging
pub const DISABLE_FAST_BATTERY_CHARGING: u32 = 11;

/// GetBatteryVoltageState
pub const GET_BATTERY_VOLTAGE_STATE: u32 = 12;

/// GetRawBatteryChargePercentage
pub const GET_RAW_BATTERY_CHARGE_PERCENTAGE: u32 = 13;

/// IsEnoughPowerSupplied
pub const IS_ENOUGH_POWER_SUPPLIED: u32 = 14;

/// GetBatteryAgePercentage
pub const GET_BATTERY_AGE_PERCENTAGE: u32 = 15;

/// GetBatteryChargeInfoEvent
pub const GET_BATTERY_CHARGE_INFO_EVENT: u32 = 16;

/// GetBatteryChargeInfoFields
pub const GET_BATTERY_CHARGE_INFO_FIELDS: u32 = 17;

/// GetBatteryChargeCalibratedEvent (3.0.0+)
pub const GET_BATTERY_CHARGE_CALIBRATED_EVENT: u32 = 18;

// ---------------------------------------------------------------------------
// IPsmSession commands
// ---------------------------------------------------------------------------

/// BindStateChangeEvent
pub const SESSION_BIND_STATE_CHANGE_EVENT: u32 = 0;

/// UnbindStateChangeEvent
pub const SESSION_UNBIND_STATE_CHANGE_EVENT: u32 = 1;

/// SetChargerTypeChangeEventEnabled
pub const SESSION_SET_CHARGER_TYPE_CHANGE_EVENT_ENABLED: u32 = 2;

/// SetPowerSupplyChangeEventEnabled
pub const SESSION_SET_POWER_SUPPLY_CHANGE_EVENT_ENABLED: u32 = 3;

/// SetBatteryVoltageStateChangeEventEnabled
pub const SESSION_SET_BATTERY_VOLTAGE_STATE_CHANGE_EVENT_ENABLED: u32 = 4;
