//! The wall-clock anchor behind [`SystemTime`](crate::SystemTime).
//!
//! Reading wall-clock time on Horizon means talking to the `time` service over IPC. This crate
//! deliberately stays out of that: it is the platform abstraction the realtime clock is built
//! on, not a service client. Instead the runtime reads the wall clock once at startup and
//! installs it here, and every [`SystemTime::now`](crate::SystemTime::now) after that is derived
//! from the counter-timer.
//!
//! Until [`set_anchor`] is called, [`SystemTime::now`](crate::SystemTime::now) reports the Unix
//! epoch and the newlib `clock_gettime`/`gettimeofday` syscalls report `EIO`.

use crate::sys::clock::service;

/// Anchors the realtime clock to `unix_secs`, paired with the counter-timer reading taken now.
///
/// `unix_secs` is a POSIX timestamp — seconds since 1970-01-01 00:00:00 UTC — as the `time`
/// service reports it. Calling this again re-anchors the clock; the most recent reading wins.
pub fn set_anchor(unix_secs: u64) {
    service::set_anchor(unix_secs);
}
