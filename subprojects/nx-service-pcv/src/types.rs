//! PCV wire-layout types and enumerations.

/// Hardware module index (pre-8.0.0 pcv interface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PcvModule {
    CpuBus = 0,
    Gpu = 1,
    I2s1 = 2,
    I2s2 = 3,
    I2s3 = 4,
    Pwm = 5,
    I2c1 = 6,
    I2c2 = 7,
    I2c3 = 8,
    I2c4 = 9,
    I2c5 = 10,
    I2c6 = 11,
    Spi1 = 12,
    Spi2 = 13,
    Spi3 = 14,
    Spi4 = 15,
    Disp1 = 16,
    Disp2 = 17,
    Isp = 18,
    Vi = 19,
    Sdmmc1 = 20,
    Sdmmc2 = 21,
    Sdmmc3 = 22,
    Sdmmc4 = 23,
    Owr = 24,
    Csite = 25,
    Tsec = 26,
    Mselect = 27,
    Hda2codec2x = 28,
    Actmon = 29,
    I2cSlow = 30,
    Sor1 = 31,
    Sata = 32,
    Hda = 33,
    XusbCoreHost = 34,
    XusbFalcon = 35,
    XusbFs = 36,
    XusbCoreDev = 37,
    XusbSsHostdev = 38,
    Uarta = 39,
    Uartb = 40,
    Uartc = 41,
    Uartd = 42,
    Host1x = 43,
    Entropy = 44,
    SocTherm = 45,
    Vic = 46,
    Nvenc = 47,
    Nvjpg = 48,
    Nvdec = 49,
    Qspi = 50,
    ViI2c = 51,
    Tsecb = 52,
    Ape = 53,
    Aclk = 54,
    Uartape = 55,
    Emc = 56,
    Plle0_0 = 57,
    Plle0_1 = 58,
    Dsi = 59,
    Maud = 60,
    Dpaux1 = 61,
    MipiCal = 62,
    UartFstMipiCal = 63,
    Osc = 64,
    Sclk = 65,
    SorSafe = 66,
    XusbSs = 67,
    XusbHost = 68,
    XusbDev = 69,
    Extperiph1 = 70,
    Ahub = 71,
    Hda2hdmicodec = 72,
    Pllp5 = 73,
    Usbd = 74,
    Usb2 = 75,
    Pcie = 76,
    Afi = 77,
    Pciexclk = 78,
    PexUsbUphy = 79,
    XusbPadctl = 80,
    Apbdma = 81,
    Usb2Trk = 82,
    Plle0_2 = 83,
    Plle0_3 = 84,
    Cec = 85,
    Extperiph2 = 86,
}

