//! Thread sleeping and yielding functionality.
//!
//! This module provides safe, idiomatic wrappers around the thread sleep and
//! yield SVCs on Horizon OS. These functions operate on the current thread and
//! are compatible with the Arc+Pin thread handle pattern used throughout the
//! nx-sys-thread crate.
//!
//! ## Sleep vs Yield
//!
//! - **Sleep**: Suspends the current thread for a specified duration
//! - **Yield**: Voluntarily gives up CPU time to other threads
//!
//! ## Yield Variants
//!
//! The module provides four yield functions with different scheduling behaviors:
//! - [`yield_now`] - Standard yield, no core migration (most common)
//! - [`yield_no_migration`] - Explicit no-migration yield
//! - [`yield_with_migration`] - Allows scheduler to move thread to another core
//! - [`yield_to_any_thread`] - Forces load balancing across all cores
//!
//! ## Arc+Pin Pattern Compatibility
//!
//! These functions work seamlessly with threads created using the Arc+Pin pattern
//! from [`crate::thread_create`]. They operate on the calling thread directly
//! via kernel SVCs, so no thread handle is needed.

use core::time::Duration;

use nx_svc::thread as svc;

/// Puts the current thread to sleep for a certain amount of time.
///
/// This function suspends the calling thread for at least the specified
/// duration. The actual sleep time may be slightly longer due to scheduling
/// granularity and system load.
///
/// ## Duration Limits
///
/// - **Maximum**: `i64::MAX` nanoseconds (approximately 292 years)
/// - **Precision**: System timer resolution (typically microseconds)
/// - Durations exceeding the maximum are automatically capped
///
/// ## Scheduling Behavior
///
/// When a thread sleeps:
/// 1. The thread is removed from the scheduler's ready queue
/// 2. A kernel timer is set for the specified duration
/// 3. Other threads become eligible for the released CPU time
/// 4. After the timer expires, the thread returns to the ready queue
/// 5. The thread resumes when scheduled (may have additional delay)
///
/// ## Use Cases
///
/// - **Periodic Tasks**: Implement fixed-interval operations
/// - **Rate Limiting**: Control execution frequency
/// - **Polling**: Add delays between status checks
/// - **Synchronization**: Simple timing-based coordination
/// - **Power Saving**: Reduce CPU usage during idle periods
///
/// ## Performance Considerations
///
/// - Short sleeps (< 1ms) may have poor accuracy due to scheduling overhead
/// - For precise timing, consider using hardware timers directly
/// - Sleeping threads don't consume CPU but still use memory resources
/// - Frequent short sleeps can cause context switching overhead
pub fn sleep(duration: Duration) {
    // Duration::as_nanos() returns a u128, but nx_svc::thread::sleep expects a u64.
    // We cap the value at u64::MAX, which is more than enough for any practical sleep duration.
    // nx_svc::thread::sleep will then cap it again at i64::MAX.
    let nanos = duration.as_nanos().min(u64::MAX as u128) as u64;
    svc::sleep(nanos);
}

/// Yields execution to another thread on the same core.
///
/// This is the standard yield function and an alias for [`yield_no_migration`].
/// In most cases, this is the function you want to use for cooperative
/// multitasking.
///
/// ## Behavior
///
/// - Voluntarily gives up remaining time slice
/// - Thread stays on the same CPU core
/// - Thread remains in ready queue at same priority
/// - Scheduler selects next eligible thread
///
/// ## Use Cases
///
/// - **Cooperative Multitasking**: Allow other threads to run
/// - **Spinlock Backoff**: Reduce contention in busy-wait loops
/// - **Fair Sharing**: Prevent monopolizing CPU time
/// - **Latency Reduction**: Improve responsiveness of other threads
///
/// # See also
///
/// * [`yield_with_migration`] - Allows core migration
/// * [`yield_to_any_thread`] - Forces load balancing
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_now() {
    svc::yield_no_migration();
}

/// Yields execution to another thread on the same core.
///
/// This function explicitly prevents the scheduler from migrating the thread
/// to a different CPU core. It's functionally identical to [`yield_now`].
///
/// ## When to Use
///
/// Use this when you need to:
/// - Maintain CPU cache locality
/// - Avoid migration overhead
/// - Keep thread-to-core affinity
/// - Implement core-local algorithms
///
/// ## Performance Notes
///
/// - No core migration overhead
/// - Preserves CPU cache state
/// - Minimizes memory access latency
/// - Best for threads with core-specific resources
///
/// # See also
///
/// * [`yield_with_migration`] - Allows scheduler to migrate thread
/// * [`yield_to_any_thread`] - Forces system-wide load balancing
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_no_migration() {
    svc::yield_no_migration();
}

/// Yields execution to another thread, allowing core migration.
///
/// This function allows the scheduler to move the thread to a different CPU
/// core if it would improve system load balancing.
///
/// ## Migration Behavior
///
/// The scheduler may migrate the thread if:
/// - Current core is overloaded
/// - Another core is idle or less loaded
/// - Migration would improve overall throughput
/// - Thread priority warrants migration
///
/// ## Use Cases
///
/// - **Load Balancing**: Distribute work across cores
/// - **Thermal Management**: Move from hot to cooler cores
/// - **Power Efficiency**: Consolidate work on fewer cores
/// - **Fairness**: Ensure equal core utilization
///
/// ## Performance Trade-offs
///
/// **Benefits:**
/// - Better system-wide load distribution
/// - Potentially reduced wait times
/// - Improved multi-core utilization
///
/// **Costs:**
/// - Cache invalidation overhead
/// - Memory migration latency
/// - Possible NUMA effects (if applicable)
///
/// # See also
///
/// * [`yield_no_migration`] - Prevents core migration
/// * [`yield_to_any_thread`] - Forces immediate load balancing
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_with_migration() {
    svc::yield_with_migration();
}

/// Yields execution to any other thread, forcing load-balancing.
///
/// This function forces the scheduler to perform immediate system-wide load
/// balancing. It's the most aggressive yield variant and should be used
/// sparingly.
///
/// ## Scheduling Impact
///
/// This variant:
/// - Forces immediate load balancing decisions
/// - May cause multiple thread migrations
/// - Triggers scheduler rebalancing algorithms
/// - Can affect threads on all cores
///
/// ## When to Use
///
/// Reserve this for scenarios requiring:
/// - **Critical Load Balancing**: System is severely imbalanced
/// - **Priority Inversion Recovery**: Breaking priority deadlocks
/// - **System Recovery**: Responding to overload conditions
/// - **Benchmark Consistency**: Ensuring fair testing conditions
///
/// ## Performance Warning
///
/// This is the most expensive yield variant:
/// - High scheduler overhead
/// - Potential system-wide cache thrashing
/// - May cause priority inversions
/// - Can reduce overall system throughput
///
/// ## Alternative Approaches
///
/// Consider these alternatives first:
/// - [`yield_with_migration`] - Gentler load balancing
/// - Thread priority adjustments
/// - CPU affinity settings
/// - Work-stealing algorithms
///
/// # See also
///
/// * [`yield_no_migration`] - Prevents migration
/// * [`yield_with_migration`] - Allows optional migration
/// * <https://switchbrew.org/wiki/SVC#SleepThread>
#[inline]
pub fn yield_to_any_thread() {
    svc::yield_to_any_thread();
}
