//! Certificates and the keys that go with them.
//!
//! What a context trusts, what it presents, and what the peer turned out to be.
//!
//! # What is here
//!
//! - [`CaCertificateId`] names one of the authorities the console ships, and [`TrustedCertStatus`]
//!   is the state the service currently holds one in.
//! - [`CertificateFormat`] says how a certificate is encoded when a caller imports one, and
//!   [`InternalPki`] which of the console's own the service may present on its behalf.
//! - [`ServerCertDetailHeader`] and [`ServerCertDetailEntry`] describe the chain a handshake
//!   reports back, laid out as the service writes it into the caller's buffer.
//! - [`KeyAndCertParams`] is what generating a private key and a certificate for it takes.
//!
//! The two enums a C caller can name a value of, [`CertificateFormat`] and [`InternalPki`], carry a
//! `TryFrom<u32>` and are reached through it. Taking either by value across `extern "C"` would be
//! unsound, because C can pass a word that names no variant.

use static_assertions::const_assert_eq;

use crate::option::UnknownOption;

/// One of the certificate authorities the console ships.
///
/// The identifier names the certificate; the documentation on each names **who operates it now**,
/// which the identifier does not and which is what decides whether a given server chains to it. A
/// root outlives the company that issued it, so several of these are branded for an owner that no
/// longer exists: the GeoTrust, Thawte and VeriSign roots are DigiCert's, and the Comodo and
/// USERTrust roots are Sectigo's.
///
/// A variant marked with a firmware version was added in that release and is absent below it. The
/// service answers a request for an id it does not know rather than faulting, so a caller reaching
/// below its baseline gets an error, not a wrong certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum CaCertificateId {
    /// Every certificate the service holds, rather than one of them (3.0.0+).
    All = -1,
    /// Nintendo's own, third generation. First-party services chain to it.
    NintendoCaG3 = 1,
    /// Nintendo's own class 2, third generation.
    NintendoClass2CaG3 = 2,
    /// Nintendo's own root, fourth generation (16.0.0+).
    NintendoRootCaG4 = 3,
    /// Amazon Trust Services, RSA 2048. What most AWS-fronted services chain to.
    AmazonRootCa1 = 1000,
    /// Starfield, a GoDaddy brand. Amazon's own chain cross-signs up to this one, so it is what an
    /// older client validates an AWS service through.
    StarfieldServicesRootCertificateAuthorityG2 = 1001,
    /// AddTrust, later Comodo and now Sectigo. **Expired in 2020**: a chain terminating here no
    /// longer verifies, whatever this option is set to.
    AddTrustExternalCaRoot = 1002,
    /// Comodo, now Sectigo.
    ComodoCertificationAuthority = 1003,
    /// The USERTrust network's legacy server-gated-cryptography root.
    UtnDataCorpSgc = 1004,
    /// Another USERTrust legacy root, now Sectigo's.
    UtnUserFirstHardware = 1005,
    /// Baltimore, now DigiCert. Long the root Microsoft Azure services chained to.
    BaltimoreCyberTrustRoot = 1006,
    /// Cybertrust, which took over the Baltimore business; now DigiCert.
    CybertrustGlobalRoot = 1007,
    /// Verizon, which held Cybertrust after Baltimore; now DigiCert.
    VerizonGlobalRootCa = 1008,
    /// DigiCert Assured ID, first generation, RSA.
    DigiCertAssuredIdRootCa = 1009,
    /// DigiCert Assured ID, second generation, RSA.
    DigiCertAssuredIdRootG2 = 1010,
    /// DigiCert Global, first generation, RSA. One of the most widely used roots on the web.
    DigiCertGlobalRootCa = 1011,
    /// DigiCert Global, second generation, RSA.
    DigiCertGlobalRootG2 = 1012,
    /// DigiCert's extended-validation root.
    DigiCertHighAssuranceEvRootCa = 1013,
    /// Entrust's 2048-bit root, the oldest of theirs here.
    EntrustnetCertificationAuthority2048 = 1014,
    /// Entrust, first generation.
    EntrustRootCertificationAuthority = 1015,
    /// Entrust, second generation.
    EntrustRootCertificationAuthorityG2 = 1016,
    /// GeoTrust, now DigiCert.
    GeoTrustGlobalCa2 = 1017,
    /// GeoTrust, now DigiCert.
    GeoTrustGlobalCa = 1018,
    /// GeoTrust primary, third generation; now DigiCert.
    GeoTrustPrimaryCertificationAuthorityG3 = 1019,
    /// GeoTrust primary, first generation; now DigiCert.
    GeoTrustPrimaryCertificationAuthority = 1020,
    /// GlobalSign, first generation.
    GlobalSignRootCa = 1021,
    /// GlobalSign R2.
    GlobalSignRootCaR2 = 1022,
    /// GlobalSign R3.
    GlobalSignRootCaR3 = 1023,
    /// GoDaddy, class 2.
    GoDaddyClass2CertificationAuthority = 1024,
    /// GoDaddy, second generation.
    GoDaddyRootCertificateAuthorityG2 = 1025,
    /// Starfield, a GoDaddy brand, class 2.
    StarfieldClass2CertificationAuthority = 1026,
    /// Starfield, a GoDaddy brand, second generation.
    StarfieldRootCertificateAuthorityG2 = 1027,
    /// Thawte primary, third generation; now DigiCert.
    ThawtePrimaryRootCaG3 = 1028,
    /// Thawte primary, first generation; now DigiCert.
    ThawtePrimaryRootCa = 1029,
    /// VeriSign class 3, third generation; now DigiCert.
    VeriSignClass3PublicPrimaryCertificationAuthorityG3 = 1030,
    /// VeriSign class 3, fifth generation; now DigiCert.
    VeriSignClass3PublicPrimaryCertificationAuthorityG5 = 1031,
    /// VeriSign's universal root; now DigiCert.
    VeriSignUniversalRootCertificationAuthority = 1032,
    /// IdenTrust's DST root, which cross-signed Let's Encrypt before ISRG's own root was widely
    /// trusted. **Expired in 2021**, so the cross-signed path it served no longer verifies
    /// (6.0.0+).
    DstRootCaX3 = 1033,
    /// Sectigo's USERTrust RSA root (10.0.3+).
    UserTrustRsaCertificationAuthority = 1034,
    /// ISRG's root, which Let's Encrypt issues under. What most free certificates chain to
    /// (10.1.0+).
    IsrgRootX10 = 1035,
    /// Sectigo's USERTrust ECC root (10.1.0+).
    UserTrustEccCertificationAuthority = 1036,
    /// Comodo's RSA root; now Sectigo (10.1.0+).
    ComodoRsaCertificationAuthority = 1037,
    /// Comodo's ECC root; now Sectigo (10.1.0+).
    ComodoEccCertificationAuthority = 1038,
    /// Amazon Trust Services, RSA 4096 (11.0.0+).
    AmazonRootCa2 = 1039,
    /// Amazon Trust Services, ECDSA P-256 (11.0.0+).
    AmazonRootCa3 = 1040,
    /// Amazon Trust Services, ECDSA P-384 (11.0.0+).
    AmazonRootCa4 = 1041,
    /// DigiCert Assured ID, third generation, ECC (11.0.0+).
    DigiCertAssuredIdRootG3 = 1042,
    /// DigiCert Global, third generation, ECC (11.0.0+).
    DigiCertGlobalRootG3 = 1043,
    /// DigiCert Trusted Root, fourth generation, RSA 4096 (11.0.0+).
    DigiCertTrustedRootG4 = 1044,
    /// Entrust's ECC root (11.0.0+).
    EntrustRootCertificationAuthorityEc1 = 1045,
    /// Entrust, fourth generation (11.0.0+).
    EntrustRootCertificationAuthorityG4 = 1046,
    /// GlobalSign's ECC root, R4 (11.0.0+).
    GlobalSignEccRootCaR4 = 1047,
    /// GlobalSign's ECC root, R5 (11.0.0+).
    GlobalSignEccRootCaR5 = 1048,
    /// GlobalSign R6 (11.0.0+).
    GlobalSignEccRootCaR6 = 1049,
    /// Google Trust Services R1, RSA. Google's own services chain here (11.0.0+).
    GtsRootR1 = 1050,
    /// Google Trust Services R2, RSA (11.0.0+).
    GtsRootR2 = 1051,
    /// Google Trust Services R3, ECDSA (11.0.0+).
    GtsRootR3 = 1052,
    /// Google Trust Services R4, ECDSA (11.0.0+).
    GtsRootR4 = 1053,
    /// SECOM Trust Systems, a Japanese authority (12.0.0+).
    SecurityCommunicationRootCa = 1054,
    /// GlobalSign E4 (15.0.0+).
    GlobalSignRootE4 = 1055,
    /// GlobalSign R4 (15.0.0+).
    GlobalSignRootR4 = 1056,
    /// T-Systems, Deutsche Telekom's authority, class 2 (15.0.0+).
    TTeleSecGlobalRootClass2 = 1057,
    /// DigiCert's TLS ECC P-384 root, fifth generation (16.0.0+).
    DigiCertTlsEccP384RootG5 = 1058,
    /// DigiCert's TLS RSA 4096 root, fifth generation (16.0.0+).
    DigiCertTlsRsa4096RootG5 = 1059,
}

/// The state the service holds a built-in certificate in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TrustedCertStatus {
    /// The certificate is not one the service knows.
    Invalid = -1,
    /// The certificate was removed and is no longer available.
    Removed = 0,
    /// Available, and trusted for verifying a peer.
    EnabledTrusted = 1,
    /// Available, but not trusted: it can be read out, and a chain resting on it does not verify.
    EnabledNotTrusted = 2,
    /// Withdrawn by its issuer, so nothing resting on it verifies.
    Revoked = 3,
}

/// How a certificate is encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CertificateFormat {
    /// Base64 text between BEGIN and END lines.
    Pem = 1,
    /// The raw DER encoding, unwrapped.
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
