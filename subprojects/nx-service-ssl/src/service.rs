//! What the `ISslService` interface itself is asked for.
//!
//! The commands that belong to the service rather than to any one context or connection.
//!
//! # What is here
//!
//! - [`SslServiceType`] selects which of the two services a process opens. `ssl:s` carries the
//!   system-only commands, and a process that is not permitted it cannot open it at all, so this
//!   is a decision made once at connect rather than per command.
//! - [`FlushSessionCacheOptionType`] says how far a cache flush reaches. The cache is the
//!   service's and is shared by every context in the process, which is why the flush is a service
//!   command and not a connection one, and why one caller's flush is felt by the others.
//! - [`DebugOptionType`] names a setting that relaxes what the service enforces. The one that
//!   exists permits turning off verification that would otherwise be mandatory.
//!
//! Both option enums carry a `TryFrom<u32>` and are reached through it, because each is a value a C
//! caller names and `extern "C"` cannot be trusted to deliver one that is in range.
//! [`SslServiceType`] has none: no command takes it, and it is chosen in Rust at connect.

use crate::option::UnknownOption;

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

/// A setting that relaxes what the service enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DebugOptionType {
    /// Permits a connection to clear verification flags the service would otherwise require.
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
