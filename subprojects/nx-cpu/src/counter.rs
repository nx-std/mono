//! The architectural counter-timer, read as typed values.
//!
//! The counter-timer is a free-running 64-bit counter that increments at a
//! fixed rate. Two registers describe it, and both hold a bare 64-bit word:
//! `cntpct_el0` holds the current count, `cntfrq_el0` holds the rate that count
//! advances at. A reading of one is never a reading of the other, but as `u64`
//! they are the same value, so passing a count where a rate is expected
//! compiles and produces a number that is wrong by a factor of the clock
//! frequency.
//!
//! [`Ticks`] and [`Hz`] separate the two. Neither constrains which 64-bit words
//! are valid: every count is a count and the register reports whatever rate the
//! system programmed. What they buy is that the two cannot be substituted for
//! one another, which the arithmetic that converts between counts and wall-clock
//! time depends on being told apart.

use crate::control_regs;

/// A reading of the counter-timer, in counter ticks.
///
/// Ticks are meaningful only against the counter's frequency ([`Hz`]): the
/// number alone says nothing about elapsed time until it is divided by the rate
/// the counter advances at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Ticks(u64);

impl Ticks {
    /// Wraps a raw tick count without checking where it came from.
    ///
    /// The caller must ensure `raw` is a counter-timer reading or a duration
    /// already expressed in ticks, rather than a value in some other unit. No
    /// validation is possible: every 64-bit word is a well-formed count, so a
    /// value in nanoseconds or hertz is accepted here and only goes wrong later,
    /// in whatever arithmetic treats it as a count.
    #[inline]
    pub const fn from_raw_unchecked(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw tick count.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// A frequency, in hertz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Hz(u64);

impl Hz {
    /// Wraps a raw frequency without checking where it came from.
    ///
    /// The caller must ensure `raw` is a count of cycles per second rather than
    /// a value in some other unit. No validation is possible: every 64-bit word
    /// is a well-formed frequency, so a tick count is accepted here and only
    /// goes wrong later, in whatever arithmetic treats it as a rate.
    #[inline]
    pub const fn from_raw_unchecked(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw frequency, in hertz.
    #[inline]
    pub const fn to_raw(self) -> u64 {
        self.0
    }
}

/// Reads the current value of the counter-timer.
///
/// The counter runs from system start and does not reset, so successive
/// readings are monotonically non-decreasing.
#[inline]
pub fn ticks() -> Ticks {
    // SAFETY: Reading `cntpct_el0` is a register move with no precondition; the
    // function is `unsafe` only because it is `naked`.
    let raw = unsafe { control_regs::cntpct_el0() };
    // SAFETY: `raw` is the counter-timer count register's own value, so it is a
    // tick count by construction.
    Ticks::from_raw_unchecked(raw)
}

/// Reads the rate the counter-timer advances at.
///
/// The value is programmed during system initialization and is constant for the
/// lifetime of the process; the hardware does not interpret it.
#[inline]
pub fn frequency() -> Hz {
    // SAFETY: Reading `cntfrq_el0` is a register move with no precondition; the
    // function is `unsafe` only because it is `naked`.
    let raw = unsafe { control_regs::cntfrq_el0() };
    // SAFETY: `raw` is the counter-timer frequency register's own value, so it
    // is a rate in hertz by construction.
    Hz::from_raw_unchecked(raw)
}
