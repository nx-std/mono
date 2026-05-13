//! Temperature measurement wire-layout types.

/// Sensor location for legacy temperature commands.
///
/// Used with [`TsService::get_temperature_range`](crate::TsService::get_temperature_range),
/// [`TsService::get_temperature`](crate::TsService::get_temperature), and
/// [`TsService::get_temperature_milli_c`](crate::TsService::get_temperature_milli_c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TsLocation {
    /// TMP451 Internal: PCB temperature.
    Internal = 0,
    /// TMP451 External: SoC temperature.
    External = 1,
}

/// Device code for session-based temperature commands.
///
/// Used with [`TsService::open_session`](crate::TsService::open_session).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TsDeviceCode {
    /// Internal temperature sensor (PCB).
    LocationInternal = 0x4100_0001,
    /// External temperature sensor (SoC).
    LocationExternal = 0x4100_0002,
}

/// Output of [`TsService::get_temperature_range`](crate::TsService::get_temperature_range).
#[derive(Debug, Clone, Copy)]
pub struct TemperatureRange {
    /// Minimum temperature in Celsius.
    pub min: i32,
    /// Maximum temperature in Celsius.
    pub max: i32,
}
