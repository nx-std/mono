//! What a socket is made of, at the moment it is created.
//!
//! Creating one takes three numbers: the family its addresses come from, the delivery guarantees
//! it carries, and the protocol underneath. All three are `int` on the wire, and any of the six
//! orderings compiles, so each is a type here instead.
//!
//! # The protocol follows from the type
//!
//! The service picks a protocol when it is given none, and under these families there is exactly
//! one to pick: a stream is TCP, a datagram is UDP. So [`Protocol::Default`] is what a caller
//! normally passes, and naming a protocol explicitly is for the caller who wants a raw socket or
//! knows the service supports something this crate does not name.

/// The family a socket's addresses belong to.
///
/// The discriminant is the family's own number. The service descends from a BSD stack, which
/// numbers IPv6 28 rather than the 10 a Linux table would give, and this is the value the command
/// carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Domain {
    /// IPv4 (`AF_INET`).
    Ipv4 = 2,
    /// IPv6 (`AF_INET6`).
    Ipv6 = 28,
}

impl Domain {
    /// The family's number, as the command sends it.
    pub(crate) const fn to_raw(self) -> i32 {
        self as i32
    }
}

impl TryFrom<i32> for Domain {
    type Error = UnknownDomain;

    /// Reads a family a caller named by number.
    ///
    /// # Errors
    ///
    /// [`UnknownDomain`] for a family this crate does not carry. The service supports the two
    /// named here over these commands, so a number outside them would be refused on arrival; it is
    /// refused here instead, where the caller can be told which value was wrong.
    fn try_from(raw: i32) -> Result<Self, Self::Error> {
        match raw {
            2 => Ok(Self::Ipv4),
            28 => Ok(Self::Ipv6),
            _ => Err(UnknownDomain { value: raw }),
        }
    }
}

/// Error returned when a number names no address family.
#[derive(Debug, thiserror::Error)]
#[error("no address family is numbered {value}")]
pub struct UnknownDomain {
    /// The value that was offered.
    pub value: i32,
}

/// What a socket carries, and with what guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SockType {
    /// A reliable, ordered byte stream (`SOCK_STREAM`).
    Stream = 1,
    /// Connectionless datagrams, neither ordered nor guaranteed (`SOCK_DGRAM`).
    Dgram = 2,
    /// Datagrams carrying their own protocol header (`SOCK_RAW`).
    Raw = 3,
}

impl SockType {
    /// The type's number, as the command sends it.
    pub(crate) const fn to_raw(self) -> i32 {
        self as i32
    }
}

impl TryFrom<i32> for SockType {
    type Error = UnknownSockType;

    /// Reads a socket type a caller named by number.
    ///
    /// # Errors
    ///
    /// [`UnknownSockType`] for a type this crate does not carry.
    fn try_from(raw: i32) -> Result<Self, Self::Error> {
        match raw {
            1 => Ok(Self::Stream),
            2 => Ok(Self::Dgram),
            3 => Ok(Self::Raw),
            _ => Err(UnknownSockType { value: raw }),
        }
    }
}

/// Error returned when a number names no socket type.
#[derive(Debug, thiserror::Error)]
#[error("no socket type is numbered {value}")]
pub struct UnknownSockType {
    /// The value that was offered.
    pub value: i32,
}

/// The protocol a socket runs over.
///
/// [`Self::Default`] is the ordinary choice: under the families here, the type already determines
/// the protocol, and the service picks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// Let the service choose the one protocol the type implies.
    Default,
    /// TCP.
    Tcp,
    /// UDP.
    Udp,
    /// ICMP, which is reached with a raw socket.
    Icmp,
}

impl Protocol {
    /// The protocol's number, as the command sends it.
    ///
    /// [`Self::Default`] sends `0`, which is how the wire spells "you choose".
    pub(crate) const fn to_raw(self) -> i32 {
        match self {
            Self::Default => 0,
            Self::Icmp => 1,
            Self::Tcp => 6,
            Self::Udp => 17,
        }
    }
}

impl TryFrom<i32> for Protocol {
    type Error = UnknownProtocol;

    /// Reads a protocol a caller named by number.
    ///
    /// Zero is the wire's "you choose" and reads as [`Protocol::Default`], which is what a caller
    /// passing it meant.
    ///
    /// # Errors
    ///
    /// [`UnknownProtocol`] for a protocol this crate does not carry.
    fn try_from(raw: i32) -> Result<Self, Self::Error> {
        match raw {
            0 => Ok(Self::Default),
            1 => Ok(Self::Icmp),
            6 => Ok(Self::Tcp),
            17 => Ok(Self::Udp),
            _ => Err(UnknownProtocol { value: raw }),
        }
    }
}

/// Error returned when a number names no protocol.
#[derive(Debug, thiserror::Error)]
#[error("no protocol is numbered {value}")]
pub struct UnknownProtocol {
    /// The value that was offered.
    pub value: i32,
}
