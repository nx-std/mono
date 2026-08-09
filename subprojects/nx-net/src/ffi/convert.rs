//! C-ABI integer conversions for the validated resolver input enums.
//!
//! The soft core's [`AddrFamily`], [`SockType`], and [`Protocol`] enums carry
//! their C numeric values as `#[repr]` discriminants, so converting an enum
//! *to* a C integer is a plain `as` cast and needs no code here. The reverse
//! direction — parsing an untrusted C `c_int` *into* one of those enums — is
//! the FFI hard shell's job: it can fail, so it is modelled as [`TryFrom`]
//! and confined to this module alongside the rejection error types.

use core::ffi::c_int;

use crate::{
    ffi::abi,
    resolve::family::{
        AddrFamily,
        Protocol,
        SockType,
    },
};

impl TryFrom<c_int> for AddrFamily {
    type Error = UnknownAddrFamily;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        match value {
            abi::AF_UNSPEC => Ok(Self::Unspec),
            abi::AF_INET => Ok(Self::Inet),
            abi::AF_INET6 => Ok(Self::Inet6),
            other => Err(UnknownAddrFamily(other)),
        }
    }
}

/// A C `AF_*` value that does not name a supported [`AddrFamily`].
#[derive(Debug, thiserror::Error)]
#[error("unsupported address family value {0}")]
pub struct UnknownAddrFamily(pub c_int);

impl TryFrom<c_int> for SockType {
    type Error = UnknownSockType;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Any),
            abi::SOCK_STREAM => Ok(Self::Stream),
            abi::SOCK_DGRAM => Ok(Self::Dgram),
            abi::SOCK_RAW => Ok(Self::Raw),
            abi::SOCK_RDM => Ok(Self::Rdm),
            abi::SOCK_SEQPACKET => Ok(Self::SeqPacket),
            other => Err(UnknownSockType(other)),
        }
    }
}

/// A C `SOCK_*` value that does not name a supported [`SockType`].
#[derive(Debug, thiserror::Error)]
#[error("unsupported socket type value {0}")]
pub struct UnknownSockType(pub c_int);

impl TryFrom<c_int> for Protocol {
    type Error = UnknownProtocol;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        match value {
            abi::IPPROTO_IP => Ok(Self::Unspec),
            abi::IPPROTO_ICMP => Ok(Self::Icmp),
            abi::IPPROTO_TCP => Ok(Self::Tcp),
            abi::IPPROTO_UDP => Ok(Self::Udp),
            abi::IPPROTO_IPV6 => Ok(Self::Ipv6),
            abi::IPPROTO_ICMPV6 => Ok(Self::Icmpv6),
            abi::IPPROTO_RAW => Ok(Self::Raw),
            other => Err(UnknownProtocol(other)),
        }
    }
}

/// A C `IPPROTO_*` value that does not name a supported [`Protocol`].
#[derive(Debug, thiserror::Error)]
#[error("unsupported protocol value {0}")]
pub struct UnknownProtocol(pub c_int);
