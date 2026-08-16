//! The socket descriptor a TLS connection is handed, and hands back.
//!
//! [`SocketFd`] maintains one invariant: it is non-negative, which is what separates a descriptor
//! from the value the service reserves for "there is none". [`SocketFd::from_raw`] is the only
//! place that is established, and [`SocketFd::from_raw_unchecked`] is the bypass for a caller that
//! already holds the proof.
//!
//! # Why the check is not a `TryFrom`
//!
//! The `rust-fn-unchecked` rule puts a newtype's validation in `TryFrom`, and this is a deliberate
//! departure from it, recorded here because it is not an oversight. A `TryFrom` needs an error
//! type, and the
//! only thing this check can conclude is that the service reported no descriptor: an error carrying
//! no information, which every call site immediately turned back into `None`. `Option` is what all
//! of them meant, and spelling it directly removes both the empty error type and the conversion out
//! of it. The rule's purpose, one validating home the unchecked constructor is named against, is
//! unaffected: [`SocketFd::from_raw`] is that home.

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
    /// Reads a descriptor the service reported, or `None` if it reported none.
    ///
    /// This is where the sentinel stops being a number and becomes an absence: a command that held
    /// no descriptor to report answers with a negative value, and that is the one thing about the
    /// number this crate can check on its own.
    ///
    /// `None` is not a failure at every call site. The commands that hand a socket to a connection
    /// answer with the one they displaced, and a connection that held none displaced none, which
    /// is an ordinary outcome. At the C boundary the same `None` is a caller's bad argument. Which
    /// of the two it is belongs to the caller, so this reports the absence and takes no view.
    #[inline]
    pub const fn from_raw(raw: i32) -> Option<Self> {
        if raw < 0 {
            return None;
        }
        Some(Self(raw))
    }

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
