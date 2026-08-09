//! The error conditions the BSD socket service reports.
//!
//! Every command answers with a `{ int ret; int errno; }` prefix, and the
//! second word is where a failed command says what went wrong. That word is
//! **not** a C `errno`: Horizon's BSD service was built against the Linux
//! error numbering, while the C library this workspace links against uses
//! newlib's. The two disagree on most values above 34 — Linux `95` is
//! `EOPNOTSUPP`, newlib's `95` is something else entirely — so a code copied
//! from the wire into a C `errno` slot reports the wrong failure.
//!
//! [`PosixError`] is what keeps the two apart. The wire word is parsed into
//! it once, at the response boundary, and every consumer above works in named
//! conditions from there. Only a C-facing surface needs a number again, and
//! producing one is that surface's job: it knows which numbering its caller
//! reads, which this crate does not.
//!
//! # References
//!
//! - `subprojects/libnx/src/nx/source/runtime/devices/convert_errno.c` — the
//!   Linux-index-to-newlib table libnx applies on the way out to C.

/// A failure condition reported by the BSD socket service.
///
/// Named after the POSIX condition rather than any one platform's number for
/// it. [`Self::Unknown`] carries a code this enum has no name for, in the
/// service's own Linux numbering, so a response is never lost in translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PosixError {
    /// Operation not permitted (Linux `1`).
    #[error("operation not permitted")]
    NotPermitted,
    /// No such file or directory (Linux `2`).
    #[error("no such file or directory")]
    NotFound,
    /// Interrupted system call (Linux `4`).
    #[error("interrupted")]
    Interrupted,
    /// Input/output error (Linux `5`).
    #[error("input/output error")]
    Io,
    /// Bad file descriptor (Linux `9`).
    #[error("bad file descriptor")]
    BadFd,
    /// Resource temporarily unavailable (Linux `11`).
    ///
    /// What a non-blocking socket reports when the operation would have had
    /// to wait. The caller retries rather than treating it as a failure.
    #[error("resource temporarily unavailable")]
    WouldBlock,
    /// Cannot allocate memory (Linux `12`).
    #[error("cannot allocate memory")]
    OutOfMemory,
    /// Permission denied (Linux `13`).
    #[error("permission denied")]
    PermissionDenied,
    /// Bad address (Linux `14`).
    #[error("bad address")]
    BadAddress,
    /// Device or resource busy (Linux `16`).
    #[error("device or resource busy")]
    Busy,
    /// File exists (Linux `17`).
    #[error("file exists")]
    AlreadyExists,
    /// Invalid argument (Linux `22`).
    #[error("invalid argument")]
    InvalidArgument,
    /// Too many open files in the system (Linux `23`).
    #[error("too many open files in the system")]
    SystemFdLimit,
    /// Too many open files for this process (Linux `24`).
    #[error("too many open files")]
    ProcessFdLimit,
    /// Inappropriate ioctl for device (Linux `25`).
    #[error("inappropriate ioctl for device")]
    NotATty,
    /// No space left on device (Linux `28`).
    #[error("no space left on device")]
    StorageFull,
    /// Illegal seek (Linux `29`).
    #[error("illegal seek")]
    NotSeekable,
    /// Broken pipe (Linux `32`).
    #[error("broken pipe")]
    BrokenPipe,
    /// File name too long (Linux `36`).
    #[error("file name too long")]
    NameTooLong,
    /// Function not implemented (Linux `38`).
    #[error("function not implemented")]
    Unsupported,
    /// Too many levels of symbolic links (Linux `40`).
    #[error("too many levels of symbolic links")]
    TooManySymbolicLinks,
    /// Bad message (Linux `74`).
    #[error("bad message")]
    BadMessage,
    /// Value too large for defined data type (Linux `75`).
    #[error("value too large for defined data type")]
    Overflow,
    /// Invalid or incomplete multibyte or wide character (Linux `84`).
    #[error("invalid or incomplete multibyte or wide character")]
    IllegalByteSequence,
    /// Socket operation on non-socket (Linux `88`).
    #[error("socket operation on non-socket")]
    NotASocket,
    /// Destination address required (Linux `89`).
    #[error("destination address required")]
    DestinationAddressRequired,
    /// Message too long (Linux `90`).
    #[error("message too long")]
    MessageTooLong,
    /// Protocol wrong type for socket (Linux `91`).
    #[error("protocol wrong type for socket")]
    WrongProtocolType,
    /// Protocol not available (Linux `92`).
    #[error("protocol not available")]
    ProtocolNotAvailable,
    /// Protocol not supported (Linux `93`).
    #[error("protocol not supported")]
    ProtocolNotSupported,
    /// Socket type not supported (Linux `94`).
    #[error("socket type not supported")]
    SocketTypeNotSupported,
    /// Operation not supported (Linux `95`).
    #[error("operation not supported")]
    OperationNotSupported,
    /// Protocol family not supported (Linux `96`).
    #[error("protocol family not supported")]
    ProtocolFamilyNotSupported,
    /// Address family not supported by protocol (Linux `97`).
    #[error("address family not supported by protocol")]
    AddressFamilyNotSupported,
    /// Address already in use (Linux `98`).
    #[error("address already in use")]
    AddressInUse,
    /// Cannot assign requested address (Linux `99`).
    #[error("cannot assign requested address")]
    AddressNotAvailable,
    /// Network is down (Linux `100`).
    #[error("network is down")]
    NetworkDown,
    /// Network is unreachable (Linux `101`).
    #[error("network is unreachable")]
    NetworkUnreachable,
    /// Network dropped connection on reset (Linux `102`).
    #[error("network dropped connection on reset")]
    NetworkReset,
    /// Software caused connection abort (Linux `103`).
    #[error("software caused connection abort")]
    ConnectionAborted,
    /// Connection reset by peer (Linux `104`).
    #[error("connection reset by peer")]
    ConnectionReset,
    /// No buffer space available (Linux `105`).
    #[error("no buffer space available")]
    NoBufferSpace,
    /// Transport endpoint is already connected (Linux `106`).
    #[error("transport endpoint is already connected")]
    AlreadyConnected,
    /// Transport endpoint is not connected (Linux `107`).
    #[error("transport endpoint is not connected")]
    NotConnected,
    /// Cannot send after transport endpoint shutdown (Linux `108`).
    #[error("cannot send after transport endpoint shutdown")]
    Shutdown,
    /// Too many references: cannot splice (Linux `109`).
    #[error("too many references: cannot splice")]
    TooManyReferences,
    /// Connection timed out (Linux `110`).
    #[error("connection timed out")]
    TimedOut,
    /// Connection refused (Linux `111`).
    #[error("connection refused")]
    ConnectionRefused,
    /// Host is down (Linux `112`).
    #[error("host is down")]
    HostDown,
    /// No route to host (Linux `113`).
    #[error("no route to host")]
    HostUnreachable,
    /// Operation already in progress (Linux `114`).
    #[error("operation already in progress")]
    AlreadyInProgress,
    /// Operation now in progress (Linux `115`).
    ///
    /// What a non-blocking `connect` reports once the handshake has been
    /// started; the caller waits for writability rather than retrying.
    #[error("operation now in progress")]
    InProgress,
    /// Operation canceled (Linux `125`).
    #[error("operation canceled")]
    Canceled,
    /// Owner died (Linux `130`).
    #[error("owner died")]
    OwnerDead,
    /// State not recoverable (Linux `131`).
    #[error("state not recoverable")]
    NotRecoverable,
    /// A condition this enum has no name for.
    ///
    /// Carries the service's own code, in the Linux numbering it was sent
    /// in. Reaching this means either a condition no socket command was
    /// expected to produce, or one worth adding a name for.
    #[error("unrecognized error condition (service code {0})")]
    Unknown(i32),
}

