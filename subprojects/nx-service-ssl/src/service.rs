//! What the `ISslService` interface itself is asked for.
//!
//! The commands that belong to the service rather than to any one context or connection: which of
//! the two services to open, the session cache shared across the process, and the debug settings.

use crate::UnknownOption;

/// Controls which SSL service to initialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SslServiceType {
    /// Initialize the `ssl` service.
    Default = 0,
    /// Initialize the `ssl:s` service (15.0.0+).
    System = 1,
}

/// Flush session cache option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FlushSessionCacheOptionType {
    /// Flush for a single host (uses the input string).
    SingleHost = 0,
    /// Flush for all hosts (ignores the input string).
    AllHosts = 1,
}

impl TryFrom<u32> for FlushSessionCacheOptionType {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SingleHost),
            1 => Ok(Self::AllHosts),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// Debug option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DebugOptionType {
    AllowDisableVerifyOption = 0,
}

impl TryFrom<u32> for DebugOptionType {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AllowDisableVerifyOption),
            _ => Err(UnknownOption { value }),
        }
    }
}
