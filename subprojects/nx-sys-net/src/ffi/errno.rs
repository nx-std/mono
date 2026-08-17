//! Reporting a failed socket call to a C caller.
//!
//! A C socket call reports failure by returning `-1` and leaving the reason in two places: the
//! calling thread's `errno`, and — for the failures that are not POSIX conditions at all — a
//! thread-local Horizon result code the caller reads back with `socketGetLastResult`. Every export
//! in this module tree funnels through [`report`], so the pair is always written together and
//! never disagrees.
//!
//! ## Two numberings, one translation
//!
//! The service answers in Linux's error numbering and the C library here uses newlib's. They agree
//! below 35 and diverge above it: Linux `95` is `EOPNOTSUPP`, newlib's `95` is not. So the wire
//! code was parsed into a named [`PosixError`] at the response boundary, and this is where a number
//! comes back — the one newlib's headers give that condition.
//!
//! The C driver does the same job with a 134-entry table indexed by the Linux code. A named
//! condition is clearer and covers everything the service actually reports; what the table has and
//! this does not are codes with no socket meaning, which reach [`PosixError::Unknown`] and come out
//! as `10000 + code`. That offset is not an invention: it is what the C table itself produces for
//! an entry it has no mapping for.

use core::ffi::c_int;

use nx_service_bsd::{
    CommandError,
    PosixError,
};
use nx_sf::error::{
    ResultCode,
    ToResultCode as _,
};

/// The Horizon result code the last failed call left behind.
///
/// Zero when the last failure was one the service reported as a POSIX condition, which is every
/// ordinary socket failure. Non-zero only when the IPC round trip itself failed, which is the case
/// `errno` cannot describe and this exists for.
#[thread_local]
static LAST_RESULT: core::cell::Cell<u32> = core::cell::Cell::new(0);

/// Returns the result code the last failed call recorded on this thread.
pub fn last_result() -> u32 {
    LAST_RESULT.get()
}

/// Clears the recorded result code on this thread.
///
/// The driver does this when it comes up, so that a result left by an earlier session is not read
/// back as though it described the new one.
pub fn clear_last_result() {
    LAST_RESULT.set(0);
}

/// Reports `err` to the calling thread and returns C's integer failure value.
///
/// The single place a failed command becomes a C return: it records the result code, writes
/// `errno`, and produces the `-1` every one of these calls returns on failure.
pub fn report(err: CommandError) -> c_int {
    match err {
        // The service executed the command and rejected it. This is a POSIX condition and nothing
        // else, so the result code is cleared: a caller checking it must not find a stale value
        // from an earlier transport failure and conclude this failure was one too.
        CommandError::Service { source, .. } => {
            LAST_RESULT.set(0);
            set_errno(to_errno(source));
        }
        // The round trip failed, so there is no POSIX condition to report. The C driver answers
        // `EPIPE` for exactly this case and leaves the real reason in the result code.
        CommandError::SendRequest { source, .. } => {
            LAST_RESULT.set(source.to_rc());
            set_errno(EPIPE);
        }
        CommandError::ParseResponse { source, .. } => {
            LAST_RESULT.set(source.to_rc());
            set_errno(EPIPE);
        }
        // Nothing was sent and no service was involved, so there is no result code to record. The
        // set was larger than the command can count, which is what C calls an invalid argument.
        CommandError::UncountableSet { .. } => {
            LAST_RESULT.set(0);
            set_errno(EINVAL);
        }
    }

    -1
}

/// Reports a command that failed at a service other than the socket service, and returns C's
/// integer failure value.
///
/// The calls that hand a socket descriptor to the TLS stack or to a network-interface request are
/// dispatched by the layer holding those services, not by this crate, so their failure arrives as
/// a result code rather than a [`CommandError`] and there is no POSIX condition behind it. The C
/// driver answers `EIO` for every one of them and leaves the reason in the result code, which is
/// what this does.
pub fn report_result(code: ResultCode) -> c_int {
    LAST_RESULT.set(code);
    set_errno(EIO);
    -1
}

/// Reports `errno` on the calling thread and returns C's integer failure value.
///
/// For the failures this crate detects itself, before any command is sent: a descriptor that names
/// no socket, an argument that does not convert, a driver that was never initialized.
pub fn fail(errno: c_int) -> c_int {
    set_errno(errno);
    -1
}

