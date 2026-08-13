//! How many waiting threads a wake releases.
//!
//! The count reaches the kernel as a plain integer carrying a sentinel: zero or less means "every
//! waiter". A sentinel is only a convention, so nothing stops a subtraction from reaching zero and
//! waking every thread when the caller meant to wake none.
//!
//! [`WakeCount`] names the two cases instead, and converts to the sentinel form at the point the
//! SVC is issued.
//!
//! A wait's deadline is the other quantity of this shape, and it is an `Option<Duration>` rather
//! than a type of its own; [`nx_svc::timeout`] holds the sentinel it encodes to.

use core::num::NonZeroU32;

/// How many waiting threads a wake releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeCount {
    /// Release every waiting thread.
    All,
    /// Release at most this many threads.
    ///
    /// The count is non-zero because the SVC reads zero as [`All`](Self::All); a request to wake
    /// nobody cannot be expressed, and writing it as `AtMost(0)` would wake everybody.
    AtMost(NonZeroU32),
}

impl WakeCount {
    /// The value the signal SVC reads as "every waiter".
    ///
    /// Any non-positive value means the same; this is the one libnx writes.
    const ALL_RAW: i32 = -1;

    /// Returns the count in the form the signal SVC takes.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        match self {
            Self::All => Self::ALL_RAW,
            // Saturating rather than wrapping matters here: a count above `i32::MAX` wraps to a
            // negative number, which the SVC reads as `All`, turning "wake a few" into "wake
            // everyone". No caller has that many waiters, so the clamp is unreachable in
            // practice and merely keeps the failure mode monotonic.
            Self::AtMost(count) if count.get() > i32::MAX as u32 => i32::MAX,
            Self::AtMost(count) => count.get() as i32,
        }
    }
}

impl From<i32> for WakeCount {
    /// Decodes the wake count a C caller passed across the FFI boundary.
    #[inline]
    fn from(raw: i32) -> Self {
        match u32::try_from(raw).ok().and_then(NonZeroU32::new) {
            Some(count) => Self::AtMost(count),
            // Zero and every negative value denote "all" to the SVC; `try_from` rejects the
            // negatives and `NonZeroU32` rejects the zero.
            None => Self::All,
        }
    }
}
