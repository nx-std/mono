//! Wire-layout types for the operation mode manager service.

/// Operation mode of the console.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperationMode {
    Handheld = 0,
    Console = 1,
}

impl OperationMode {
    /// Converts a raw `u8` value to an [`OperationMode`], if valid.
    #[inline]
    pub fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Handheld),
            1 => Some(Self::Console),
            _ => None,
        }
    }
}

/// Operation mode policy.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperationModePolicy {
    Auto = 0,
    Handheld = 1,
    Console = 2,
}

impl OperationModePolicy {
    /// Returns the raw `u8` value of this policy.
    #[inline]
    pub fn as_raw(self) -> u8 {
        self as u8
    }
}
