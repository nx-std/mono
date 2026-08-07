//! Wire-layout types for the power state controller service.

/// Power management state.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmState {
    Awake = 0,
    ReadyAwaken = 1,
    ReadySleep = 2,
    ReadySleepCritical = 3,
    ReadyAwakenCritical = 4,
    ReadyShutdown = 5,
}

impl PmState {
    /// Creates a `PmState` from a raw `u32` value.
    ///
    /// Returns `None` if the value does not correspond to a known state.
    pub fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Awake),
            1 => Some(Self::ReadyAwaken),
            2 => Some(Self::ReadySleep),
            3 => Some(Self::ReadySleepCritical),
            4 => Some(Self::ReadyAwakenCritical),
            5 => Some(Self::ReadyShutdown),
            _ => None,
        }
    }
}

/// Power management module identifier.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PmModuleId {
    Usb = 4,
    Ethernet = 5,
    Fgm = 6,
    PcvClock = 7,
    PcvVoltage = 8,
    Gpio = 9,
    Pinmux = 10,
    Uart = 11,
    I2c = 12,
    I2cPcv = 13,
    Spi = 14,
    Pwm = 15,
    Psm = 16,
    Tc = 17,
    Omm = 18,
    Pcie = 19,
    Lbl = 20,
    Display = 21,
    Hid = 24,
    WlanSockets = 25,
    Fs = 27,
    Audio = 28,
    TmaHostIo = 30,
    Bluetooth = 31,
    Bpc = 32,
    Fan = 33,
    Pcm = 34,
    Nfc = 35,
    Apm = 36,
    Btm = 37,
    Nifm = 38,
    GpioLow = 39,
    Npns = 40,
    Lm = 41,
    Bcat = 42,
    Time = 43,
    Pctl = 44,
    Erpt = 45,
    Eupld = 46,
    Friends = 47,
    Bgtc = 48,
    Account = 49,
    Sasbus = 50,
    Ntc = 51,
    Idle = 52,
    Tcap = 53,
    PsmLow = 54,
    Ndd = 55,
    Olsc = 56,
    Ns = 61,
    Nvservices = 101,
    Spsm = 127,
}

/// Output of `IPmModule::GetRequest` (cmd 1).
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct GetRequestOut {
    pub state: u32,
    pub flags: u32,
}
