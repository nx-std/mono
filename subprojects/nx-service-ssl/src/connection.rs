//! How an `ISslConnection` is configured, and what it reports.
//!
//! A connection is one TLS session over one socket: how it blocks, what it verifies, what it
//! negotiated, and the options that hold for it alone rather than for the context it came from.
//!
//! # What is here
//!
//! - [`IoMode`] decides whether a transfer waits, and [`VerifyOption`] what the handshake checks
//!   about the peer before it will complete.
//! - [`SessionCacheMode`] and [`RenegotiationMode`] set what is reused across handshakes and
//!   whether the peer may start a new one mid-session.
//! - [`OptionType`] and [`PrivateOptionType`] are the two option namespaces the service keeps.
//!   Which one an option lives in is the service's choice, not a distinction of kind.
//! - [`PollEvent`] is what a wait on the connection asks about and answers with. It is the TLS
//!   stack's own readiness, not the socket's: a connection can hold buffered plaintext when the
//!   socket underneath has nothing left to read.
//! - [`AlpnProtoState`] and [`CipherInfo`] are what a completed handshake settled on.
//!
//! Every enum here carries a `TryFrom<u32>` and is reached through it, because each one is a value
//! a C caller names. Taking one by value across `extern "C"` would be unsound: C can pass a word
//! that names no variant, and a `#[repr(u32)]` enum holding one is undefined behaviour.
//!
//! The `pub(crate)` structs at the end are the request and response layouts of the commands above,
//! and are what those values are packed into on the wire.

use static_assertions::const_assert_eq;

use crate::option::UnknownOption;

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

/// What the connection reuses to shorten a later handshake with the same peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SessionCacheMode {
    /// Nothing is cached, so every handshake runs in full.
    None = 0,
    /// The server's session id is kept, and the server holds the state it indexes.
    SessionId = 1,
    /// The server's ticket is kept, which carries the state rather than indexing it.
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

/// Whether a peer may start a second handshake inside an established session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RenegotiationMode {
    /// Refused.
    None = 0,
    /// Allowed, but only bound to the session already running.
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

/// An option in the connection's private namespace.
///
/// Separate from [`OptionType`] because the service keeps two namespaces, not because these mean
/// anything different in kind. The two are carried by different commands with different layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PrivateOptionType {
    /// Runs the session over DTLS rather than TLS.
    DtlsSession = 1,
    /// Pins the cipher suite instead of negotiating one (17.0.0+).
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

/// How the ALPN exchange ended.
///
/// A protocol name is only worth reading in the states that produced one, which is why the state
/// is reported alongside it rather than left for a caller to infer from an empty name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlpnProtoState {
    /// The peer does not speak ALPN, so nothing was exchanged.
    NoSupport = 0,
    /// A protocol was agreed.
    Negotiated = 1,
    /// Both sides offered lists and no entry appeared in both.
    NoOverlap = 2,
    /// The server chose from the list this side offered.
    Selected = 3,
    /// A protocol carried in early data, before the handshake settled it.
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
    /// What the handshake checks about the peer before it will complete.
    ///
    /// Each flag is a check that is skipped when clear, so an empty set completes a handshake
    /// against any certificate at all.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VerifyOption: u32 {
        /// The chain terminates at an authority the context trusts.
        const PEER_CA           = 1 << 0;
        /// The certificate names the host the connection was told to reach.
        const HOST_NAME         = 1 << 1;
        /// The certificate is inside its validity period.
        const DATE_CHECK        = 1 << 2;
        /// Extended-validation checking, accepting a chain that is only partly EV.
        const EV_CERT_PARTIAL   = 1 << 3;
        /// The certificate carries the policy OID an EV issuer is required to assert (6.0.0+).
        const EV_POLICY_OID     = 1 << 4;
        /// The certificate matches a fingerprint pinned for it (6.0.0+).
        const EV_CERT_FINGERPRINT = 1 << 5;
    }
}

bitflags::bitflags! {
    /// What a wait on the connection asks about, and what it answers with.
    ///
    /// The TLS stack's readiness rather than the socket's. A connection holding decrypted bytes
    /// reads as readable even when the socket beneath it has nothing left, which is why a caller
    /// waits here rather than on the descriptor it handed over.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PollEvent: u32 {
        /// Plaintext is available to read.
        const READ   = 1 << 0;
        /// A write will not block.
        const WRITE  = 1 << 1;
        /// An exceptional condition is pending.
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