impl PcvModule {
    /// Maps a [`PcvModule`] index to its corresponding [`PcvModuleId`].
    pub fn to_module_id(self) -> PcvModuleId {
        #[rustfmt::skip]
        const MAP: [PcvModuleId; 87] = [
            PcvModuleId::CpuBus,        PcvModuleId::Gpu,           PcvModuleId::I2s1,              PcvModuleId::I2s2,
            PcvModuleId::I2s3,          PcvModuleId::Pwm,           PcvModuleId::I2c1,              PcvModuleId::I2c2,
            PcvModuleId::I2c3,          PcvModuleId::I2c4,          PcvModuleId::I2c5,              PcvModuleId::I2c6,
            PcvModuleId::Spi1,          PcvModuleId::Spi2,          PcvModuleId::Spi3,              PcvModuleId::Spi4,
            PcvModuleId::Disp1,         PcvModuleId::Disp2,         PcvModuleId::Isp,               PcvModuleId::Vi,
            PcvModuleId::Sdmmc1,        PcvModuleId::Sdmmc2,        PcvModuleId::Sdmmc3,            PcvModuleId::Sdmmc4,
            PcvModuleId::Owr,           PcvModuleId::Csite,         PcvModuleId::Tsec,              PcvModuleId::Mselect,
            PcvModuleId::Hda2codec2x,   PcvModuleId::Actmon,        PcvModuleId::I2cSlow,           PcvModuleId::Sor1,
            PcvModuleId::Sata,          PcvModuleId::Hda,           PcvModuleId::XusbCoreHost,      PcvModuleId::XusbFalcon,
            PcvModuleId::XusbFs,        PcvModuleId::XusbCoreDev,   PcvModuleId::XusbSsHostdev,     PcvModuleId::Uarta,
            PcvModuleId::Uartb,         PcvModuleId::Uartc,         PcvModuleId::Uartd,             PcvModuleId::Host1x,
            PcvModuleId::Entropy,       PcvModuleId::SocTherm,      PcvModuleId::Vic,               PcvModuleId::Nvenc,
            PcvModuleId::Nvjpg,         PcvModuleId::Nvdec,         PcvModuleId::Qspi,              PcvModuleId::ViI2c,
            PcvModuleId::Tsecb,         PcvModuleId::Ape,           PcvModuleId::Aclk,              PcvModuleId::Uartape,
            PcvModuleId::Emc,           PcvModuleId::Plle0_0,       PcvModuleId::Plle0_1,           PcvModuleId::Dsi,
            PcvModuleId::Maud,          PcvModuleId::Dpaux1,        PcvModuleId::MipiCal,           PcvModuleId::UartFstMipiCal,
            PcvModuleId::Osc,           PcvModuleId::Sclk,          PcvModuleId::SorSafe,           PcvModuleId::XusbSs,
            PcvModuleId::XusbHost,      PcvModuleId::XusbDev,       PcvModuleId::Extperiph1,        PcvModuleId::Ahub,
            PcvModuleId::Hda2hdmicodec, PcvModuleId::Pllp5,         PcvModuleId::Usbd,              PcvModuleId::Usb2,
            PcvModuleId::Pcie,          PcvModuleId::Afi,           PcvModuleId::Pciexclk,          PcvModuleId::PexUsbUphy,
            PcvModuleId::XusbPadctl,    PcvModuleId::Apbdma,        PcvModuleId::Usb2Trk,           PcvModuleId::Plle0_2,
            PcvModuleId::Plle0_3,       PcvModuleId::Cec,           PcvModuleId::Extperiph2,
        ];
        MAP[self as usize]
    }
}

