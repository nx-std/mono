//! Reporting failures to the C standard library.
//!
//! C entry points report failure by returning a sentinel and leaving the reason in an error number
//! held by the calling thread's reentrancy structure. That convention exists only at this boundary:
//! the crate's own interfaces report failure through their return types, so translating a `Result`
//! into a number is the last thing that happens on the way out.
//!
//! The error number is the first field of the reentrancy structure. The C declaration says so
//! explicitly and calls it out as a binary-compatibility guarantee rather than an accident of
//! ordering, which is what makes it safe to write at a known offset without mirroring the rest of a
//! large and configuration dependent structure.
//!
//! # References
//!
//! - newlib/libc/include/sys/reent.h
//! - newlib/libc/include/sys/errno.h

use core::ffi::c_int;

use super::{
    ctypes::SsizeT,
    reent::Reent,
};
use crate::{
    device::DeviceError,
    table::{
        AttachError,
        CloseError,
        DuplicateError,
        DuplicateToError,
        MetadataError,
        OpenError,
        ReadError,
        SeekError,
        SetLenError,
        SyncError,
        WriteError,
    },
};

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

/// Permission denied.
pub const EACCES: c_int = 13;

/// File exists.
pub const EEXIST: c_int = 17;

/// No such device.
pub const ENODEV: c_int = 19;

/// Invalid argument.
pub const EINVAL: c_int = 22;

/// Too many open files in the system.
pub const ENFILE: c_int = 23;

/// Illegal seek.
pub const ESPIPE: c_int = 29;

/// Function not implemented.
pub const ENOSYS: c_int = 88;

/// Connection reset by peer.
pub const ECONNRESET: c_int = 104;

/// Connection timed out.
pub const ETIMEDOUT: c_int = 116;

/// Socket is not connected.
pub const ENOTCONN: c_int = 128;

/// Reports `errno` through `r` and returns C's integer failure value.
pub fn fail(r: *mut Reent, errno: c_int) -> c_int {
    // SAFETY: the caller passes a live or null reentrancy pointer.
    unsafe { set_errno(r, errno) };
    -1
}

/// Reports `errno` through `r` and returns C's byte-count failure value.
pub fn fail_ssize(r: *mut Reent, errno: c_int) -> SsizeT {
    // SAFETY: the caller passes a live or null reentrancy pointer.
    unsafe { set_errno(r, errno) };
    -1
}

/// Converts a failure into the error number the C standard library reports for it.
///
/// The C convention is that a call returns a sentinel and leaves the reason in an error number, so
/// every failure crossing this boundary is translated exactly once, here. Nothing inside the crate
/// deals in error numbers: a caller on the Rust side matches on the error itself.
///
/// The trait is sealed. An error declared outside this crate is not a descriptor failure, so giving
/// it an error number here would put a value a C caller decodes as an `errno` where none was ever
/// produced. A crate layered on top of this one declares its own trait instead.
pub trait ToErrno: core::error::Error + _sealed::Sealed {
    /// Converts the failure into an error number.
    fn to_errno(self) -> c_int;
}

impl ToErrno for OpenError {
    fn to_errno(self) -> c_int {
        match self {
            Self::NoDevice => ENODEV,
            Self::NoDescriptors => ENFILE,
        }
    }
}

impl _sealed::Sealed for OpenError {}

impl ToErrno for AttachError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor | Self::AlreadyAttached => EBADF,
        }
    }
}

impl _sealed::Sealed for AttachError {}

impl ToErrno for CloseError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor => EBADF,
            Self::File(err) => err.to_errno(),
        }
    }
}

impl _sealed::Sealed for CloseError {}

impl ToErrno for DuplicateError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor => EBADF,
            Self::NoDescriptors => ENFILE,
        }
    }
}

impl _sealed::Sealed for DuplicateError {}

impl ToErrno for DuplicateToError {
    fn to_errno(self) -> c_int {
        EBADF
    }
}

impl _sealed::Sealed for DuplicateToError {}

impl ToErrno for WriteError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor => EBADF,
            Self::NoDevice => ENODEV,
            Self::Device(err) => err.to_errno(),
        }
    }
}

impl _sealed::Sealed for WriteError {}

