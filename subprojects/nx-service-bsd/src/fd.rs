//! BSD socket file descriptor newtype.

/// File descriptor returned by [`BsdService::socket`](crate::BsdService::socket)
/// and consumed by every other socket operation.
///
/// Wraps the `i32` returned by the BSD service. A value of `-1`
/// ([`BsdSockFd::INVALID`]) signifies "no socket / closed", matching POSIX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BsdSockFd(i32);

impl BsdSockFd {
    /// Sentinel for "no file descriptor". Equal to `-1`, matching POSIX.
    pub const INVALID: Self = Self(-1);

    /// Returns the raw `i32` file descriptor value.
    #[inline]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Returns whether this descriptor is the [`INVALID`](Self::INVALID) sentinel.
    #[inline]
    pub const fn is_invalid(self) -> bool {
        self.0 < 0
    }

    /// Constructs a [`BsdSockFd`] from a raw `i32` returned by the BSD service.
    ///
    /// Crate-private — external callers obtain descriptors via
    /// [`BsdService::socket`](crate::BsdService::socket) and
    /// [`BsdService::accept`](crate::BsdService::accept) only.
    #[inline]
    pub(crate) const fn from_raw(fd: i32) -> Self {
        Self(fd)
    }
}
