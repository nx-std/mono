//! `struct timespec` and related definitions.

use core::time::Duration;

use super::nsec::{NSEC_PER_SEC, Nanoseconds};
use crate::sys::clock;

/// A structure representing a date and time.
///
/// The simplest data type used to represent simple calendar time.
///
/// On POSIX-conformant systems, `time_t` is an integer type and its values represent the number of
/// seconds elapsed since the epoch, which is 00:00:00 on January 1, 1970, UTC.
///
/// # References
///
/// - [GNU C Library: Time Types](https://www.gnu.org/software/libc/manual/html_node/Time-Types.html)
#[allow(non_camel_case_types)]
pub(crate) type c_time_t = i64;

/// A structure representing a time value.
///
/// Represents a simple calendar time, or an elapsed time, with sub-second resolution.
///
/// This struct is the equivalent to libc's `struct timespec`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Timespec {
    /// The number of whole seconds elapsed since the epoch (for a simple calendar time) or since
    /// some other starting point (for an elapsed time).
    tv_sec: c_time_t,
    /// The number of nanoseconds elapsed since the time given by the `tv_sec` member.
    tv_nsec: Nanoseconds,
}

impl Timespec {
    /// Create a new `Timespec` without checking the nanoseconds field.
    ///
    /// The caller must ensure `tv_nsec` is in `0..=999_999_999`; see
    /// [`Nanoseconds::from_u32_unchecked`] for what an out-of-range value costs.
    pub(crate) const fn new_unchecked(tv_sec: i64, tv_nsec: i64) -> Timespec {
        Timespec {
            tv_sec,
            // SAFETY: Delegated to this function's own precondition.
            tv_nsec: Nanoseconds::from_u32_unchecked(tv_nsec as u32),
        }
    }

    /// Create a new `Timespec` with 0 seconds and 0 nanoseconds.
    pub const fn zero() -> Timespec {
        // SAFETY: 0 is within the valid nanoseconds range.
        Self::new_unchecked(0, 0)
    }

    /// Get the current time for the specified clock.
    pub fn now(clock: ClockId) -> Timespec {
        match clock {
            ClockId::Realtime => {
                // Get the current time from the RTC service
                clock::service::gettime()
            }
            ClockId::Monotonic => {
                // Get the current time from the AArch64 CPU counter
                clock::aarch64::gettime()
            }
        }
        .unwrap_or(Timespec::zero())
    }

    /// Get the number of whole seconds.
    pub fn sec(&self) -> i64 {
        self.tv_sec
    }

    /// Get the number of nanoseconds.
    pub fn nsec(&self) -> i64 {
        self.tv_nsec.as_inner() as i64
    }

    pub fn sub_timespec(&self, other: &Timespec) -> Result<Duration, Duration> {
        if self >= other {
            // NOTE(eddyb) two aspects of this `if`-`else` are required for LLVM
            // to optimize it into a branchless form (see also #75545):
            //
            // 1. `self.tv_sec - other.tv_sec` shows up as a common expression
            //    in both branches, i.e. the `else` must have its `- 1`
            //    subtraction after the common one, not interleaved with it
            //    (it used to be `self.tv_sec - 1 - other.tv_sec`)
            //
            // 2. the `Duration::new` call (or any other additional complexity)
            //    is outside of the `if`-`else`, not duplicated in both branches
            //
            // Ideally this code could be rearranged such that it more
            // directly expresses the lower-cost behavior we want from it.
            let (secs, nsec) = if self.tv_nsec.as_inner() >= other.tv_nsec.as_inner() {
                (
                    (self.tv_sec - other.tv_sec) as u64,
                    self.tv_nsec.as_inner() - other.tv_nsec.as_inner(),
                )
            } else {
                (
                    (self.tv_sec - other.tv_sec - 1) as u64,
                    self.tv_nsec.as_inner() + (NSEC_PER_SEC as u32) - other.tv_nsec.as_inner(),
                )
            };

            Ok(Duration::new(secs, nsec))
        } else {
            match other.sub_timespec(self) {
                Ok(d) => Err(d),
                Err(d) => Ok(d),
            }
        }
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Timespec> {
        let mut secs = self.tv_sec.checked_add_unsigned(other.as_secs())?;

        // Nano calculations can't overflow because nanos are <1B which fit
        // in an u32.
        let mut nsec = other.subsec_nanos() + self.tv_nsec.as_inner();
        if nsec >= NSEC_PER_SEC as u32 {
            nsec -= NSEC_PER_SEC as u32;
            secs = secs.checked_add(1)?;
        }
        // SAFETY: The branch above normalises `nsec` into `0..NSEC_PER_SEC`.
        Some(Timespec::new_unchecked(secs, nsec.into()))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Timespec> {
        let mut secs = self.tv_sec.checked_sub_unsigned(other.as_secs())?;

        // Similar to above, nanos can't overflow.
        let mut nsec = self.tv_nsec.as_inner() as i32 - other.subsec_nanos() as i32;
        if nsec < 0 {
            nsec += NSEC_PER_SEC as i32;
            secs = secs.checked_sub(1)?;
        }
        // SAFETY: The branch above normalises `nsec` into `0..NSEC_PER_SEC`.
        Some(Timespec::new_unchecked(secs, nsec.into()))
    }
}

/// Identifies a specific system clock source.
///
/// There are two main types of clocks:
/// - System-wide clocks that are visible to all processes
/// - Per-process clocks that only measure time within a single process
///
/// The most basic clock is the realtime clock ([`ClockId::Realtime`]) which measures
/// wall-clock time as seconds and nanoseconds since the Unix epoch (January 1, 1970).
/// Changes to this clock (e.g. via NTP) will affect absolute timers but not relative ones.
///
/// The monotonic clock ([`ClockId::Monotonic`]) provides a steady time source that is
/// guaranteed to be strictly increasing and unaffected by system time changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ClockId {
    /// System-wide realtime clock.
    Realtime = 0,
    /// Clock that cannot be set and represents monotonic time since some unspecified
    /// starting point.
    Monotonic = 1,
}

impl TryFrom<i32> for ClockId {
    type Error = UnknownClockIdError;

    /// Decodes the `clockid_t` a C caller passed across the FFI boundary.
    ///
    /// # Errors
    ///
    /// Returns [`UnknownClockIdError`] if the value names no clock this platform
    /// implements.
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Realtime),
            1 => Ok(Self::Monotonic),
            unknown => Err(UnknownClockIdError(unknown)),
        }
    }
}

/// An error indicating that a `clockid_t` names no clock this platform implements.
#[derive(Debug, thiserror::Error)]
#[error("Unknown clock id {0}")]
pub struct UnknownClockIdError(pub i32);