/// Module ID returned by 8.0.0+ pcv services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PcvModuleId {
    CpuBus = 0x4000_0001,
    Gpu = 0x4000_0002,
    I2s1 = 0x4000_0003,
    I2s2 = 0x4000_0004,
    I2s3 = 0x4000_0005,
    Pwm = 0x4000_0006,
    I2c1 = 0x0200_0001,
    I2c2 = 0x0200_0002,
    I2c3 = 0x0200_0003,
    I2c4 = 0x0200_0004,
    I2c5 = 0x0200_0005,
    I2c6 = 0x0200_0006,
    Spi1 = 0x0700_0000,
    Spi2 = 0x0700_0001,
    Spi3 = 0x0700_0002,
    Spi4 = 0x0700_0003,
    Disp1 = 0x4000_0011,
    Disp2 = 0x4000_0012,
    Isp = 0x4000_0013,
    Vi = 0x4000_0014,
    Sdmmc1 = 0x4000_0015,
    Sdmmc2 = 0x4000_0016,
    Sdmmc3 = 0x4000_0017,
    Sdmmc4 = 0x4000_0018,
    Owr = 0x4000_0019,
    Csite = 0x4000_001A,
    Tsec = 0x4000_001B,
    Mselect = 0x4000_001C,
    Hda2codec2x = 0x4000_001D,
    Actmon = 0x4000_001E,
    I2cSlow = 0x4000_001F,
    Sor1 = 0x4000_0020,
    Sata = 0x4000_0021,
    Hda = 0x4000_0022,
    XusbCoreHost = 0x4000_0023,
    XusbFalcon = 0x4000_0024,
    XusbFs = 0x4000_0025,
    XusbCoreDev = 0x4000_0026,
    XusbSsHostdev = 0x4000_0027,
    Uarta = 0x0300_0001,
    Uartb = 0x3500_0405,
    Uartc = 0x3500_040F,
    Uartd = 0x3700_0001,
    Host1x = 0x4000_002C,
    Entropy = 0x4000_002D,
    SocTherm = 0x4000_002E,
    Vic = 0x4000_002F,
    Nvenc = 0x4000_0030,
    Nvjpg = 0x4000_0031,
    Nvdec = 0x4000_0032,
    Qspi = 0x4000_0033,
    ViI2c = 0x4000_0034,
    Tsecb = 0x4000_0035,
    Ape = 0x4000_0036,
    Aclk = 0x4000_0037,
    Uartape = 0x4000_0038,
    Emc = 0x4000_0039,
    Plle0_0 = 0x4000_003A,
    Plle0_1 = 0x4000_003B,
    Dsi = 0x4000_003C,
    Maud = 0x4000_003D,
    Dpaux1 = 0x4000_003E,
    MipiCal = 0x4000_003F,
    UartFstMipiCal = 0x4000_0040,
    Osc = 0x4000_0041,
    Sclk = 0x4000_0042,
    SorSafe = 0x4000_0043,
    XusbSs = 0x4000_0044,
    XusbHost = 0x4000_0045,
    XusbDev = 0x4000_0046,
    Extperiph1 = 0x4000_0047,
    Ahub = 0x4000_0048,
    Hda2hdmicodec = 0x4000_0049,
    Pllp5 = 0x4000_004A,
    Usbd = 0x4000_004B,
    Usb2 = 0x4000_004C,
    Pcie = 0x4000_004D,
    Afi = 0x4000_004E,
    Pciexclk = 0x4000_004F,
    PexUsbUphy = 0x4000_0050,
    XusbPadctl = 0x4000_0051,
    Apbdma = 0x4000_0052,
    Usb2Trk = 0x4000_0053,
    Plle0_2 = 0x4000_0054,
    Plle0_3 = 0x4000_0055,
    Cec = 0x4000_0056,
    Extperiph2 = 0x4000_0057,
}

/// Clock rate list type returned by `get_possible_clock_rates`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PcvClockRatesListType {
    Invalid = 0,
    Discrete = 1,
    Range = 2,
}

impl PcvClockRatesListType {
    /// Converts a raw `i32` wire value to a [`PcvClockRatesListType`].
    ///
    /// Returns `None` for unrecognised values.
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::Invalid),
            1 => Some(Self::Discrete),
            2 => Some(Self::Range),
            _ => None,
        }
    }
}

/// Wire-layout input for `SetClockRate` (cmd 2).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetClockRateIn {
    pub module: u32,
    pub hz: u32,
}

/// Wire-layout input for `GetPossibleClockRates` (cmd 5).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct GetPossibleClockRatesIn {
    pub module: u32,
    pub max_count: i32,
}

/// Wire-layout output for `GetPossibleClockRates` (cmd 5).
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct GetPossibleClockRatesOut {
    pub list_type: i32,
    pub count: i32,
}

/// Wire-layout input for `SetVoltageEnabled` (cmd 8).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetVoltageEnabledIn {
    pub state: u8,
    pub _pad: [u8; 3],
    pub power_domain: u32,
}

/// Result from [`PcvService::get_possible_clock_rates`](crate::PcvService::get_possible_clock_rates).
#[derive(Debug, Clone, Copy)]
pub struct PossibleClockRates {
    /// The type of rate list returned.
    pub list_type: Option<PcvClockRatesListType>,
    /// Number of rate entries written to the output buffer.
    pub count: i32,
}

static_assertions::const_assert_eq!(size_of::<SetClockRateIn>(), 0x08);
static_assertions::const_assert_eq!(size_of::<GetPossibleClockRatesIn>(), 0x08);
static_assertions::const_assert_eq!(size_of::<GetPossibleClockRatesOut>(), 0x08);
static_assertions::const_assert_eq!(size_of::<SetVoltageEnabledIn>(), 0x08);
