//! Realtime clock, derived from a wall-clock reading taken once at startup.
//!
//! Horizon exposes no realtime clock a process can read directly: wall-clock time comes from the
//! `time` service over IPC, which is far too expensive to pay on every `clock_gettime`. So the
//! runtime reads it once during startup and anchors it here, and every later reading is that
//! anchor plus the counter-timer ticks elapsed since. This is libnx's `__boottime` /
//! `__bootticks` pair, and the anchoring step is its `__libnx_init_time`.
//!
//! The anchor lives here rather than in the runtime because this crate owns the realtime clock;
//! the runtime, which owns the `time` service session, pushes the reading in through
//! [`crate::realtime`].

use core::sync::atomic::{
    AtomicU64,
    Ordering,
};

use nx_cpu::counter::Ticks;

use super::aarch64::{
    TIMER_FREQ,
    cpu_ticks_to_ns,
    get_system_tick,
};
use crate::sys::timespec::Timespec;

/// Value stored in [`ANCHOR_UNIX_SECS`] while no anchor has been installed.
///
/// libnx spells the same "no wall clock yet" state as `__boottime == UINT64_MAX`.
const NO_ANCHOR: u64 = u64::MAX;

/// Seconds since the Unix epoch at the moment [`ANCHOR_TICKS`] was read.
static ANCHOR_UNIX_SECS: AtomicU64 = AtomicU64::new(NO_ANCHOR);

/// Counter-timer reading taken alongside [`ANCHOR_UNIX_SECS`].
static ANCHOR_TICKS: AtomicU64 = AtomicU64::new(0);

/// Anchors the realtime clock to `unix_secs`, paired with the counter-timer reading taken now.
///
/// The wall-clock value is whatever the caller already obtained; the tick it is paired with is
/// read here. Any delay between the two (an IPC round trip, say) shifts the clock forward by
/// that much, exactly as it does in libnx.
pub fn set_anchor(unix_secs: u64) {
    // The tick is published first so a reader that races this install can only observe the
    // older, slower anchor rather than one dated into the future.
    ANCHOR_TICKS.store(get_system_tick().to_raw(), Ordering::Relaxed);
    ANCHOR_UNIX_SECS.store(unix_secs, Ordering::Release);
}

/// Reads the realtime clock.
///
/// # Errors
///
/// Returns [`NotAnchoredError`] until [`set_anchor`] has installed a wall-clock reading.
pub fn gettime() -> Result<Timespec, NotAnchoredError> {
    let anchor_unix_secs = ANCHOR_UNIX_SECS.load(Ordering::Acquire);
    if anchor_unix_secs == NO_ANCHOR {
        return Err(NotAnchoredError);
    }
    let anchor_ticks = ANCHOR_TICKS.load(Ordering::Relaxed);

    let elapsed = get_system_tick().to_raw() - anchor_ticks;
    let elapsed_secs = elapsed / TIMER_FREQ.to_raw();
    // SAFETY: A remainder of a tick count is itself a tick count.
    let subsec_ticks = Ticks::from_u64_unchecked(elapsed % TIMER_FREQ.to_raw());

    // The seconds cast is lossless for any wall-clock value the `time` service reports; it would
    // take a timestamp beyond year 292277026596 to reach the sign bit.
    // SAFETY: `subsec_ticks` is strictly less than one second's worth of ticks, so the
    // nanoseconds it converts to are within `0..NSEC_PER_SEC`.
    Ok(Timespec::new_unchecked(
        (anchor_unix_secs + elapsed_secs) as i64,
        cpu_ticks_to_ns(subsec_ticks) as i64,
    ))
}

/// An error indicating that the realtime clock has no wall-clock anchor yet.
#[derive(Debug, thiserror::Error)]
#[error("the realtime clock has no wall-clock anchor")]
pub struct NotAnchoredError;
