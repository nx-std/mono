//! Naming a socket option.
//!
//! An option is named by two numbers on the wire, and neither means anything without the other:
//! `0x0004` is `SO_REUSEADDR` under the socket level and something else entirely under TCP. The
//! pair is therefore not a pair in this API. [`SockOpt`] names the option, and both numbers are
//! derived from it, so a level and a name from different namespaces cannot be put together and
//! cannot be transposed.
//!
//! # Why there is no raw constructor
//!
//! Nothing here builds a [`SockOpt`] out of two integers. Such a constructor would admit exactly
//! the combination this type exists to rule out, and every guarantee below would hold only for
//! callers who chose not to use it.
//!
//! The C surface is what a raw pair is really for, and it has its own path:
//! [`BsdService::get_sock_opt_bytes`](crate::BsdService::get_sock_opt_bytes) and
//! [`set_sock_opt_bytes`](crate::BsdService::set_sock_opt_bytes) take the two numbers as they
//! arrive and send them unexamined. That is the honest shape for a conduit, and keeping it a
//! separate function is what stops it from weakening the typed API next to it.

/// A socket option, named rather than numbered.
///
/// The variants are the options this crate carries. An option outside the set is reachable only
/// through the byte-level commands, for the reason given in the module documentation.
///
/// Which level answers an option is a property of the option, not a choice a caller makes, so it
/// is not in the constructor: [`Self::level`] derives it.
///
/// # Where the set stops
///
/// The service descends from a FreeBSD stack and inherits its whole option space, most of which is
/// plumbing a program here has no route to: firewall tables, dummynet, RSVP, packet-filter hooks.
/// What is carried below is the intersection of that space with what the reference implementations
/// record the service as actually answering, which is a far smaller set and the one worth naming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SockOpt {
    /// Whether the socket may bind an address a previous connection still lingers on.
    ///
    /// What a listener on a fixed port needs to be restartable: without it the port stays unusable
    /// for as long as the kernel holds the old connection.
    ReuseAddr,
    /// Whether more than one socket may bind the same address and port.
    ReusePort,
    /// Whether the socket sends keep-alive probes on an idle connection.
    KeepAlive,
    /// Whether the socket may send to a broadcast address.
    Broadcast,
    /// Whether routing is bypassed, sending only to a directly attached network.
    DontRoute,
    /// Whether out-of-band data arrives in the normal stream rather than separately.
    OobInline,
    /// Whether debugging is recorded for the socket.
    Debug,
    /// Whether the socket is listening. Read only.
    AcceptConn,
    /// Whether sends are looped back to the sending host rather than put on the wire.
    UseLoopback,
    /// Whether a received datagram carries the time it arrived.
    Timestamp,
    /// Whether writing to a socket whose peer has gone reports an error rather than raising a
    /// signal. There is no signal to raise here, so this is carried for a caller porting code that
    /// sets it.
    NoSigpipe,
    /// How long a close waits for queued data to be sent.
    Linger,
    /// The transmit buffer size.
    SendBuffer,
    /// The receive buffer size.
    RecvBuffer,
    /// How much room must be free before the socket reports itself writable.
    SendLowWater,
    /// How much data must have arrived before the socket reports itself readable.
    RecvLowWater,
    /// How long a send waits before giving up.
    SendTimeout,
    /// How long a receive waits before giving up.
    RecvTimeout,
    /// The pending error, which reading clears. How a non-blocking connect reports its outcome.
    Error,
    /// The socket's type. Read only.
    Type,

    /// The type-of-service byte on outgoing packets.
    IpTos,
    /// The time-to-live on outgoing packets.
    IpTtl,
    /// Which interface outgoing multicast leaves by.
    IpMulticastIf,
    /// The time-to-live on outgoing multicast, which bounds how far it travels.
    IpMulticastTtl,
    /// Whether outgoing multicast is also delivered back to this host.
    IpMulticastLoop,
    /// Joins a multicast group. What a caller listening for LAN discovery sets.
    IpAddMembership,
    /// Leaves a multicast group.
    IpDropMembership,
    /// Whether a received packet reports the time-to-live it arrived with.
    IpRecvTtl,
    /// The smallest time-to-live an incoming packet may carry to be accepted.
    IpMinTtl,
    /// Whether outgoing packets are marked not to be fragmented.
    IpDontFrag,

    /// Whether small writes are sent at once rather than coalesced.
    TcpNoDelay,
    /// The largest segment TCP will send.
    TcpMaxSegment,
    /// Whether output is held back until the socket has a full segment to send.
    TcpNoPush,
    /// Whether TCP options are suppressed on the connection.
    TcpNoOpt,
    /// How long a connection attempt may take before it is abandoned.
    TcpKeepInit,
    /// How long a connection stays idle before the first keep-alive probe.
    TcpKeepIdle,
    /// How long between keep-alive probes once they have started.
    TcpKeepInterval,
    /// How many unanswered keep-alive probes close the connection.
    TcpKeepCount,
}

