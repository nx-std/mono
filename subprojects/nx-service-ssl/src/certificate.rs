//! Certificates and the keys that go with them.
//!
//! What a context trusts, what it presents, and what the peer turned out to be: the built-in
//! authorities, the status a trusted certificate is in, the formats a certificate is carried in,
//! and the chain a completed handshake reports back.

use static_assertions::const_assert_eq;

use crate::UnknownOption;

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

/// Certificate format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CertificateFormat {
    Pem = 1,
    Der = 2,
}

impl TryFrom<u32> for CertificateFormat {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Pem),
            2 => Ok(Self::Der),
            _ => Err(UnknownOption { value }),
        }
    }
}

/// Internal PKI type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InternalPki {
    /// Enables using the DeviceCert.
    DeviceClientCertDefault = 1,
}

impl TryFrom<u32> for InternalPki {
    type Error = UnknownOption;

    /// # Errors
    ///
    /// [`UnknownOption`] when `value` names no variant.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::DeviceClientCertDefault),
            _ => Err(UnknownOption { value }),
        }
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

/// Output payload for `GeneratePrivateKeyAndCert`.
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct GenerateKeyAndCertOut {
    pub cert_size: u32,
    pub key_size: u32,
}

const_assert_eq!(core::mem::size_of::<GenerateKeyAndCertOut>(), 0x08);
