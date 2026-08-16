//! How an `ISslConnection` is configured, and what it reports.
//!
//! A connection is one TLS session over one socket: how it blocks, what it verifies, what it
//! negotiated, and the options that hold for it alone rather than for the context it came from.

use static_assertions::const_assert_eq;

use crate::UnknownOption;

/// I/O mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IoMode {
    /// Timeout = 5 minutes.
    Blocking = 1,
    /// Timeout = 0.
    NonBlocking = 2,
}

impl TryFrom<u32> for IoMode {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Blocking),
            2 => Ok(Self::NonBlocking),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// Session cache mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SessionCacheMode {
    None = 0,
    SessionId = 1,
    SessionTicket = 2,
}

impl TryFrom<u32> for SessionCacheMode {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::SessionId),
            2 => Ok(Self::SessionTicket),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// Renegotiation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RenegotiationMode {
    None = 0,
    Secure = 1,
}

impl TryFrom<u32> for RenegotiationMode {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Secure),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// Option type for connection options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OptionType {
    /// Only available if `SetSocketDescriptor` was not used yet.
    DoNotCloseSocket = 0,
    /// 3.0.0+
    GetServerCertChain = 1,
    /// 5.0.0+
    SkipDefaultVerify = 2,
    /// 9.0.0+
    EnableAlpn = 3,
}

impl TryFrom<u32> for OptionType {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::DoNotCloseSocket),
            1 => Ok(Self::GetServerCertChain),
            2 => Ok(Self::SkipDefaultVerify),
            3 => Ok(Self::EnableAlpn),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// Private option type for connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PrivateOptionType {
    DtlsSession = 1,
    /// 17.0.0+
    SetCipher = 2,
}

impl TryFrom<u32> for PrivateOptionType {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::DtlsSession),
            2 => Ok(Self::SetCipher),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// ALPN protocol negotiation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlpnProtoState {
    NoSupport = 0,
    Negotiated = 1,
    NoOverlap = 2,
    Selected = 3,
    EarlyValue = 4,
}

impl AlpnProtoState {
    /// Reads the state out of the word `GetNextAlpnProto` reports.
    ///
    /// A value outside the set reads as [`NoSupport`](Self::NoSupport), which is what a caller
    /// does with a state it cannot act on anyway. Rejecting it would turn a service that grew a
    /// state into a hard failure on firmware this crate predates.
    pub(crate) fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Negotiated,
            2 => Self::NoOverlap,
            3 => Self::Selected,
            4 => Self::EarlyValue,
            _ => Self::NoSupport,
        }
    }
}

bitflags::bitflags! {
    /// Verify option bitmask for connection verification settings.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VerifyOption: u32 {
        const PEER_CA           = 1 << 0;
        const HOST_NAME         = 1 << 1;
        const DATE_CHECK        = 1 << 2;
        const EV_CERT_PARTIAL   = 1 << 3;
        /// 6.0.0+
        const EV_POLICY_OID     = 1 << 4;
        /// 6.0.0+
        const EV_CERT_FINGERPRINT = 1 << 5;
    }
}

bitflags::bitflags! {
    /// Poll event bitmask.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PollEvent: u32 {
        const READ   = 1 << 0;
        const WRITE  = 1 << 1;
        const EXCEPT = 1 << 2;
    }
}

/// Cipher information returned by `GetCipherInfo`.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct CipherInfo {
    /// Cipher suite name string.
    pub cipher: [u8; 0x40],
    /// Protocol version string.
    pub protocol_version: [u8; 0x08],
}

const_assert_eq!(core::mem::size_of::<CipherInfo>(), 0x48);

/// Input payload for connection `SetOption`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ConnSetOptionIn {
    pub flag: u8,
    pub _pad: [u8; 3],
    pub option: u32,
}

const_assert_eq!(core::mem::size_of::<ConnSetOptionIn>(), 0x08);

/// Input payload for connection `Poll`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct PollIn {
    pub in_pollevent: u32,
    pub timeout: u32,
}

const_assert_eq!(core::mem::size_of::<PollIn>(), 0x08);

/// Output payload for handshake with server cert.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct HandshakeServerCertOut {
    pub data_size: u32,
    pub total_certs: u32,
}

const_assert_eq!(core::mem::size_of::<HandshakeServerCertOut>(), 0x08);

/// Input payload for `SetPrivateOption` (pre-17.0.0 layout).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetPrivateOptionLegacyIn {
    pub value: u8,
    pub _pad: [u8; 3],
    pub option: u32,
}

const_assert_eq!(core::mem::size_of::<SetPrivateOptionLegacyIn>(), 0x08);

/// Input payload for `SetPrivateOption` (17.0.0+ layout).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct SetPrivateOptionIn {
    pub option: u32,
    pub value: u32,
}

const_assert_eq!(core::mem::size_of::<SetPrivateOptionIn>(), 0x08);

/// Output payload for `GetNextAlpnProto`.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct GetNextAlpnProtoOut {
    pub state: u32,
    pub proto_size: u32,
}

const_assert_eq!(core::mem::size_of::<GetNextAlpnProtoOut>(), 0x08);
