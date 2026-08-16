//! The socket descriptor a TLS connection is handed, and hands back.

/// A socket descriptor exchanged with the SSL service.
///
/// **A value of this type always names a socket.** The service reports "no descriptor" as a
/// negative number, and this crate turns that into `None` before a descriptor is ever built, so
/// there is no sentinel here to test against and no caller has to.
///
/// # It is not this crate's number
///
/// The SSL service takes a socket over and hands one back, but it does not issue either: the
/// descriptors belong to the socket service's space, and this crate does not speak to that
/// service or know which numbers are live in it. So this type carries only what a caller
/// asserted, which is what [`Self::from_raw_unchecked`] says.
///
/// The layer that holds both: the one that resolved the caller's descriptor against the socket
/// driver: is where the two spaces are known to be the same, and that is where the conversion
/// belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(transparent)]
pub struct SocketFd(i32);

impl SocketFd {
    /// Names a socket for a command that hands it to, or takes it from, a TLS connection.
    ///
    /// The caller must ensure `raw` is a descriptor the socket service issued and has not since
    /// closed, and that it is non-negative: the value the service reserves for "no descriptor".
    /// Nothing here can establish either: this crate never sees the socket service, and only that
    /// service knows which of its numbers are live. A descriptor that names nothing is answered
    /// with an error by the command it reaches rather than faulting, which is why this is a safe
    /// function.
    ///
    /// # Panics
    ///
    /// In debug builds, if `raw` is negative.
    #[inline]
    pub const fn from_raw_unchecked(raw: i32) -> Self {
        debug_assert!(
            raw >= 0,
            "socket descriptor is the service's `no descriptor` sentinel"
        );
        Self(raw)
    }

    /// Returns the raw `i32` the services know this descriptor by.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

impl From<nx_service_bsd::SocketFd> for SocketFd {
    /// Names a socket the socket service issued as the descriptor this service exchanges.
    ///
    /// Infallible, and no assertion is made: the two types carry the same invariant. A
    /// [`nx_service_bsd::SocketFd`] already names a descriptor the socket service issued, which is
    /// exactly what [`SocketFd::from_raw_unchecked`] asks a caller to vouch for, so the proof
    /// arrives with the value rather than being supplied at the call.
    fn from(fd: nx_service_bsd::SocketFd) -> Self {
        // SAFETY: `nx_service_bsd::SocketFd`'s own invariant is that it names a descriptor the
        // socket service issued, and it is non-negative because that crate rejects the service's
        // failure return before ever building one. Both halves of this constructor's precondition
        // therefore hold by the argument's type.
        Self::from_raw_unchecked(fd.to_raw())
    }
}

impl TryFrom<i32> for SocketFd {
    type Error = NoDescriptor;

    /// Reads a descriptor the service reported.
    ///
    /// This is where the sentinel stops being a number and becomes an absence: a command that
    /// held no descriptor to report answers with a negative value, and that is the one thing
    /// about the number this crate can check on its own.
    fn try_from(raw: i32) -> Result<Self, Self::Error> {
        if raw < 0 {
            return Err(NoDescriptor);
        }
        Ok(Self(raw))
    }
}

/// Error returned when a reported value names no socket.
#[derive(Debug, thiserror::Error)]
#[error("the service reported no socket descriptor")]
pub struct NoDescriptor;