impl ToErrno for ReadError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor => EBADF,
            Self::NoDevice => ENODEV,
            Self::Device(err) => err.to_errno(),
        }
    }
}

impl _sealed::Sealed for ReadError {}

impl ToErrno for SeekError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor => EBADF,
            // A stream has no position, which C reports as the descriptor being the wrong kind
            // rather than as an unimplemented operation.
            Self::NotAFile => ESPIPE,
            Self::File(err) => err.to_errno(),
        }
    }
}

impl _sealed::Sealed for SeekError {}

impl ToErrno for MetadataError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor | Self::NotAFile => EBADF,
            Self::File(err) => err.to_errno(),
        }
    }
}

impl _sealed::Sealed for MetadataError {}

impl ToErrno for SetLenError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor | Self::NotAFile => EBADF,
            Self::File(err) => err.to_errno(),
        }
    }
}

impl _sealed::Sealed for SetLenError {}

impl ToErrno for SyncError {
    fn to_errno(self) -> c_int {
        match self {
            Self::BadDescriptor | Self::NotAFile => EBADF,
            Self::File(err) => err.to_errno(),
        }
    }
}

impl _sealed::Sealed for SyncError {}

impl ToErrno for DeviceError {
    fn to_errno(self) -> c_int {
        match self {
            Self::Unsupported => ENOSYS,
            Self::Io => EIO,
            Self::NotFound => ENOENT,
            Self::AlreadyExists => EEXIST,
            Self::InvalidPath => EINVAL,
            Self::WouldBlock => EAGAIN,
            Self::Interrupted => EINTR,
            Self::ConnectionReset => ECONNRESET,
            Self::NotConnected => ENOTCONN,
            Self::PermissionDenied => EACCES,
            Self::TimedOut => ETIMEDOUT,
        }
    }
}

impl _sealed::Sealed for DeviceError {}

/// Writes `errno` into the reentrancy structure, reporting failure to the caller.
///
/// Does nothing when `r` is null: the C entry points pass a null reentrancy pointer in paths that
/// discard errors, and faulting there would turn a reported failure into a crash.
///
/// # Safety
///
/// `r` must be null or point to a live `struct _reent` owned by the calling thread.
pub unsafe fn set_errno(r: *mut Reent, errno: c_int) {
    if r.is_null() {
        return;
    }
    // SAFETY: `r` is non-null and the caller guarantees it points to a live `struct _reent`, whose
    // first field is this error number.
    unsafe { (*r).set_errno(errno) };
}

/// Reports `errno` on the calling thread.
///
/// The entry points whose C prototypes take no reentrancy pointer still have to report failure, so
/// they reach the calling thread's structure through its thread variables, which is where the
/// thread runtime records it.
///
/// Does nothing when the calling thread has no initialized thread variables or no reentrancy
/// structure yet, which happens only before the thread runtime has set the thread up, when there is
/// no reader for the error either.
pub fn set_thread_errno(errno: c_int) {
    // SAFETY: the pointer is this thread's own reentrancy structure, or null.
    unsafe { set_errno(thread_reent(), errno) };
}

/// Returns the calling thread's reentrancy structure, or null when it has none.
///
/// The entry points whose C prototypes take no reentrancy pointer have one more reason to want it
/// than reporting failure: an operation they hand on to a device expects to be passed one, and the
/// calling thread's is the one that operation would have been given had it been reached the usual
/// way.
pub fn thread_reent() -> *mut Reent {
    let vars = nx_sys_thread_tls::thread_vars_ptr();
    if vars.is_null() {
        return core::ptr::null_mut();
    }

    // SAFETY: `thread_vars_ptr` returns a pointer into the calling thread's own thread-local
    // region, which is live for as long as the thread is.
    let vars = unsafe { &*vars };
    if vars.magic != nx_sys_thread_tls::THREAD_VARS_MAGIC {
        return core::ptr::null_mut();
    }

    // The magic value says the thread runtime initialized these variables, so `reent` either is
    // null or points to this thread's reentrancy structure.
    vars.reent.to_raw().cast::<Reent>()
}

pub(crate) mod _sealed {
    /// Restricts [`ToErrno`](super::ToErrno) to this crate's error types. Implemented immediately
    /// after every `ToErrno` impl.
    pub trait Sealed {}
}
