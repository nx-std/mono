//! Thread management

use super::raw;

/// Puts the current thread to sleep for a certain amount of nanoseconds
///
/// Note: `svcSleepThread` takes an `i64`, but negative values are used for yielding,
/// which is a different concern. This function only handles sleeping and will cap
/// the input at `i64::MAX`.
pub fn sleep(nanos: u64) {
    let nanos = nanos.min(i64::MAX as u64) as i64;
    unsafe { raw::sleep_thread(nanos) }
}

/// Yields execution to another thread on the same core.
///
/// This function calls the `svcSleepThread` syscall with `raw::YieldType::NoMigration` (0),
/// signaling the kernel to yield to a different thread scheduled on the same CPU core.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
pub fn yield_no_migration() {
    unsafe { raw::sleep_thread(raw::YieldType::NoMigration as i64) }
}

/// Yields execution to another thread, allowing core migration.
///
/// This function calls the `svcSleepThread` syscall with `raw::YieldType::WithMigration` (-1),
/// signaling the kernel to yield to a different thread, which may be on another CPU core.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
pub fn yield_with_migration() {
    unsafe { raw::sleep_thread(raw::YieldType::WithMigration as i64) }
}

/// Yields execution to any other thread, forcing load-balancing.
///
/// This function calls the `svcSleepThread` syscall with `raw::YieldType::ToAnyThread` (-2),
/// signaling the kernel to yield and perform a forced load-balancing of threads across cores.
///
/// # See also
///
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
pub fn yield_to_any_thread() {
    unsafe { raw::sleep_thread(raw::YieldType::ToAnyThread as i64) }
}
