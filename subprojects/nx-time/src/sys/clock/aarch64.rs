//! The timer frequency of the system counter-timer.
//!
//! The system counter-timer is a 64-bit register, `cntptc_el0`, that increments at a fixed rate.
//! The frequency is read from the `cntfrq_el0` system register.
//!
//! For the Nintendo Switch, the frequency of the system counter-timer is 19.2MHz.

use nx_cpu::control_regs;

use crate::sys::timespec::Timespec;

/// System counter-timer frequency (19.2MHz)
pub const TIMER_FREQ: u64 = 19_200_000; // Hz

/// Clock resolution in nanoseconds (~52.083ns per tick)
pub const NSEC_PER_TICK: u64 = 1_000_000_000 / 19_200_000; // ns

/// Gets the current system tick.
///
/// This function reads the `cntpct_el0` system register, which holds the current value of the
/// CPU counter-timer.
#[inline]
pub fn get_system_tick() -> u64 {
    unsafe { control_regs::cntpct_el0() }
}

/// Gets the system counter-timer frequency.
///
/// This function reads the `cntfrq_el0` system register, which holds the
/// frequency of the system counter-timer.
///
/// Returns the system counter-timer frequency, in Hz.
#[inline]
pub fn get_system_tick_freq() -> u64 {
    unsafe { control_regs::cntfrq_el0() }
}

/// Converts time from nanoseconds to CPU ticks.
///
/// ```
/// f(x) = (x * 19_200_000Hz) / 1_000_000_000ns = (x * 12) / 625
/// ```
///
/// Returns the equivalent CPU ticks for a given time in nanoseconds, based on the
/// system counter frequency.
#[inline]
pub const fn ns_to_cpu_ticks(ns: u64) -> u64 {
    (ns * 12) / 625
}

/// Converts from CPU ticks to nanoseconds.
///
/// ```
/// f(x) = (x * 1_000_000_000ns) / 19_200_000Hz = (x * 625) / 12
/// ```
///
/// Returns the equivalent time in nanoseconds for a given number of CPU ticks.
#[inline]
pub const fn cpu_ticks_to_ns(tick: u64) -> u64 {
    (tick * 625) / 12
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
    let now = get_system_tick();

    // Convert to seconds and nanoseconds
    let seconds = now / TIMER_FREQ;
    let subsec_ticks = now % TIMER_FREQ;
    let nanoseconds = cpu_ticks_to_ns(subsec_ticks);

    // Create timespec with monotonic time (time since boot)
    // SAFETY: We know our calculations produce valid ranges for timespec
    unsafe { Ok(Timespec::new_unchecked(seconds as i64, nanoseconds as i64)) }
}