/// Converts a condition the service reported into the number newlib gives it.
pub fn to_errno(err: PosixError) -> c_int {
    match err {
        PosixError::NotPermitted => EPERM,
        PosixError::NotFound => ENOENT,
        PosixError::Interrupted => EINTR,
        PosixError::Io => EIO,
        PosixError::BadFd => EBADF,
        PosixError::WouldBlock => EAGAIN,
        PosixError::OutOfMemory => ENOMEM,
        PosixError::PermissionDenied => EACCES,
        PosixError::BadAddress => EFAULT,
        PosixError::Busy => EBUSY,
        PosixError::AlreadyExists => EEXIST,
        PosixError::InvalidArgument => EINVAL,
        PosixError::SystemFdLimit => ENFILE,
        PosixError::ProcessFdLimit => EMFILE,
        PosixError::NotATty => ENOTTY,
        PosixError::StorageFull => ENOSPC,
        PosixError::NotSeekable => ESPIPE,
        PosixError::BrokenPipe => EPIPE,
        PosixError::NameTooLong => ENAMETOOLONG,
        PosixError::Unsupported => ENOSYS,
        PosixError::TooManySymbolicLinks => ELOOP,
        PosixError::BadMessage => EBADMSG,
        PosixError::Overflow => EOVERFLOW,
        PosixError::IllegalByteSequence => EILSEQ,
        PosixError::NotASocket => ENOTSOCK,
        PosixError::DestinationAddressRequired => EDESTADDRREQ,
        PosixError::MessageTooLong => EMSGSIZE,
        PosixError::WrongProtocolType => EPROTOTYPE,
        PosixError::ProtocolNotAvailable => ENOPROTOOPT,
        PosixError::ProtocolNotSupported => EPROTONOSUPPORT,
        PosixError::SocketTypeNotSupported => ESOCKTNOSUPPORT,
        PosixError::OperationNotSupported => EOPNOTSUPP,
        PosixError::ProtocolFamilyNotSupported => EPFNOSUPPORT,
        PosixError::AddressFamilyNotSupported => EAFNOSUPPORT,
        PosixError::AddressInUse => EADDRINUSE,
        PosixError::AddressNotAvailable => EADDRNOTAVAIL,
        PosixError::NetworkDown => ENETDOWN,
        PosixError::NetworkUnreachable => ENETUNREACH,
        PosixError::NetworkReset => ENETRESET,
        PosixError::ConnectionAborted => ECONNABORTED,
        PosixError::ConnectionReset => ECONNRESET,
        PosixError::NoBufferSpace => ENOBUFS,
        PosixError::AlreadyConnected => EISCONN,
        PosixError::NotConnected => ENOTCONN,
        PosixError::Shutdown => ESHUTDOWN,
        PosixError::TooManyReferences => ETOOMANYREFS,
        PosixError::TimedOut => ETIMEDOUT,
        PosixError::ConnectionRefused => ECONNREFUSED,
        PosixError::HostDown => EHOSTDOWN,
        PosixError::HostUnreachable => EHOSTUNREACH,
        PosixError::AlreadyInProgress => EALREADY,
        PosixError::InProgress => EINPROGRESS,
        PosixError::Canceled => ECANCELED,
        PosixError::OwnerDead => EOWNERDEAD,
        PosixError::NotRecoverable => ENOTRECOVERABLE,
        // No name for it, so no newlib number either. The offset is what the C driver produces
        // for a code its own table does not map, and it keeps the original visible.
        PosixError::Unknown(code) => UNKNOWN_ERROR_OFFSET + code,
    }
}

/// Added to a code with no newlib equivalent, so the original stays readable.
const UNKNOWN_ERROR_OFFSET: c_int = 10000;

/// Writes `code` into newlib's thread-local `errno` slot.
///
/// `__errno` is newlib's accessor for the calling thread's slot, and writing through it is how
/// every C library on this platform reports a POSIX error.
pub fn set_errno(code: c_int) {
    unsafe extern "C" {
        // newlib accessor for the calling thread's `errno` slot.
        fn __errno() -> *mut c_int;
    }

    // SAFETY: `__errno` always returns a valid, writable pointer to the calling thread's slot.
    unsafe { *__errno() = code };
}

