//! What the resolver reports when it refuses a lookup.
//!
//! A resolver failure arrives as two words: the resolver's own verdict, and a
//! POSIX code beside it. This module names both, so neither travels through
//! the crate's API as a bare integer.
//!
//! # Why these are types and the code they replace was not
//!
//! `h_errno` and `errno` sit adjacent in every one of these replies, both
//! `u32`, and they mean entirely different things — one is a verdict from a
//! six-value set the resolver owns, the other a POSIX condition. Two
//! same-typed fields side by side is the swap hazard a newtype exists to
//! remove, and nothing but a doc comment was keeping them apart.
//!
//! The verdicts themselves are small closed sets: `h_errno` has six legal
//! values and the `getaddrinfo` return code fourteen, both fixed by `netdb.h`
//! rather than observed. That makes them enums, not open-ended codes.
//!
//! # Where the numbers come back
//!
//! Only at a C boundary, which is why [`HostError::to_wire`] and
//! [`AddrInfoError::to_wire`] exist and nothing above this crate needs them.
//! Unlike the BSD socket service, the resolver answers in the numbering the C
//! library already uses — a C caller's `h_errno` and `errno` receive these
//! words directly, with no translating table in between — so a code that
//! round-trips through these types comes back unchanged.
//!
//! The `h_errno` and `EAI_*` values are the ones `netdb.h` fixes, so a caller
//! comparing against the C constants sees what it expects.

/// The verdict a host lookup returns in `h_errno`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HostError {
    /// `NETDB_INTERNAL` — the resolver failed before it could answer, and the
    /// POSIX code beside it is what says why.
    #[error("resolver internal failure")]
    Internal,
    /// `HOST_NOT_FOUND` — an authoritative answer that the name does not
    /// exist.
    #[error("host not found")]
    NotFound,
    /// `TRY_AGAIN` — a non-authoritative failure; the same lookup may succeed
    /// later.
    #[error("host lookup failed, try again")]
    TryAgain,
    /// `NO_RECOVERY` — an unrecoverable failure, so retrying will not help.
    #[error("unrecoverable host lookup failure")]
    NoRecovery,
    /// `NO_DATA` — the name exists but carries no address of the family asked
    /// for.
    #[error("host has no address of the requested family")]
    NoData,
    /// A verdict this enum has no name for, carried unchanged.
    #[error("unrecognized host lookup verdict ({0})")]
    Unknown(i32),
}

impl HostError {
    /// Classifies the `h_errno` word, or `None` when it reports success.
    pub(crate) fn from_wire(raw: u32) -> Option<Self> {
        // The word is a C `int`; `NETDB_INTERNAL` is -1 and arrives with every
        // high bit set, so it has to be read signed before it is matched.
        match raw as i32 {
            0 => None,
            -1 => Some(Self::Internal),
            1 => Some(Self::NotFound),
            2 => Some(Self::TryAgain),
            3 => Some(Self::NoRecovery),
            4 => Some(Self::NoData),
            other => Some(Self::Unknown(other)),
        }
    }

    /// The `h_errno` value a C caller expects for this verdict.
    pub const fn to_wire(self) -> i32 {
        match self {
            Self::Internal => -1,
            Self::NotFound => 1,
            Self::TryAgain => 2,
            Self::NoRecovery => 3,
            Self::NoData => 4,
            Self::Unknown(raw) => raw,
        }
    }
}

