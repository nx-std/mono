//! BSD service protocol constants and wire-format types.
//!
//! Every `#[repr(C)]` struct here crosses the IPC boundary, so each must match
//! the layout the service reads, byte for byte.

use core::mem::size_of;

use nx_sf::ServiceName;
use static_assertions::const_assert_eq;

/// Service name `bsd:u` (user-mode BSD sockets).
pub const SERVICE_NAME_USER: ServiceName = ServiceName::new_truncate("bsd:u");

/// Service name `bsd:s` (system-mode BSD sockets).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("bsd:s");

/// A command in the `IBsdServices` interface.
///
/// Pairs the wire command id with a name, so a request is built from a variant
/// rather than a bare integer and a failure can say which command produced it.
/// Command `28` is absent from the interface, and so from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Initial handshake establishing the transfer-memory backed session.
    /// Sent on the main service handle.
    RegisterClient,
    /// Enables session monitoring. Sent on the monitor service handle.
    StartMonitoring,
    /// Creates a socket.
    Socket,
    /// Creates a socket exempt from the system's socket accounting.
    SocketExempt,
    /// Opens a path in the service's own namespace.
    Open,
    /// Waits for readiness across three descriptor sets.
    Select,
    /// Waits for readiness across a descriptor array.
    Poll,
    /// Reads or writes a kernel networking parameter.
    Sysctl,
    /// Receives from a connected socket.
    Recv,
    /// Receives, reporting the sender's address.
    RecvFrom,
    /// Sends on a connected socket.
    Send,
    /// Sends to an explicit address.
    SendTo,
    /// Takes the next connection off a listening socket's queue.
    Accept,
    /// Assigns a local address to a socket.
    Bind,
    /// Initiates a connection to a peer.
    Connect,
    /// Reports the address of the connected peer.
    GetPeerName,
    /// Reports the socket's own address.
    GetSockName,
    /// Reads a socket option.
    GetSockOpt,
    /// Marks a socket as accepting connections.
    Listen,
    /// Issues a device control request against a descriptor.
    Ioctl,
    /// Reads or replaces a descriptor's status flags.
    Fcntl,
    /// Writes a socket option.
    SetSockOpt,
    /// Disables further sends, receives, or both.
    Shutdown,
    /// Disables further transfer on every socket this client owns.
    ShutdownAllSockets,
    /// Writes to a descriptor.
    Write,
    /// Reads from a descriptor.
    Read,
    /// Releases a descriptor.
    Close,
    /// Produces a second descriptor for one socket.
    DuplicateSocket,
    /// Receives several messages in one request.
    RecvMMsg,
    /// Sends several messages in one request.
    SendMMsg,
}

impl Command {
    /// Returns the wire command id.
    pub const fn id(self) -> u32 {
        match self {
            Self::RegisterClient => 0,
            Self::StartMonitoring => 1,
            Self::Socket => 2,
            Self::SocketExempt => 3,
            Self::Open => 4,
            Self::Select => 5,
            Self::Poll => 6,
            Self::Sysctl => 7,
            Self::Recv => 8,
            Self::RecvFrom => 9,
            Self::Send => 10,
            Self::SendTo => 11,
            Self::Accept => 12,
            Self::Bind => 13,
            Self::Connect => 14,
            Self::GetPeerName => 15,
            Self::GetSockName => 16,
            Self::GetSockOpt => 17,
            Self::Listen => 18,
            Self::Ioctl => 19,
            Self::Fcntl => 20,
            Self::SetSockOpt => 21,
            Self::Shutdown => 22,
            Self::ShutdownAllSockets => 23,
            Self::Write => 24,
            Self::Read => 25,
            Self::Close => 26,
            Self::DuplicateSocket => 27,
            Self::RecvMMsg => 29,
            Self::SendMMsg => 30,
        }
    }

    /// Returns the name a socket programmer knows this command by.
    ///
    /// Chosen over the `IBsdServices` method name because this is what appears
    /// in a failure the caller has to act on. The commands with no POSIX
    /// equivalent keep their interface name.
    const fn name(self) -> &'static str {
        match self {
            Self::RegisterClient => "RegisterClient",
            Self::StartMonitoring => "StartMonitoring",
            Self::Socket => "socket",
            Self::SocketExempt => "SocketExempt",
            Self::Open => "open",
            Self::Select => "select",
            Self::Poll => "poll",
            Self::Sysctl => "sysctl",
            Self::Recv => "recv",
            Self::RecvFrom => "recvfrom",
            Self::Send => "send",
            Self::SendTo => "sendto",
            Self::Accept => "accept",
            Self::Bind => "bind",
            Self::Connect => "connect",
            Self::GetPeerName => "getpeername",
            Self::GetSockName => "getsockname",
            Self::GetSockOpt => "getsockopt",
            Self::Listen => "listen",
            Self::Ioctl => "ioctl",
            Self::Fcntl => "fcntl",
            Self::SetSockOpt => "setsockopt",
            Self::Shutdown => "shutdown",
            Self::ShutdownAllSockets => "ShutdownAllSockets",
            Self::Write => "write",
            Self::Read => "read",
            Self::Close => "close",
            Self::DuplicateSocket => "DuplicateSocket",
            Self::RecvMMsg => "recvmmsg",
            Self::SendMMsg => "sendmmsg",
        }
    }
}

