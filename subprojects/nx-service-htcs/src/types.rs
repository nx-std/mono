//! HTCS wire-layout types and enums.

use static_assertions::const_assert_eq;

/// Maximum length of a peer name.
pub const PEER_NAME_MAX: usize = 32;

/// Maximum length of a port name.
pub const PORT_NAME_MAX: usize = 32;

/// Maximum number of sockets in an fd set.
pub const SOCKET_COUNT_MAX: usize = 40;

/// Maximum number of entries in an fd set (same as socket count).
pub const FD_SET_SIZE: usize = SOCKET_COUNT_MAX;

/// Maximum session count for the HTCS session pool.
pub const SESSION_COUNT_MAX: usize = 0x10;

/// HTC peer name (32-byte fixed string).
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct HtcsPeerName {
    pub name: [u8; PEER_NAME_MAX],
}

const_assert_eq!(size_of::<HtcsPeerName>(), 0x20);

/// HTC port name (32-byte fixed string).
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct HtcsPortName {
    pub name: [u8; PORT_NAME_MAX],
}

const_assert_eq!(size_of::<HtcsPortName>(), 0x20);

/// HTC socket address.
#[derive(
    Clone,
    Copy,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct HtcsSockAddr {
    pub family: u16,
    pub peer_name: HtcsPeerName,
    pub port_name: HtcsPortName,
}

const_assert_eq!(size_of::<HtcsSockAddr>(), 0x42);

/// Time value for select operations.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct HtcsTimeVal {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

const_assert_eq!(size_of::<HtcsTimeVal>(), 0x10);

/// File descriptor set for select operations.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HtcsFdSet {
    pub fds: [i32; FD_SET_SIZE],
}

const_assert_eq!(size_of::<HtcsFdSet>(), 0xA0);

/// HTCS socket error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum HtcsSocketError {
    None = 0,
    Acces = 2,
    AddrInUse = 3,
    AddrNotAvail = 4,
    Again = 6,
    Already = 7,
    Badf = 8,
    Busy = 10,
    ConnAborted = 13,
    ConnRefused = 14,
    ConnReset = 15,
    DestAddrReq = 17,
    Fault = 21,
    InProgress = 26,
    Intr = 27,
    Inval = 28,
    Io = 29,
    IsConn = 30,
    Mfile = 33,
    MsgSize = 35,
    NetDown = 38,
    NetReset = 39,
    NoBufs = 42,
    NoMem = 49,
    NotConn = 56,
    TimedOut = 76,
    Unknown = 79,
}

/// HTCS shutdown types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum HtcsShutdownType {
    /// Shut down reads.
    Rd = 0,
    /// Shut down writes.
    Wr = 1,
    /// Shut down both reads and writes.
    RdWr = 2,
}

/// HTCS fcntl operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum HtcsFcntlOperation {
    /// Get file flags.
    GetFl = 3,
    /// Set file flags.
    SetFl = 4,
}

/// HTCS address family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum HtcsAddressFamily {
    /// HTCS address family.
    Htcs = 0,
}

bitflags::bitflags! {
    /// HTCS message flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HtcsMessageFlag: i32 {
        /// Peek at incoming data without consuming it.
        const PEEK = 1;
        /// Wait for the full amount of data requested.
        const WAITALL = 2;
    }
}

bitflags::bitflags! {
    /// HTCS fcntl flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HtcsFcntlFlag: i32 {
        /// Non-blocking I/O.
        const NONBLOCK = 4;
    }
}

/// Result of a socket operation returning an error code and a result value.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct SocketResult {
    pub err: i32,
    pub res: i32,
}

const_assert_eq!(size_of::<SocketResult>(), 0x08);

/// Result of a transfer operation returning an error code and byte count.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct TransferResult {
    pub err: i32,
    pub size: i64,
}

const_assert_eq!(size_of::<TransferResult>(), 0x10);

/// Input for fcntl (cmd 8).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct FcntlIn {
    pub command: i32,
    pub value: i32,
}

const_assert_eq!(size_of::<FcntlIn>(), 0x08);

/// Input for recv_start (cmd 11).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct RecvStartIn {
    pub mem_size: i32,
    pub flags: i32,
}

const_assert_eq!(size_of::<RecvStartIn>(), 0x08);

/// Input for start_send (cmd 17) and start_recv (cmd 20).
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct StartTransferIn {
    pub flags: i32,
    _pad: u32,
    pub size: i64,
}

const_assert_eq!(size_of::<StartTransferIn>(), 0x10);

impl StartTransferIn {
    pub fn new(flags: i32, size: i64) -> Self {
        Self {
            flags,
            _pad: 0,
            size,
        }
    }
}

/// Output for start_send (cmd 17).
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct StartSendOut {
    pub task_id: u32,
    _pad: u32,
    pub max_size: i64,
}

const_assert_eq!(size_of::<StartSendOut>(), 0x10);

/// Output for continue_send (cmd 23).
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct ContinueSendOut {
    pub wait: u8,
    _pad: [u8; 7],
    pub size: i64,
}

const_assert_eq!(size_of::<ContinueSendOut>(), 0x10);

/// Output for accept_results (cmd 10).
#[derive(Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct AcceptResultsOut {
    pub address: HtcsSockAddr,
    _pad: [u8; 2],
    pub err: i32,
}

const_assert_eq!(size_of::<AcceptResultsOut>(), 0x48);

/// Output for end_select (cmd 131).
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub(crate) struct EndSelectOut {
    pub err: i32,
    pub count: i32,
}

const_assert_eq!(size_of::<EndSelectOut>(), 0x08);

/// Result of a start_send operation.
#[derive(Debug, Clone, Copy)]
pub struct StartSendResult {
    /// Task ID for the send operation.
    pub task_id: u32,
    /// Event handle for completion notification.
    pub event_handle: u32,
    /// Maximum size that can be sent in this operation.
    pub max_size: i64,
}

/// Result of a continue_send operation.
#[derive(Debug, Clone, Copy)]
pub struct ContinueSendResult {
    /// Number of bytes consumed from the buffer.
    pub size: i64,
    /// Whether the caller should wait before sending more data.
    pub wait: bool,
}

/// Result of an accept operation.
pub struct AcceptResultData<'svc> {
    /// HTCS-level error code.
    pub err: i32,
    /// Address of the accepted peer.
    pub address: HtcsSockAddr,
    /// The accepted socket.
    pub socket: super::HtcsSocket<'svc>,
}

/// Result of a select end operation.
#[derive(Debug, Clone, Copy)]
pub struct EndSelectResult {
    /// HTCS-level error code.
    pub err: i32,
    /// Number of ready file descriptors.
    pub count: i32,
}
