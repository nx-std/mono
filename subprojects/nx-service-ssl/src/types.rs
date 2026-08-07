//! Wire-layout types and enums for the SSL service.

use static_assertions::const_assert_eq;

/// Controls which SSL service to initialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SslServiceType {
    /// Initialize the `ssl` service.
    Default = 0,
    /// Initialize the `ssl:s` service (15.0.0+).
    System = 1,
}

/// CA certificate identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum CaCertificateId {
    /// All certificates (3.0.0+).
    All = -1,
    NintendoCaG3 = 1,
    NintendoClass2CaG3 = 2,
    /// 16.0.0+
    NintendoRootCaG4 = 3,
    AmazonRootCa1 = 1000,
    StarfieldServicesRootCertificateAuthorityG2 = 1001,
    AddTrustExternalCaRoot = 1002,
    ComodoCertificationAuthority = 1003,
    UtnDataCorpSgc = 1004,
    UtnUserFirstHardware = 1005,
    BaltimoreCyberTrustRoot = 1006,
    CybertrustGlobalRoot = 1007,
    VerizonGlobalRootCa = 1008,
    DigiCertAssuredIdRootCa = 1009,
    DigiCertAssuredIdRootG2 = 1010,
    DigiCertGlobalRootCa = 1011,
    DigiCertGlobalRootG2 = 1012,
    DigiCertHighAssuranceEvRootCa = 1013,
    EntrustnetCertificationAuthority2048 = 1014,
    EntrustRootCertificationAuthority = 1015,
    EntrustRootCertificationAuthorityG2 = 1016,
    GeoTrustGlobalCa2 = 1017,
    GeoTrustGlobalCa = 1018,
    GeoTrustPrimaryCertificationAuthorityG3 = 1019,
    GeoTrustPrimaryCertificationAuthority = 1020,
    GlobalSignRootCa = 1021,
    GlobalSignRootCaR2 = 1022,
    GlobalSignRootCaR3 = 1023,
    GoDaddyClass2CertificationAuthority = 1024,
    GoDaddyRootCertificateAuthorityG2 = 1025,
    StarfieldClass2CertificationAuthority = 1026,
    StarfieldRootCertificateAuthorityG2 = 1027,
    ThawtePrimaryRootCaG3 = 1028,
    ThawtePrimaryRootCa = 1029,
    VeriSignClass3PublicPrimaryCertificationAuthorityG3 = 1030,
    VeriSignClass3PublicPrimaryCertificationAuthorityG5 = 1031,
    VeriSignUniversalRootCertificationAuthority = 1032,
    /// 6.0.0+
    DstRootCaX3 = 1033,
    /// 10.0.3+
    UserTrustRsaCertificationAuthority = 1034,
    /// 10.1.0+
    IsrgRootX10 = 1035,
    /// 10.1.0+
    UserTrustEccCertificationAuthority = 1036,
    /// 10.1.0+
    ComodoRsaCertificationAuthority = 1037,
    /// 10.1.0+
    ComodoEccCertificationAuthority = 1038,
    /// 11.0.0+
    AmazonRootCa2 = 1039,
    /// 11.0.0+
    AmazonRootCa3 = 1040,
    /// 11.0.0+
    AmazonRootCa4 = 1041,
    /// 11.0.0+
    DigiCertAssuredIdRootG3 = 1042,
    /// 11.0.0+
    DigiCertGlobalRootG3 = 1043,
    /// 11.0.0+
    DigiCertTrustedRootG4 = 1044,
    /// 11.0.0+
    EntrustRootCertificationAuthorityEc1 = 1045,
    /// 11.0.0+
    EntrustRootCertificationAuthorityG4 = 1046,
    /// 11.0.0+
    GlobalSignEccRootCaR4 = 1047,
    /// 11.0.0+
    GlobalSignEccRootCaR5 = 1048,
    /// 11.0.0+
    GlobalSignEccRootCaR6 = 1049,
    /// 11.0.0+
    GtsRootR1 = 1050,
    /// 11.0.0+
    GtsRootR2 = 1051,
    /// 11.0.0+
    GtsRootR3 = 1052,
    /// 11.0.0+
    GtsRootR4 = 1053,
    /// 12.0.0+
    SecurityCommunicationRootCa = 1054,
    /// 15.0.0+
    GlobalSignRootE4 = 1055,
    /// 15.0.0+
    GlobalSignRootR4 = 1056,
    /// 15.0.0+
    TTeleSecGlobalRootClass2 = 1057,
    /// 16.0.0+
    DigiCertTlsEccP384RootG5 = 1058,
    /// 16.0.0+
    DigiCertTlsRsa4096RootG5 = 1059,
}

/// Trusted certificate status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TrustedCertStatus {
    Invalid = -1,
    Removed = 0,
    EnabledTrusted = 1,
    EnabledNotTrusted = 2,
    Revoked = 3,
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

/// Debug option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DebugOptionType {
    AllowDisableVerifyOption = 0,
}

/// Certificate format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CertificateFormat {
    Pem = 1,
    Der = 2,
}

/// Internal PKI type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InternalPki {
    /// Enables using the DeviceCert.
    DeviceClientCertDefault = 1,
}

/// Context option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ContextOption {
    /// Default value at context creation is 1.
    CrlImportDateCheckEnable = 1,
}

/// I/O mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum IoMode {
    /// Timeout = 5 minutes.
    Blocking = 1,
    /// Timeout = 0.
    NonBlocking = 2,
}

/// Session cache mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SessionCacheMode {
    None = 0,
    SessionId = 1,
    SessionTicket = 2,
}

/// Renegotiation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RenegotiationMode {
    None = 0,
    Secure = 1,
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

/// Private option type for connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PrivateOptionType {
    DtlsSession = 1,
    /// 17.0.0+
    SetCipher = 2,
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

/// Server certificate detail header (output from `DoHandshakeGetServerCert`
/// when `GetServerCertChain` option is set).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct ServerCertDetailHeader {
    /// Magic number (`CertChMN` = 0x4E4D684374726543).
    pub magic: u64,
    /// Total certificates in the chain.
    pub cert_total: u32,
    pub _pad: u32,
}

const_assert_eq!(core::mem::size_of::<ServerCertDetailHeader>(), 0x10);

/// Server certificate detail entry.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct ServerCertDetailEntry {
    /// Size of the certificate data.
    pub size: u32,
    /// Offset from the start of the buffer to the certificate data.
    pub offset: u32,
}

const_assert_eq!(core::mem::size_of::<ServerCertDetailEntry>(), 0x08);

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

/// Parameters for `GeneratePrivateKeyAndCert` (16.0.0+).
///
/// The struct is sent whole, so the trailing padding the ABI leaves after
/// `common_name_len` is a named field: without it those four bytes reach the
/// service uninitialised.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct KeyAndCertParams {
    /// Must be value 1.
    pub unk_x0: u32,
    /// Key size in bits.
    pub key_size: i32,
    /// Public exponent (only low 4 bytes used). Must be non-zero.
    pub public_exponent: u64,
    /// CN (Common Name) NUL-terminated string.
    pub common_name: [u8; 0x40],
    /// Length of common_name excluding NUL-terminator. Must be 0x1-0x3F.
    pub common_name_len: u32,
    pub _pad: u32,
}

const_assert_eq!(core::mem::size_of::<KeyAndCertParams>(), 0x58);

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

/// Output payload for `GeneratePrivateKeyAndCert`.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct GenerateKeyAndCertOut {
    pub cert_size: u32,
    pub key_size: u32,
}

const_assert_eq!(core::mem::size_of::<GenerateKeyAndCertOut>(), 0x08);

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
