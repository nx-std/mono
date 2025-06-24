//! Thread sleeping and yielding
//!
//! This module is an idiomatic wrapper around the `nx_svc::thread` module.

use core::time::Duration;

use nx_svc::thread as svc;

/// Puts the current thread to sleep for a certain amount of time.
///
/// This function might sleep for slightly longer than the specified duration due to
/// scheduling specifics.
///
/// The maximum sleep duration is `i64::MAX` nanoseconds (about 292 years).
/// Durations longer than that will be capped.
pub fn sleep(duration: Duration) {
    // Duration::as_nanos() returns a u128, but nx_svc::thread::sleep expects a u64.
    // We cap the value at u64::MAX, which is more than enough for any practical sleep duration.
    // nx_svc::thread::sleep will then cap it again at i64::MAX.
    let nanos = duration.as_nanos().min(u64::MAX as u128) as u64;
    svc::sleep(nanos);
}

/// Yields execution to another thread on the same core.
///
/// This is an alias for `yield_no_migration`.
/// In most cases, this is the function you want to use.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_now() {
    svc::yield_no_migration();
}

/// Yields execution to another thread on the same core.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_no_migration() {
    svc::yield_no_migration();
}

/// Yields execution to another thread, allowing core migration.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_with_migration() {
    svc::yield_with_migration();
}

/// Yields execution to any other thread, forcing load-balancing.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_to_any_thread() {
    svc::yield_to_any_thread();
}