impl core::fmt::Display for Command {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())
    }
}

/// The service configuration shipped with `RegisterClient`.
///
/// Every buffer size is in
/// bytes and is the service's, not this process's: the transfer memory
/// registered alongside is what they are carved out of.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct BsdServiceConfigWire {
    /// Service interface version: `1` on `[2.0.0+]`, `2` on `[3.0.0+]`.
    pub version: u32,
    /// Initial TCP transmit buffer size.
    pub tcp_tx_buf_size: u32,
    /// Initial TCP receive buffer size.
    pub tcp_rx_buf_size: u32,
    /// Ceiling the TCP transmit buffer may grow to; `0` pins it to the
    /// initial size.
    pub tcp_tx_buf_max_size: u32,
    /// Ceiling the TCP receive buffer may grow to; `0` pins it to the initial
    /// size.
    pub tcp_rx_buf_max_size: u32,
    /// UDP transmit buffer size, which does not grow.
    pub udp_tx_buf_size: u32,
    /// UDP receive buffer size, which does not grow.
    pub udp_rx_buf_size: u32,
    /// Buffers held per socket. Multiplies the transfer memory the service
    /// requires.
    pub sb_efficiency: u32,
}
const_assert_eq!(size_of::<BsdServiceConfigWire>(), 32);

/// Input payload for `RegisterClient`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct RegisterClientIn {
    /// The buffer sizes the service should provision for this client.
    pub config: BsdServiceConfigWire,
    /// Slot the kernel overwrites with the sending process id. Sent as zero;
    /// the request carries `send_pid` and the kernel fills this in.
    pub pid_placeholder: u64,
    /// Size of the transfer memory whose handle accompanies this request.
    pub tmem_size: u64,
}
const_assert_eq!(size_of::<RegisterClientIn>(), 48);

/// The response prefix every dispatched command shares.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
pub(crate) struct CallResponse {
    /// The command's return value, POSIX-style: negative means it was
    /// rejected, and only then does `error_code` mean anything.
    pub ret: i32,
    /// The condition behind a rejection, in the service's own Linux
    /// numbering. Not a C `errno` — see [`crate::posix`].
    pub error_code: i32,
}
const_assert_eq!(size_of::<CallResponse>(), 8);

/// [`CallResponse`] followed by a `u32`: a `socklen_t` for `accept`,
/// `getsockname`, `getpeername` and `recvfrom`, or an `optlen` for
/// `getsockopt`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
pub(crate) struct CallResponseExtraU32 {
    /// The shared prefix.
    pub prefix: CallResponse,
    /// The appended length.
    pub extra: u32,
}
const_assert_eq!(size_of::<CallResponseExtraU32>(), 12);

/// [`CallResponse`] followed by a `u64`: the `size_t` `sysctl` reports as the
/// length it wrote.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
pub(crate) struct CallResponseExtraU64 {
    /// The shared prefix.
    pub prefix: CallResponse,
    /// The appended length.
    pub extra: u64,
}
const_assert_eq!(size_of::<CallResponseExtraU64>(), 16);

/// Input payload for `Socket` and `SocketExempt`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SocketIn {
    /// Address family (`AF_INET`, `AF_INET6`, …).
    pub domain: i32,
    /// Socket type (`SOCK_STREAM`, `SOCK_DGRAM`, …).
    pub type_: i32,
    /// Protocol within the family, or `0` for the family's default.
    pub protocol: i32,
}

/// Input payload for the transfer commands: `Recv`, `RecvFrom`, `Send`,
/// `SendTo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SockfdFlagsIn {
    /// The socket to transfer on.
    pub sockfd: i32,
    /// Transfer flags (`MSG_*`).
    pub flags: i32,
}

/// Input payload for `GetSockOpt` and `SetSockOpt`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SockOptIn {
    /// The socket whose option is being read or written.
    pub sockfd: i32,
    /// Protocol level the option belongs to (`SOL_SOCKET`, `IPPROTO_TCP`, …).
    pub level: i32,
    /// The option within that level (`SO_*`, `TCP_*`, …).
    pub optname: i32,
}

