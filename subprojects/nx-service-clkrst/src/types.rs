//! Wire-layout types for the clkrst service.

use core::mem::size_of;

use static_assertions::const_assert_eq;

/// PCV module identifier used by the [8.0.0+] clock/reset interface.
///
/// Each variant maps to a hardware clock/bus/device in the Tegra SoC.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PcvModuleId {
    CpuBus = 0x40000001,
    Gpu = 0x40000002,
    I2s1 = 0x40000003,
    I2s2 = 0x40000004,
    I2s3 = 0x40000005,
    Pwm = 0x40000006,
    I2c1 = 0x02000001,
    I2c2 = 0x02000002,
    I2c3 = 0x02000003,
    I2c4 = 0x02000004,
    I2c5 = 0x02000005,
    I2c6 = 0x02000006,
    Spi1 = 0x07000000,
    Spi2 = 0x07000001,
    Spi3 = 0x07000002,
    Spi4 = 0x07000003,
    Disp1 = 0x40000007,
    Disp2 = 0x40000008,
    Isp = 0x40000009,
    Vi = 0x4000000A,
    Sdmmc1 = 0x40000017,
    Sdmmc2 = 0x40000018,
    Sdmmc3 = 0x40000019,
    Sdmmc4 = 0x4000001A,
    Owr = 0x40000024,
    Csite = 0x40000025,
    Tsec = 0x40000028,
    Mselect = 0x4000002E,
    Hda2codec2x = 0x40000033,
    Actmon = 0x40000035,
    ExtPeriph1 = 0x40000036,
    ExtPeriph2 = 0x40000037,
    ExtPeriph3 = 0x40000038,
    I2cSlow = 0x40000039,
    Sor1 = 0x4000003C,
    Sata = 0x40000041,
    Hda = 0x40000042,
    XusbCoreHost = 0x40000044,
    XusbFalcon = 0x40000045,
    XusbFs = 0x40000046,
    XusbCoreDev = 0x40000047,
    XusbSs = 0x4000004B,
    UartA = 0x03000001,
    UartB = 0x35000405,
    UartC = 0x3500040F,
    UartD = 0x37000001,
    Host1x = 0x4000004C,
    Entropy = 0x4000004D,
    Ape = 0x40000050,
    Hda2hdmicodec = 0x40000051,
    Pcie = 0x40000053,
    GenMax = 0x40000054,
    Emc = 0x40000055,
    Ahb = 0x40000056,
    Apb = 0x40000057,
    AxiCbx = 0x40000058,
    Mc = 0x40000059,
    McB = 0x4000005A,
    KFuse = 0x4000005B,
    Plla = 0x4000005D,
    Pllc = 0x4000005E,
    PllaS = 0x4000005F,
    PlleHw = 0x40000060,
    Pvd = 0x40000061,
    Plld = 0x40000062,
    Plld2 = 0x40000063,
    Plldp = 0x40000064,
    PllcUd = 0x40000065,
    PllpUd = 0x40000066,
    Usbpad = 0x40000067,
    MemMax = 0x40000068,
    UsbCar = 0x40000069,
    MsEnc = 0x4000000B,
    Nvenc = 0x4000000C,
    Nvjpg = 0x4000000E,
    Nvdec = 0x4000000D,
    VicI = 0x4000000F,
    Tsecb = 0x40000029,
}

impl PcvModuleId {
    /// Alias for [`XusbSs`](Self::XusbSs) (same hardware block).
    pub const XUSB_SS_HOST_DEV: Self = Self::XusbSs;

    /// Alias for [`UsbCar`](Self::UsbCar) (same sentinel value).
    pub const GPU_MAX: Self = Self::UsbCar;

    /// Returns the raw `u32` value of this module ID.
    #[inline]
    pub fn as_raw(self) -> u32 {
        self as u32
    }
}

/// Type of clock rate list returned by
/// [`GetPossibleClockRates`](crate::proto::GET_POSSIBLE_CLOCK_RATES).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockRatesListType {
    Invalid = 0,
    Discrete = 1,
    Range = 2,
}

impl ClockRatesListType {
    /// Converts a raw `i32` to a [`ClockRatesListType`], returning `None`
    /// for unrecognised values.
    pub fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Invalid),
            1 => Some(Self::Discrete),
            2 => Some(Self::Range),
            _ => None,
        }
    }
}

const_assert_eq!(size_of::<PcvModuleId>(), 4);
const_assert_eq!(size_of::<ClockRatesListType>(), 4);
