//! How an `ISslContext` is created and configured.
//!
//! A context is the settings a connection inherits: which TLS versions it will speak, and the
//! options that hold for every connection made from it.

use static_assertions::const_assert_eq;

use crate::UnknownOption;

bitflags::bitflags! {
    /// TLS version bitmask controlling min/max TLS versions.
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

/// Context option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContextOption {
    /// Default value at context creation is 1.
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
