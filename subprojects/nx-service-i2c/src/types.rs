//! I2C wire-layout types.

use bitflags::bitflags;

/// I2C device identifiers.
///
/// Each variant corresponds to a specific hardware device on the Switch's
/// I2C bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum I2cDevice {
    DebugPad = 0,
    TouchPanel = 1,
    Tmp451 = 2,
    Nct72 = 3,
    Alc5639 = 4,
    Max77620Rtc = 5,
    Max77620Pmic = 6,
    Max77621Cpu = 7,
    Max77621Gpu = 8,
    Bq24193 = 9,
    Max17050 = 10,
    Bm92t30mwv = 11,
    Ina226Vdd15v0Hb = 12,
    Ina226VsysCpuDs = 13,
    Ina226VsysGpuDs = 14,
    Ina226VsysDdrDs = 15,
    Ina226VsysAp = 16,
    Ina226VsysBlDs = 17,
    Bh1730 = 18,
    Ina226VsysCore = 19,
    Ina226Soc1V8 = 20,
    Ina226Lpddr1V8 = 21,
    Ina226Reg1V32 = 22,
    Ina226Vdd3V3Sys = 23,
    HdmiDdc = 24,
    HdmiScdc = 25,
    HdmiHdcp = 26,
    Fan53528 = 27,
    Max77812_3 = 28,
    Max77812_2 = 29,
    Ina226VddDdr0V6 = 30,
}

bitflags! {
    /// Transaction option flags for I2C send/receive operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct I2cTransactionOption: u32 {
        /// Generate a START condition before the transfer.
        const START = 1 << 0;
        /// Generate a STOP condition after the transfer.
        const STOP = 1 << 1;
        /// Convenience: both START and STOP.
        const ALL = Self::START.bits() | Self::STOP.bits();
    }
}