/// Operation not permitted.
pub const EPERM: c_int = 1;
/// No such file or directory.
pub const ENOENT: c_int = 2;
/// Interrupted system call.
pub const EINTR: c_int = 4;
/// I/O error.
pub const EIO: c_int = 5;
/// Bad file descriptor.
pub const EBADF: c_int = 9;
/// Resource temporarily unavailable.
pub const EAGAIN: c_int = 11;
/// Cannot allocate memory.
pub const ENOMEM: c_int = 12;
/// Permission denied.
pub const EACCES: c_int = 13;
/// Bad address.
pub const EFAULT: c_int = 14;
/// Device or resource busy.
pub const EBUSY: c_int = 16;
/// File exists.
pub const EEXIST: c_int = 17;
/// Invalid argument.
pub const EINVAL: c_int = 22;
/// Too many open files in the system.
pub const ENFILE: c_int = 23;
/// Too many open files in this process.
pub const EMFILE: c_int = 24;
/// Not a terminal.
pub const ENOTTY: c_int = 25;
/// No space left on device.
pub const ENOSPC: c_int = 28;
/// Illegal seek.
pub const ESPIPE: c_int = 29;
/// Broken pipe.
pub const EPIPE: c_int = 32;
/// Protocol error.
pub const EPROTO: c_int = 71;
/// Bad message.
pub const EBADMSG: c_int = 77;
/// Function not implemented.
pub const ENOSYS: c_int = 88;
/// File or path name too long.
pub const ENAMETOOLONG: c_int = 91;
/// Too many symbolic links.
pub const ELOOP: c_int = 92;
/// Operation not supported on socket.
pub const EOPNOTSUPP: c_int = 95;
/// Protocol family not supported.
pub const EPFNOSUPPORT: c_int = 96;
/// Connection reset by peer.
pub const ECONNRESET: c_int = 104;
/// No buffer space available.
pub const ENOBUFS: c_int = 105;
/// Address family not supported by protocol family.
pub const EAFNOSUPPORT: c_int = 106;
/// Protocol wrong type for socket.
pub const EPROTOTYPE: c_int = 107;
/// Socket operation on non-socket.
pub const ENOTSOCK: c_int = 108;
/// Protocol not available.
pub const ENOPROTOOPT: c_int = 109;
/// Cannot send after socket shutdown.
pub const ESHUTDOWN: c_int = 110;
/// Connection refused.
pub const ECONNREFUSED: c_int = 111;
/// Address already in use.
pub const EADDRINUSE: c_int = 112;
/// Software caused connection abort.
pub const ECONNABORTED: c_int = 113;
/// Network is unreachable.
pub const ENETUNREACH: c_int = 114;
/// Network interface is not configured.
pub const ENETDOWN: c_int = 115;
/// Connection timed out.
pub const ETIMEDOUT: c_int = 116;
/// Host is down.
pub const EHOSTDOWN: c_int = 117;
/// Host is unreachable.
pub const EHOSTUNREACH: c_int = 118;
/// Connection already in progress.
pub const EINPROGRESS: c_int = 119;
/// Socket already connected.
pub const EALREADY: c_int = 120;
/// Destination address required.
pub const EDESTADDRREQ: c_int = 121;
/// Message too long.
pub const EMSGSIZE: c_int = 122;
/// Unknown protocol.
pub const EPROTONOSUPPORT: c_int = 123;
/// Socket type not supported.
pub const ESOCKTNOSUPPORT: c_int = 124;
/// Address not available.
pub const EADDRNOTAVAIL: c_int = 125;
/// Connection aborted by network.
pub const ENETRESET: c_int = 126;
/// Socket is already connected.
pub const EISCONN: c_int = 127;
/// Socket is not connected.
pub const ENOTCONN: c_int = 128;
/// Too many references: cannot splice.
pub const ETOOMANYREFS: c_int = 129;
/// Value too large for defined data type.
pub const EOVERFLOW: c_int = 139;
/// Illegal byte sequence.
pub const EILSEQ: c_int = 138;
/// Operation canceled.
pub const ECANCELED: c_int = 140;
/// State not recoverable.
pub const ENOTRECOVERABLE: c_int = 141;
/// Previous owner died.
pub const EOWNERDEAD: c_int = 142;
