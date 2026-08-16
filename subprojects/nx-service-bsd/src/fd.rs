//! BSD socket file descriptor newtype.
//!
//! [`SocketFd`] maintains one invariant: it always names a descriptor the BSD
//! service issued. Nothing in this module establishes it, because nothing here
//! can — whether a descriptor was issued is what the command's response says,
//! and only the caller that read that response knows. So validation happens in
//! [`crate::cmif`], where a negative return becomes an error before any
//! descriptor is built, and [`SocketFd::from_raw_unchecked`] is what the
//! commands use to record that they did the check.

/// A descriptor the BSD socket service issued.
///
/// Returned by [`BsdService::socket`](crate::BsdService::socket) and
/// [`BsdService::accept`](crate::BsdService::accept), and taken by every
/// command that names a socket.
///
/// **A value of this type always names a descriptor the service issued.** The
/// service reports "no descriptor" the POSIX way, as a negative return
/// alongside the condition that caused it, and this crate turns that into an
/// `Err` before a descriptor is ever built — so there is no sentinel here to
/// test against, and no caller has to. Holding the invariant in the type is
/// what removes the check every operation would otherwise have to repeat, and
/// the guessing about whether it was performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SocketFd(i32);

impl SocketFd {
    /// Returns the raw `i32` the service knows this descriptor by.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    /// Adopts a descriptor the BSD service issued.
    ///
    /// The caller must ensure `fd` came from a command that was accepted,
    /// which is what makes it non-negative; nothing here can establish that
    /// on its own, since only the response the caller read says whether the
    /// command succeeded. A debug build asserts the part that is checkable.
    ///
    /// Most callers are the command wrappers in this crate, which read the
    /// response themselves. The exception is the descriptor another service
    /// reports rather than issues: the TLS service answers the command that
    /// takes a socket over with the descriptor it displaced, and that number
    /// belongs to the BSD service's space like any other. A caller outside
    /// this crate adopting one carries the same obligation: it has read a
    /// response, and the sentinel the service uses for "no descriptor" is
    /// ruled out before it gets here.
    ///
    /// # Panics
    ///
    /// In debug builds, if `fd` is negative — the service returning a
    /// descriptor for a command it rejected, or a caller adopting one without
    /// checking the response first.
    #[inline]
    pub const fn from_raw_unchecked(fd: i32) -> Self {
        debug_assert!(fd >= 0, "adopted a negative BSD socket descriptor");
        Self(fd)
    }
}
