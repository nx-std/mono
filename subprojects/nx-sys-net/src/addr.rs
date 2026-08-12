//! Socket addresses, in the two shapes this crate holds at once.
//!
//! [`nx_service_bsd::RawSockAddr`] carries the bytes the service exchanges and deliberately does
//! not interpret them: which family they belong to, and therefore how they decode, is the socket
//! layer's business. This module is that layer, and this is the same place `std` puts it: in the
//! function that turns a `sockaddr_storage` into a [`SocketAddr`].
//!
//! # The layouts are transcribed, not inferred
//!
//! Both structs below are pinned against the BSD headers this workspace's C code compiles against,
//! under `subprojects/libnx/src/nx/external/bsd/include/netinet/in.h`. Getting one wrong sends a
//! command to the wrong peer rather than failing a check, so each field is transcribed in the
//! header's order and the two differ exactly where the header does: `sockaddr_in` opens with a
//! length byte and `sockaddr_in6` does not.
//!
//! The fields are [`zerocopy`] byte-order types rather than plain integers, so a port that lives on
//! the wire big-endian cannot be read as the host's little-endian by omitting a conversion.

use core::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
};

use nx_service_bsd::RawSockAddr;
use zerocopy::{
    FromBytes as _,
    IntoBytes as _,
    byteorder::network_endian::{
        U16,
        U32,
    },
};

/// `AF_INET`: an IPv4 address follows.
const AF_INET: u8 = 2;

/// `AF_INET6`: an IPv6 address follows.
///
/// The service descends from a BSD stack, which numbers this family 28. Linux numbers it 10, and a
/// value copied from the wrong table names no family the service accepts.
const AF_INET6: u8 = 28;

/// Encodes an address into the bytes the service exchanges.
pub fn encode(addr: SocketAddr) -> RawSockAddr {
    match addr {
        SocketAddr::V4(addr) => adopt(
            SockAddrIn {
                // The structure is 16 bytes, so the narrowing is exact.
                len: size_of::<SockAddrIn>() as u8,
                family: AF_INET,
                port: U16::new(addr.port()),
                addr: addr.ip().octets(),
                zero: [0; 8],
            }
            .as_bytes(),
        ),
        SocketAddr::V6(addr) => adopt(
            SockAddrIn6 {
                family: AF_INET6,
                reserved: 0,
                port: U16::new(addr.port()),
                flow_info: U32::new(addr.flowinfo()),
                addr: addr.ip().octets(),
                scope_id: U32::new(addr.scope_id()),
            }
            .as_bytes(),
        ),
    }
}

/// Decodes an address the service reported.
///
/// # Errors
///
/// Returns [`DecodeAddrError::NoAddress`] when the service reported none, which is an ordinary
/// outcome for a socket whose family carries no peer address rather than a failure.
/// [`DecodeAddrError::UnknownFamily`] and [`DecodeAddrError::Truncated`] both mean the bytes do not
/// describe an address this layer can name.
pub fn decode(raw: &RawSockAddr) -> Result<SocketAddr, DecodeAddrError> {
    let bytes = raw.as_bytes();
    if bytes.is_empty() {
        return Err(DecodeAddrError::NoAddress);
    }

    // The two layouts keep their family byte in different places, because one opens with a length
    // byte and the other does not, so each is looked for where its own layout puts it.
    //
    // Only one can match. An IPv4 address carries its own length, 16, in the byte an IPv6 address
    // carries the family in, and 16 is not a family this reads; an IPv6 address carries its padding
    // byte, zero, where an IPv4 address carries the family. Neither value is the other's.
    match (bytes.first(), bytes.get(1)) {
        (Some(&AF_INET6), _) => decode_v6(bytes),
        (_, Some(&AF_INET)) => decode_v4(bytes),
        // Neither position named a family, so the leading byte is the most this can report: it is
        // the family for one layout and a length for the other, and nothing here says which.
        (Some(&leading), _) => Err(DecodeAddrError::UnknownFamily { family: leading }),
        (None, _) => Err(DecodeAddrError::Truncated { len: bytes.len() }),
    }
}