/// The verdict an address or name lookup returns as its `EAI_*` code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AddrInfoError {
    /// `EAI_ADDRFAMILY` — the name has no address in the requested family.
    #[error("no address of the requested family for this name")]
    AddressFamily,
    /// `EAI_AGAIN` — a temporary failure; the same lookup may succeed later.
    #[error("temporary resolution failure, try again")]
    Again,
    /// `EAI_BADFLAGS` — the hint flags are not a legal combination.
    #[error("invalid lookup flags")]
    BadFlags,
    /// `EAI_FAIL` — an unrecoverable failure.
    #[error("unrecoverable resolution failure")]
    Fail,
    /// `EAI_FAMILY` — the requested address family is not supported.
    #[error("address family not supported")]
    Family,
    /// `EAI_MEMORY` — the resolver could not allocate.
    #[error("resolver out of memory")]
    Memory,
    /// `EAI_NODATA` — the name is known but carries no address.
    #[error("name has no associated address")]
    NoData,
    /// `EAI_NONAME` — neither a name nor a service was given, or neither is
    /// known.
    #[error("name or service not known")]
    NoName,
    /// `EAI_SERVICE` — the service is not available for the requested socket
    /// type.
    #[error("service not available for this socket type")]
    Service,
    /// `EAI_SOCKTYPE` — the requested socket type is not supported.
    #[error("socket type not supported")]
    SockType,
    /// `EAI_SYSTEM` — a system failure, described by the POSIX code beside
    /// this one.
    #[error("system failure during resolution")]
    System,
    /// `EAI_BADHINTS` — the hints structure is not valid.
    #[error("invalid hints")]
    BadHints,
    /// `EAI_PROTOCOL` — the requested protocol is not known.
    #[error("protocol not known")]
    Protocol,
    /// `EAI_OVERFLOW` — a result did not fit the buffer provided for it.
    #[error("result did not fit the caller's buffer")]
    Overflow,
    /// A code this enum has no name for, carried unchanged.
    #[error("unrecognized resolution failure ({0})")]
    Unknown(i32),
}

impl AddrInfoError {
    /// Classifies the return code, or `None` when it reports success.
    pub(crate) fn from_wire(raw: i32) -> Option<Self> {
        match raw {
            0 => None,
            1 => Some(Self::AddressFamily),
            2 => Some(Self::Again),
            3 => Some(Self::BadFlags),
            4 => Some(Self::Fail),
            5 => Some(Self::Family),
            6 => Some(Self::Memory),
            7 => Some(Self::NoData),
            8 => Some(Self::NoName),
            9 => Some(Self::Service),
            10 => Some(Self::SockType),
            11 => Some(Self::System),
            12 => Some(Self::BadHints),
            13 => Some(Self::Protocol),
            14 => Some(Self::Overflow),
            other => Some(Self::Unknown(other)),
        }
    }

    /// The `EAI_*` value a C caller expects for this verdict.
    pub const fn to_wire(self) -> i32 {
        match self {
            Self::AddressFamily => 1,
            Self::Again => 2,
            Self::BadFlags => 3,
            Self::Fail => 4,
            Self::Family => 5,
            Self::Memory => 6,
            Self::NoData => 7,
            Self::NoName => 8,
            Self::Service => 9,
            Self::SockType => 10,
            Self::System => 11,
            Self::BadHints => 12,
            Self::Protocol => 13,
            Self::Overflow => 14,
            Self::Unknown(raw) => raw,
        }
    }
}

/// The POSIX condition the resolver reports beside its verdict.
///
/// Kept as an opaque code rather than classified, because unlike the verdict
/// it is drawn from the whole POSIX set and this crate has no reason to
/// interpret it: the verdict is what a Rust caller acts on, and the number is
/// only meaningful to a C surface. A distinct type is still what keeps it from
/// being read as an `h_errno`, which is the field it has always sat next to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolverErrno(u32);

impl ResolverErrno {
    /// Adopts the code as the resolver reported it.
    pub(crate) const fn from_wire(raw: u32) -> Self {
        Self(raw)
    }

    /// The code, for a surface that has to write a C `errno`.
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

/// A host lookup the resolver refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the resolver refused the host lookup")]
pub struct HostFailure {
    /// The verdict it returned.
    #[source]
    pub kind: HostError,
    /// The POSIX condition it reported beside the verdict.
    pub errno: ResolverErrno,
}

/// An address or name lookup the resolver refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the resolver refused the lookup")]
pub struct AddrInfoFailure {
    /// The verdict it returned.
    #[source]
    pub kind: AddrInfoError,
    /// The POSIX condition it reported beside the verdict.
    pub errno: ResolverErrno,
}
