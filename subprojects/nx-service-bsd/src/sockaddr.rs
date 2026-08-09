//! The socket address as the BSD service exchanges it.
//!
//! [`RawSockAddr`] maintains one invariant: its length never exceeds its own
//! capacity, so the bytes it reports are always bytes it holds. Both
//! constructors establish it — the crate-private one the commands use by
//! clamping the length the service reported, and the [`TryFrom`] impl by
//! rejecting an over-long input.
//!
//! # Why the address is owned rather than borrowed
//!
//! The C interface fills a caller's buffer and reports how long the address
//! really was — separately, as a `socklen_t`. Two things go wrong with that
//! pair once it escapes the call that produced it. The length can exceed the
//! buffer, because that is precisely how the interface signals the address was
//! truncated, so a caller that reads it as "bytes written" and slices with it
//! is indexing out of bounds. And nothing binds the length to the buffer it
//! describes, so the two can be separated and re-paired with something else.
//!
//! Neither is a risk worth managing, because the buffer never had to be the
//! caller's. Every address the service can return fits `sockaddr_storage`,
//! whose whole purpose is to be large enough for any address in any family the
//! interface supports. Owning a buffer of that size makes truncation
//! unrepresentable rather than merely detectable, and leaves nothing for a
//! caller to size wrongly.
//!
//! This is the shape `std` uses. Its Unix socket layer keeps the storage and
//! the `socklen_t` inside one function body and hands out an owned
//! `SocketAddr`; the pair never crosses a boundary.
//!
//! # What this type deliberately does not do
//!
//! It does not interpret the bytes. Which family they belong to, and therefore
//! how they decode into an address, is the socket layer's business — the same
//! place `std` puts it, in the function that turns a `sockaddr_storage` into a
//! `SocketAddr`. This crate speaks to the service; it does not know what an IP
//! address is.

/// A socket address, exactly as the BSD service exchanges it.
///
/// Carries its own length, so there is no second value to keep alongside it
/// and no way to describe more bytes than it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawSockAddr {
    /// Sized by `sockaddr_storage`. Bytes past `len` are zero and carry no
    /// meaning.
    bytes: [u8; Self::CAPACITY],
    /// Always `<= CAPACITY`; the two constructors are the only writers.
    len: u16,
}

impl RawSockAddr {
    /// Capacity in bytes, matching `sockaddr_storage`.
    ///
    /// The BSD interface defines this as large enough for an address in any
    /// family it supports, which is what lets this type own its buffer instead
    /// of borrowing one.
    pub const CAPACITY: usize = 128;

    /// The address bytes, and nothing past them.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    /// Whether the service reported no address at all.
    ///
    /// A command can succeed and still report nothing — `accept` on a socket
    /// whose family carries no peer address, for one — so this is an ordinary
    /// outcome rather than a failure.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Builds an address from a response buffer and the length the service
    /// reported for it.
    ///
    /// `reported` is clamped to what `buf` holds. A service that reports more
    /// than it was given room for is describing an address that did not fit,
    /// and the bytes past the buffer do not exist to be read; clamping keeps
    /// the invariant without inventing an error the caller could not act on.
    pub(crate) fn from_response(buf: &[u8; Self::CAPACITY], reported: u32) -> Self {
        // `CAPACITY` is far below `u16::MAX`, so the narrowing is exact.
        let len = core::cmp::min(reported as usize, Self::CAPACITY) as u16;
        Self { bytes: *buf, len }
    }
}

impl TryFrom<&[u8]> for RawSockAddr {
    type Error = AddrTooLongError;

    /// Adopts caller-supplied address bytes.
    ///
    /// # Errors
    ///
    /// [`AddrTooLongError`] if `bytes` is longer than [`RawSockAddr::CAPACITY`],
    /// which no address in a family the service supports can be.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() > Self::CAPACITY {
            return Err(AddrTooLongError { len: bytes.len() });
        }

        let mut storage = [0u8; Self::CAPACITY];
        storage[..bytes.len()].copy_from_slice(bytes);
        // Bounded by the check above, so the narrowing is exact.
        let len = bytes.len() as u16;
        Ok(Self {
            bytes: storage,
            len,
        })
    }
}

/// Error returned when adopting address bytes that cannot fit a
/// [`RawSockAddr`].
///
/// Occurs only for input longer than [`RawSockAddr::CAPACITY`]. Detected
/// before anything is sent, so no command was issued.
#[derive(Debug, thiserror::Error)]
#[error(
    "socket address of {len} bytes exceeds the {} the service exchanges",
    RawSockAddr::CAPACITY
)]
pub struct AddrTooLongError {
    /// The length that was offered.
    pub len: usize,
}
