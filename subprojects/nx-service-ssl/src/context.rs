//! How an `ISslContext` is created and configured.
//!
//! A context is the settings a connection inherits: which TLS versions it will speak, and the
//! options that hold for every connection made from it.
//!
//! # What is here
//!
//! - [`SslVersion`] is the version range a context is created with, and the one setting that
//!   cannot be changed afterwards: it is an argument to the create command rather than an option.
//! - [`ContextOption`] is what can be changed afterwards, and holds for every connection the
//!   context goes on to make. A connection's own settings live in [`crate::connection`].
//!
//! [`ContextOption`] carries a `TryFrom<u32>` and is reached through it, because it is a value a C
//! caller names and `extern "C"` cannot be trusted to deliver one that is in range.
//!
//! The `pub(crate)` structs at the end are the request layouts of the two commands above.

use static_assertions::const_assert_eq;

use crate::option::UnknownOption;

bitflags::bitflags! {
    /// The TLS versions a context will speak.
    ///
    /// A set rather than a range: the service takes the versions it may use, and picks among them
    /// during the handshake. Setting one flag pins the connection to that version.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SslVersion: u32 {
        /// TLS version min = 1.0, max = 1.2.
        const AUTO    = 1 << 0;
        /// TLS 1.0.
        const TLS_V10 = 1 << 3;
        /// TLS 1.1.
        const TLS_V11 = 1 << 4;
        /// TLS 1.2.
        const TLS_V12 = 1 << 5;
        /// TLS 1.3 (11.0.0+).
        const TLS_V13 = 1 << 6;
        /// Same as Auto (11.0.0+).
        const AUTO24  = 1 << 24;
    }
}

/// A setting that holds for every connection the context makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContextOption {
    /// Whether an imported CRL is checked against its validity dates. On at context creation, so a
    /// caller only sets this to turn the check off.
    CrlImportDateCheckEnable = 1,
}

impl TryFrom<u32> for ContextOption {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::CrlImportDateCheckEnable),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// Input payload for `CreateContext`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct CreateContextIn {
    pub ssl_version: u32,
    pub _pad: u32,
    pub pid_placeholder: u64,
}

const_assert_eq!(core::mem::size_of::<CreateContextIn>(), 0x10);

/// Input payload for context `SetOption`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct CtxSetOptionIn {
    pub option: u32,
    pub value: i32,
}

const_assert_eq!(core::mem::size_of::<CtxSetOptionIn>(), 0x08);