impl From<i32> for PosixError {
    /// Classifies a code as the service sent it, in Linux numbering.
    ///
    /// Total by construction: anything without a name lands in
    /// [`PosixError::Unknown`] carrying the code unchanged.
    fn from(code: i32) -> Self {
        match code {
            1 => Self::NotPermitted,
            2 => Self::NotFound,
            4 => Self::Interrupted,
            5 => Self::Io,
            9 => Self::BadFd,
            11 => Self::WouldBlock,
            12 => Self::OutOfMemory,
            13 => Self::PermissionDenied,
            14 => Self::BadAddress,
            16 => Self::Busy,
            17 => Self::AlreadyExists,
            22 => Self::InvalidArgument,
            23 => Self::SystemFdLimit,
            24 => Self::ProcessFdLimit,
            25 => Self::NotATty,
            28 => Self::StorageFull,
            29 => Self::NotSeekable,
            32 => Self::BrokenPipe,
            36 => Self::NameTooLong,
            38 => Self::Unsupported,
            40 => Self::TooManySymbolicLinks,
            74 => Self::BadMessage,
            75 => Self::Overflow,
            84 => Self::IllegalByteSequence,
            88 => Self::NotASocket,
            89 => Self::DestinationAddressRequired,
            90 => Self::MessageTooLong,
            91 => Self::WrongProtocolType,
            92 => Self::ProtocolNotAvailable,
            93 => Self::ProtocolNotSupported,
            94 => Self::SocketTypeNotSupported,
            95 => Self::OperationNotSupported,
            96 => Self::ProtocolFamilyNotSupported,
            97 => Self::AddressFamilyNotSupported,
            98 => Self::AddressInUse,
            99 => Self::AddressNotAvailable,
            100 => Self::NetworkDown,
            101 => Self::NetworkUnreachable,
            102 => Self::NetworkReset,
            103 => Self::ConnectionAborted,
            104 => Self::ConnectionReset,
            105 => Self::NoBufferSpace,
            106 => Self::AlreadyConnected,
            107 => Self::NotConnected,
            108 => Self::Shutdown,
            109 => Self::TooManyReferences,
            110 => Self::TimedOut,
            111 => Self::ConnectionRefused,
            112 => Self::HostDown,
            113 => Self::HostUnreachable,
            114 => Self::AlreadyInProgress,
            115 => Self::InProgress,
            125 => Self::Canceled,
            130 => Self::OwnerDead,
            131 => Self::NotRecoverable,
            other => Self::Unknown(other),
        }
    }
}
