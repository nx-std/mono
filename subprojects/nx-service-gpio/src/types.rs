//! GPIO wire-layout types and enums.

/// GPIO pad name identifiers.
///
/// Only a subset of pad names is listed here (those exposed in libnx).
/// The underlying wire type is `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GpioPadName {
    AudioCodec = 1,
    ButtonVolUp = 25,
    ButtonVolDown = 26,
    SdCd = 56,
}

/// GPIO pad direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GpioDirection {
    Input = 0,
    Output = 1,
}

/// GPIO pad value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GpioValue {
    Low = 0,
    High = 1,
}

/// GPIO interrupt mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GpioInterruptMode {
    LowLevel = 0,
    HighLevel = 1,
    RisingEdge = 2,
    FallingEdge = 3,
    AnyEdge = 4,
}

/// GPIO interrupt status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GpioInterruptStatus {
    Inactive = 0,
    Active = 1,
}