/// Decodes the IPv4 layout.
fn decode_v4(bytes: &[u8]) -> Result<SocketAddr, DecodeAddrError> {
    let Ok((addr, _)) = SockAddrIn::read_from_prefix(bytes) else {
        return Err(DecodeAddrError::Truncated { len: bytes.len() });
    };

    Ok(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::from(addr.addr),
        addr.port.get(),
    )))
}

/// Decodes the IPv6 layout.
fn decode_v6(bytes: &[u8]) -> Result<SocketAddr, DecodeAddrError> {
    let Ok((addr, _)) = SockAddrIn6::read_from_prefix(bytes) else {
        return Err(DecodeAddrError::Truncated { len: bytes.len() });
    };

    Ok(SocketAddr::V6(SocketAddrV6::new(
        Ipv6Addr::from(addr.addr),
        addr.port.get(),
        addr.flow_info.get(),
        addr.scope_id.get(),
    )))
}

/// Errors returned by [`decode`].
#[derive(Debug, thiserror::Error)]
pub enum DecodeAddrError {
    /// The service reported no address at all
    ///
    /// Occurs when a command succeeds without naming an address, which a socket whose family
    /// carries no peer address does routinely. Nothing is wrong; there is simply no address to
    /// name.
    #[error("The service reported no address")]
    NoAddress,

    /// The address names a family this layer cannot decode
    ///
    /// Occurs when the service reports an address outside IPv4 and IPv6. The bytes are intact and
    /// a caller that knows the family can still read them off the [`RawSockAddr`].
    #[error("The address names family {family}, which is neither IPv4 nor IPv6")]
    UnknownFamily {
        /// The family byte that was reported.
        family: u8,
    },

    /// The address is shorter than the family it names requires
    ///
    /// Occurs when the reported length does not cover the layout the family selects. Nothing can
    /// be recovered from it: the missing bytes are the address.
    #[error("The address is {len} bytes, too few for the family it names")]
    Truncated {
        /// How many bytes the service reported.
        len: usize,
    },
}

/// An IPv4 socket address, as `struct sockaddr_in`.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
    zerocopy::Unaligned,
)]
#[repr(C)]
struct SockAddrIn {
    /// Total length of the address, in bytes.
    len: u8,
    /// Always [`AF_INET`].
    family: u8,
    /// Port, big-endian on the wire.
    port: U16,
    /// The four address octets, in network order.
    addr: [u8; 4],
    /// Padding to the size of the generic address.
    zero: [u8; 8],
}

/// An IPv6 socket address, as `struct sockaddr_in6`.
///
/// Transcribed from this platform's header, which declares it without the leading length byte that
/// the family it descends from gives every address. The C socket driver never inspects an address,
/// so this shape is what every C caller on this platform builds and what the service is handed.
// TODO: confirm against the service that an IPv6 address is exchanged without a leading length
//  byte. Nothing on the console exercises one today, and the header this follows disagrees with the
//  stack it derives from, so the layout is settled only for IPv4.
#[derive(
    Debug,
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::IntoBytes,
    zerocopy::KnownLayout,
    zerocopy::Unaligned,
)]
#[repr(C)]
struct SockAddrIn6 {
    /// Always [`AF_INET6`].
    ///
    /// First, unlike [`SockAddrIn::family`]: the header this is transcribed from gives this
    /// structure no leading length byte, so the family opens it.
    family: u8,
    /// The byte the C layout spends aligning the port, named here because a structure with a hole
    /// in it cannot be written out as bytes.
    reserved: u8,
    /// Port, big-endian on the wire.
    port: U16,
    /// Flow information.
    flow_info: U32,
    /// The sixteen address octets, in network order.
    addr: [u8; 16],
    /// Scope identifier, for a link-local address.
    scope_id: U32,
}

/// Wraps encoded bytes as the owned form the commands take.
///
/// Shared by both arms of [`encode`], which differ only in the structure they lay out.
fn adopt(bytes: &[u8]) -> RawSockAddr {
    // SAFETY: `bytes` is the byte image of one of the two structures above, so it is 16 or 28 bytes
    // long; both are far below `RawSockAddr::CAPACITY` (128), which is the only length the
    // conversion rejects. It therefore cannot fail here.
    RawSockAddr::try_from(bytes).expect("encoded address fits the service's address storage")
}
