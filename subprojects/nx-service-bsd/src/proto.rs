//! BSD service protocol constants and wire-format types.
//!
//! Mirrors `subprojects/libnx/src/nx/source/services/bsd.c`. Every `#[repr(C)]`
//! struct here crosses the IPC boundary and must match libnx byte for byte.

use core::mem::size_of;

use nx_sf::ServiceName;
use static_assertions::const_assert_eq;

/// Service name `bsd:u` (user-mode BSD sockets).
pub const SERVICE_NAME_USER: ServiceName = ServiceName::new_truncate("bsd:u");

/// Service name `bsd:s` (system-mode BSD sockets).
pub const SERVICE_NAME_SYSTEM: ServiceName = ServiceName::new_truncate("bsd:s");

/// `IBsdServices` command IDs (libnx `bsd.c`).
pub mod cmds {
    /// `RegisterClient` — initial handshake establishing the transfer-memory
    /// backed session. Sent on the main service handle.
    pub const REGISTER_CLIENT: u32 = 0;
    /// `StartMonitoring` — enables session monitoring. Sent on the monitor
    /// service handle.
    pub const START_MONITORING: u32 = 1;
    pub const SOCKET: u32 = 2;
    pub const SELECT: u32 = 5;
    pub const POLL: u32 = 6;
    pub const RECV: u32 = 8;
    pub const RECV_FROM: u32 = 9;
    pub const SEND: u32 = 10;
    pub const SEND_TO: u32 = 11;
    pub const ACCEPT: u32 = 12;
    pub const BIND: u32 = 13;
    pub const CONNECT: u32 = 14;
    pub const GET_PEER_NAME: u32 = 15;
    pub const GET_SOCK_NAME: u32 = 16;
    pub const GET_SOCK_OPT: u32 = 17;
    pub const LISTEN: u32 = 18;
    pub const IOCTL: u32 = 19;
    pub const FCNTL: u32 = 20;
    pub const SET_SOCK_OPT: u32 = 21;
    pub const SHUTDOWN: u32 = 22;
    pub const WRITE: u32 = 24;
    pub const READ: u32 = 25;
    pub const CLOSE: u32 = 26;
}

/// Wire-format `BsdServiceConfig` shipped to `RegisterClient` (cmd 0).
///
/// Layout matches `BsdServiceConfig` in libnx `bsd.c`. All fields are `u32`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct BsdServiceConfigWire {
    pub version: u32,
    pub tcp_tx_buf_size: u32,
    pub tcp_rx_buf_size: u32,
    pub tcp_tx_buf_max_size: u32,
    pub tcp_rx_buf_max_size: u32,
    pub udp_tx_buf_size: u32,
    pub udp_rx_buf_size: u32,
    pub sb_efficiency: u32,
}
const_assert_eq!(size_of::<BsdServiceConfigWire>(), 32);

/// Input payload for `RegisterClient`. Embeds the service config followed by
/// the placeholder slot for the PID descriptor and the transfer-memory size.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct RegisterClientIn {
    pub config: BsdServiceConfigWire,
    pub pid_placeholder: u64,
    pub tmem_size: u64,
}
const_assert_eq!(size_of::<RegisterClientIn>(), 48);

/// Common response prefix returned by every BSD service command. The service
/// reports POSIX-style outcomes via `ret`; `errno` is meaningful when `ret < 0`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct CallResponse {
    pub ret: i32,
    pub errno: i32,
}
const_assert_eq!(size_of::<CallResponse>(), 8);

/// Input payload for [`cmds::SOCKET`] / `SocketExempt`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SocketIn {
    pub domain: i32,
    pub type_: i32,
    pub protocol: i32,
}

/// Input payload for commands that take only `(sockfd, flags)`
/// (`Recv` / `RecvFrom` / `Send` / `SendTo`).
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SockfdFlagsIn {
    pub sockfd: i32,
    pub flags: i32,
}

/// Input payload for commands that take only `(sockfd, level, optname)`
/// (`GetSockOpt` / `SetSockOpt`).
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SockOptIn {
    pub sockfd: i32,
    pub level: i32,
    pub optname: i32,
}

/// Input payload for `Listen`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct ListenIn {
    pub sockfd: i32,
    pub backlog: i32,
}

/// Input payload for `Shutdown`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct ShutdownIn {
    pub sockfd: i32,
    pub how: i32,
}

/// Input payload for `Fcntl` (`F_GETFL`/`F_SETFL` only).
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct FcntlIn {
    pub fd: i32,
    pub cmd: i32,
    pub flags: i32,
}

/// Input payload for `Ioctl` (generic case — special `SIOC*` variants are not
/// yet supported by this port).
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct IoctlIn {
    pub fd: i32,
    pub request: i32,
    pub bufcount: i32,
}

/// `timeval` wire layout (POSIX `struct timeval`) used by `Select`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}
const_assert_eq!(size_of::<Timeval>(), 16);

/// `BsdSelectTimeval` wire layout — a `timeval` plus a "null" flag. Aligned to
/// 8 bytes; the boolean occupies one byte and the rest is padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SelectTimeval {
    pub tv: Timeval,
    /// libnx writes a C `bool` (1 byte); we encode the same byte with `u8`.
    /// Trailing padding to 8-byte alignment is implicit.
    pub is_null: u8,
    pub _pad: [u8; 7],
}
const_assert_eq!(size_of::<SelectTimeval>(), 24);

/// Input payload for `Select`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct SelectIn {
    pub nfds: i32,
    pub _pad: u32,
    pub timeout: SelectTimeval,
}
const_assert_eq!(size_of::<SelectIn>(), 32);

/// Input payload for `Poll`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub(crate) struct PollIn {
    /// `nfds_t` in libnx is `unsigned long` (64-bit on Switch / aarch64).
    pub nfds: u64,
    pub timeout: i32,
    pub _pad: u32,
}
const_assert_eq!(size_of::<PollIn>(), 16);
