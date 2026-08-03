//! The timer frequency of the system counter-timer.
//!
//! The system counter-timer is a 64-bit register, `cntpct_el0`, that increments at a fixed rate.
//! The frequency is read from the `cntfrq_el0` system register.
//!
//! For the Nintendo Switch, the frequency of the system counter-timer is 19.2MHz.
//!
//! That frequency is stated once, as [`TIMER_FREQ`]. Everything else here is derived from it at
//! compile time: the reduced integer ratio the tick/nanosecond conversions multiply by, and the
//! resolution one tick represents. Writing any of those out by hand is how the four constants this
//! module used to carry drifted apart from each other.

use nx_cpu::counter::{self, Hz, Ticks};
use static_assertions::const_assert_eq;

use crate::sys::{nsec::NSEC_PER_SEC, timespec::Timespec};

/// System counter-timer frequency (19.2MHz).
// SAFETY: A frequency in hertz, which is the unit `Hz` names.
pub const TIMER_FREQ: Hz = Hz::new_unchecked(19_200_000);

/// The number of nanoseconds in a second, as a `u64`.
///
/// `NSEC_PER_SEC` is declared as an `i64` for the POSIX-shaped arithmetic in
/// [`Timespec`]; the conversions here are unsigned.
// `NSEC_PER_SEC` is a positive literal constant, so the cast is lossless.
const NSEC_PER_SEC_U64: u64 = NSEC_PER_SEC as u64;

/// The greatest common divisor of the tick rate and the nanosecond rate.
///
/// Dividing both by this reduces the tick/nanosecond ratio to the smallest integers that
/// represent it exactly, which is what keeps the conversions below from overflowing as early
/// as multiplying by the full frequency would.
const RATIO_GCD: u64 = gcd(TIMER_FREQ.to_raw(), NSEC_PER_SEC_U64);

/// Ticks side of the reduced tick/nanosecond ratio.
const TICKS_PER_RATIO: u64 = TIMER_FREQ.to_raw() / RATIO_GCD;

/// Nanoseconds side of the reduced tick/nanosecond ratio.
const NSEC_PER_RATIO: u64 = NSEC_PER_SEC_U64 / RATIO_GCD;

// Pins the reduction at 19.2MHz to the ratio the conversions were originally written with, so a
// change to `TIMER_FREQ` that silently alters them fails the build rather than the clock.
const_assert_eq!(TICKS_PER_RATIO, 12);
const_assert_eq!(NSEC_PER_RATIO, 625);

/// Clock resolution in nanoseconds (~52.083ns per tick, truncated to 52).
///
/// One tick is not a whole number of nanoseconds, so this is the floor. It is reported to
/// `clock_getres`, which has no way to express a fractional resolution.
#[cfg(feature = "ffi")]
pub const NSEC_PER_TICK: u64 = cpu_ticks_to_ns(Ticks::new_unchecked(1));

/// Computes the greatest common divisor of `a` and `b` by Euclid's algorithm.
const fn gcd(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

/// Gets the current system tick.
///
/// This function reads the `cntpct_el0` system register, which holds the current value of the
/// CPU counter-timer.
#[inline]
pub fn get_system_tick() -> Ticks {
    counter::ticks()
}

/// Gets the system counter-timer frequency.
///
/// This function reads the `cntfrq_el0` system register, which holds the
/// frequency of the system counter-timer.
///
/// The value is read from the hardware rather than assumed to be [`TIMER_FREQ`], which is what
/// the conversions in this module are compiled against.
#[cfg(feature = "ffi")]
#[inline]
pub fn get_system_tick_freq() -> Hz {
    counter::frequency()
}

/// Converts time from nanoseconds to CPU ticks.
///
/// ```
/// f(x) = (x * 19_200_000Hz) / 1_000_000_000ns = (x * 12) / 625
/// ```
#[cfg(feature = "ffi")]
#[inline]
pub const fn ns_to_cpu_ticks(ns: u64) -> Ticks {
    // SAFETY: Scaling a nanosecond count by the tick/nanosecond ratio yields a tick count.
    Ticks::new_unchecked((ns * TICKS_PER_RATIO) / NSEC_PER_RATIO)
}

/// Converts from CPU ticks to nanoseconds.
///
/// ```
/// f(x) = (x * 1_000_000_000ns) / 19_200_000Hz = (x * 625) / 12
/// ```
#[inline]
pub const fn cpu_ticks_to_ns(ticks: Ticks) -> u64 {
    (ticks.to_raw() * NSEC_PER_RATIO) / TICKS_PER_RATIO
}

/// Get system clock time.
///
/// Get a monotonic time value from the system counter-timer.
///
/// # References
///
/// - [switchbrew/nx: `__syscall_clock_gettime`](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/source/runtime/newlib.c#L361-L386)
pub fn gettime() -> Result<Timespec, i32> {
    // Get current tick count relative to boot
    let now = get_system_tick().to_raw();

    // Convert to seconds and nanoseconds
    let seconds = now / TIMER_FREQ.to_raw();
    // SAFETY: A remainder of a tick count is itself a tick count.
    let subsec_ticks = Ticks::new_unchecked(now % TIMER_FREQ.to_raw());
    let nanoseconds = cpu_ticks_to_ns(subsec_ticks);

    // Create timespec with monotonic time (time since boot)
    // SAFETY: `subsec_ticks` is strictly less than one second's worth of ticks, so the
    // nanoseconds it converts to are within `0..NSEC_PER_SEC`.
    Ok(Timespec::new_unchecked(seconds as i64, nanoseconds as i64))
}
