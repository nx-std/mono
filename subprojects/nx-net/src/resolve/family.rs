//! The address-family, socket-type, and protocol selector enums.
//!
//! Each variant's discriminant is its C `AF_*` / `SOCK_*` / `IPPROTO_*`
//! numeric value, so `selector as c_int` yields the wire/ABI representation
//! directly; the FFI layer parses the reverse direction with `TryFrom`. See
//! the crate-root documentation for how the validated input types fit the
//! three-layer design.

/// Address family selector for a resolver request.
///
/// Restricts the result set to a single IP version, or accepts both with
/// [`AddrFamily::Unspec`]. Each variant's discriminant is its C `AF_*`
/// numeric value, so `family as c_int` yields the wire/ABI representation
/// directly; the FFI layer parses the reverse direction with `TryFrom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AddrFamily {
    /// Accept any address family the resolver returns (`AF_UNSPEC`).
    #[default]
    Unspec = 0,
    /// IPv4 only (`AF_INET`).
    Inet = 2,
    /// IPv6 only (`AF_INET6`).
    Inet6 = 28,
}

/// Socket type selector for a resolver request.
///
/// Each variant's discriminant is its C `SOCK_*` numeric value;
/// [`SockType::Any`] is the unspecified hint (`0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SockType {
    /// Accept any socket type the resolver returns (hint value `0`).
    #[default]
    Any = 0,
    /// Reliable byte stream (`SOCK_STREAM`, TCP).
    Stream = 1,
    /// Connectionless datagrams (`SOCK_DGRAM`, UDP).
    Dgram = 2,
    /// Raw protocol interface (`SOCK_RAW`).
    Raw = 3,
    /// Reliably-delivered messages (`SOCK_RDM`).
    Rdm = 4,
    /// Sequenced packet stream (`SOCK_SEQPACKET`).
    SeqPacket = 5,
}

/// Protocol selector for a resolver request.
///
/// Each variant's discriminant is its C `IPPROTO_*` numeric value;
/// [`Protocol::Unspec`] is the unspecified hint (`IPPROTO_IP`, `0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Protocol {
    /// Let the resolver pick a protocol (`IPPROTO_IP`).
    #[default]
    Unspec = 0,
    /// Internet Control Message Protocol (`IPPROTO_ICMP`).
    Icmp = 1,
    /// Transmission Control Protocol (`IPPROTO_TCP`).
    Tcp = 6,
    /// User Datagram Protocol (`IPPROTO_UDP`).
    Udp = 17,
    /// IPv6 header (`IPPROTO_IPV6`).
    Ipv6 = 41,
    /// ICMP for IPv6 (`IPPROTO_ICMPV6`).
    Icmpv6 = 58,
    /// Raw IP packet (`IPPROTO_RAW`).
    Raw = 255,
}