impl SockOpt {
    /// Options the socket layer answers itself, whatever protocol is underneath.
    ///
    /// Deliberately outside the range a protocol number occupies, so a level and a protocol can
    /// never be confused for one another.
    const SOL_SOCKET: i32 = 0xffff;
    /// Options IPv4 answers, named by its protocol number.
    const IPPROTO_IP: i32 = 0;
    /// Options TCP answers, named by its protocol number.
    const IPPROTO_TCP: i32 = 6;

    /// Which level answers this option, as the command sends it.
    pub(crate) const fn level(self) -> i32 {
        match self {
            Self::IpTos
            | Self::IpTtl
            | Self::IpMulticastIf
            | Self::IpMulticastTtl
            | Self::IpMulticastLoop
            | Self::IpAddMembership
            | Self::IpDropMembership
            | Self::IpRecvTtl
            | Self::IpMinTtl
            | Self::IpDontFrag => Self::IPPROTO_IP,

            Self::TcpNoDelay
            | Self::TcpMaxSegment
            | Self::TcpNoPush
            | Self::TcpNoOpt
            | Self::TcpKeepInit
            | Self::TcpKeepIdle
            | Self::TcpKeepInterval
            | Self::TcpKeepCount => Self::IPPROTO_TCP,

            _ => Self::SOL_SOCKET,
        }
    }

    /// What this option is numbered within its level, as the command sends it.
    ///
    /// Not a discriminant: the numbers repeat across levels, so `TcpNoDelay` and `Debug` are both
    /// `1` and only the pair with [`Self::level`] identifies either.
    pub(crate) const fn name(self) -> i32 {
        match self {
            Self::Debug => 0x0001,
            Self::AcceptConn => 0x0002,
            Self::ReuseAddr => 0x0004,
            Self::KeepAlive => 0x0008,
            Self::DontRoute => 0x0010,
            Self::Broadcast => 0x0020,
            Self::UseLoopback => 0x0040,
            Self::Linger => 0x0080,
            Self::OobInline => 0x0100,
            Self::ReusePort => 0x0200,
            Self::Timestamp => 0x0400,
            Self::NoSigpipe => 0x0800,
            Self::SendBuffer => 0x1001,
            Self::RecvBuffer => 0x1002,
            Self::SendLowWater => 0x1003,
            Self::RecvLowWater => 0x1004,
            Self::SendTimeout => 0x1005,
            Self::RecvTimeout => 0x1006,
            Self::Error => 0x1007,
            Self::Type => 0x1008,

            Self::IpTos => 3,
            Self::IpTtl => 4,
            Self::IpMulticastIf => 9,
            Self::IpMulticastTtl => 10,
            Self::IpMulticastLoop => 11,
            Self::IpAddMembership => 12,
            Self::IpDropMembership => 13,
            Self::IpRecvTtl => 65,
            Self::IpMinTtl => 66,
            Self::IpDontFrag => 67,

            Self::TcpNoDelay => 1,
            Self::TcpMaxSegment => 2,
            Self::TcpNoPush => 4,
            Self::TcpNoOpt => 8,
            Self::TcpKeepInit => 128,
            Self::TcpKeepIdle => 256,
            Self::TcpKeepInterval => 512,
            Self::TcpKeepCount => 1024,
        }
    }
}