/// Input payload for `Listen`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct ListenIn {
    /// The socket to start accepting on.
    pub sockfd: i32,
    /// How many connections may wait in the queue.
    pub backlog: i32,
}

/// Input payload for `Shutdown`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct ShutdownIn {
    /// The socket to shut down.
    pub sockfd: i32,
    /// Which directions to disable (`SHUT_RD`, `SHUT_WR`, `SHUT_RDWR`).
    pub how: i32,
}

/// Input payload for `Fcntl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct FcntlIn {
    /// The descriptor whose flags are being read or replaced.
    pub fd: i32,
    /// The operation: only `F_GETFL` and `F_SETFL` are answered.
    pub cmd: i32,
    /// The flags to install, or zero on a read.
    pub flags: i32,
}

/// Input payload for `Ioctl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct IoctlIn {
    /// The descriptor the request is issued against.
    pub fd: i32,
    /// The request code, carrying its own direction bits and payload length.
    pub request: i32,
    /// How many buffers accompany the request: one for a flat payload, two
    /// for the requests that answer with a header plus a list.
    pub bufcount: i32,
}

/// Input payload for `DuplicateSocket`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct DuplicateSocketIn {
    /// The socket to produce a second descriptor for.
    pub sockfd: i32,
    /// Alignment padding ahead of the reserved word.
    pub _pad: u32,
    /// Reserved by the interface. Sent as zero.
    pub reserved: u64,
}
const_assert_eq!(size_of::<DuplicateSocketIn>(), 16);

/// `struct timeval` as `Select` carries it.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct Timeval {
    /// Whole seconds.
    pub tv_sec: i64,
    /// Microseconds past `tv_sec`.
    pub tv_usec: i64,
}
const_assert_eq!(size_of::<Timeval>(), 16);

/// `struct timespec` as `RecvMMsg` carries it.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct Timespec {
    /// Whole seconds.
    pub tv_sec: i64,
    /// Nanoseconds past `tv_sec`.
    pub tv_nsec: i64,
}
const_assert_eq!(size_of::<Timespec>(), 16);

/// A `timeval` plus the flag that says to ignore it.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SelectTimeval {
    /// How long to wait, meaningful only when `is_null` is zero.
    pub tv: Timeval,
    /// Non-zero to wait indefinitely, which is what a C caller passing a null
    /// `struct timeval *` means. The interface carries a C `bool` here; the
    /// same byte is written as a `u8`.
    pub is_null: u8,
    /// Padding out to the struct's 8-byte alignment.
    pub _pad: [u8; 7],
}
const_assert_eq!(size_of::<SelectTimeval>(), 24);

/// Input payload for `Select`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SelectIn {
    /// One past the highest descriptor number in any of the three sets.
    pub nfds: i32,
    /// Alignment padding ahead of the timeout.
    pub _pad: u32,
    /// How long to wait for one of the sets to become ready.
    pub timeout: SelectTimeval,
}
const_assert_eq!(size_of::<SelectIn>(), 32);

/// Input payload for `Poll`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct PollIn {
    /// How many `pollfd` entries the accompanying buffer holds. `nfds_t` is
    /// `unsigned int` on this target, so the field is 32 bits wide and the
    /// timeout follows it immediately. Widening it costs the timeout: the
    /// service reads that field from the next four bytes either way, and those
    /// bytes are the high half of a 64-bit count, which is zero for every set
    /// anyone can name. A wait asked for in that shape is answered at once.
    pub nfds: u32,
    /// Milliseconds to wait; negative waits indefinitely.
    pub timeout: i32,
}
const_assert_eq!(size_of::<PollIn>(), 8);

/// Input payload for `RecvMMsg`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct RecvMMsgIn {
    /// The socket to receive on.
    pub sockfd: i32,
    /// How many messages the accompanying `mmsghdr` array holds.
    pub vlen: i32,
    /// Transfer flags (`MSG_*`).
    pub flags: i32,
    /// Alignment padding ahead of the timeout.
    pub _pad: u32,
    /// How long to wait for the first message.
    pub timeout: Timespec,
}
const_assert_eq!(size_of::<RecvMMsgIn>(), 32);

/// Input payload for `SendMMsg`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SendMMsgIn {
    /// The socket to send on.
    pub sockfd: i32,
    /// How many messages the accompanying `mmsghdr` array holds.
    pub vlen: i32,
    /// Transfer flags (`MSG_*`).
    pub flags: i32,
}
const_assert_eq!(size_of::<SendMMsgIn>(), 12);
