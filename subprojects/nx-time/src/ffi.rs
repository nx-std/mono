//! FFI bindings for the `nx-time` crate
//!
//! # References
//!
//! - [switchbrew/libnx: switch/arm/counter.h](https://github.com/switchbrew/libnx/blob/60bf943ec14b1fb2ae169e627e64ab93a24c042b/nx/include/switch/arm/counter.h)

mod libsysbase;

use nx_cpu::counter::Ticks;

use crate::sys::clock;

//<editor-fold desc="switch/arm/counter.h">

/// Gets the current system tick.
///
/// This function reads the `cntpct_el0` system register, which holds the current value of the
/// CPU counter-timer.
///
/// Returns the current system tick as a `u64`.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__arm_get_system_tick() -> u64 {
    clock::aarch64::get_system_tick().to_raw()
}

/// Gets the system counter-timer frequency.
///
/// This function reads the `cntfrq_el0` system register, which holds the
/// frequency of the system counter-timer.
///
/// Returns the system counter-timer frequency, in Hz.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__arm_get_system_tick_freq() -> u64 {
    clock::aarch64::get_system_tick_freq().to_raw()
}

/// Converts time from nanoseconds to CPU ticks.
///
/// Returns the equivalent CPU ticks for a given time in nanoseconds, based on the
/// system counter frequency.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__arm_ns_to_ticks(ns: u64) -> u64 {
    clock::aarch64::ns_to_cpu_ticks(ns).to_raw()
}

/// Converts from CPU ticks to nanoseconds.
///
/// Returns the equivalent time in nanoseconds for a given number of CPU ticks.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__arm_ticks_to_ns(tick: u64) -> u64 {
    // SAFETY: The C caller's contract for `armTicksToNs` is that this argument is a
    // counter-timer reading; nothing at this boundary can check that.
    clock::aarch64::cpu_ticks_to_ns(Ticks::from_u64_unchecked(tick))
}

//</editor-fold>
