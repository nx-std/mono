//! System-specific time implementation

// Code borrowed, with modifications, from: https://github.com/rust-lang/rust/blob/ed49386d3aa3a445a9889707fd405df01723eced/library/std/src/sys/pal/unix/time.rs
// Licensed under: Apache-2.0 OR MIT

pub mod clock;
mod nsec;
pub(crate) mod timespec;

use core::{fmt, time::Duration};

use self::timespec::{ClockId, Timespec};

// An anchor in time which can be used to create new `SystemTime` instances or
/// learn about where in time a `SystemTime` lies.
pub const UNIX_EPOCH: SystemTime = SystemTime {
    t: Timespec::zero(),
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemTime {
    t: Timespec,
}

impl SystemTime {
    pub fn now() -> SystemTime {
        SystemTime {
            t: Timespec::now(ClockId::Realtime),
        }
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.t.sub_timespec(&other.t)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime {
            t: self.t.checked_add_duration(other)?,
        })
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime {
            t: self.t.checked_sub_duration(other)?,
        })
    }
}

impl fmt::Debug for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemTime")
            .field("tv_sec", &self.t.sec())
            .field("tv_nsec", &self.t.nsec())
            .finish()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(Timespec);

impl Instant {
    pub fn now() -> Instant {
        Instant(Timespec::now(ClockId::Monotonic))
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        self.0.sub_timespec(&other.0).ok()
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        self.0.checked_add_duration(other).map(Instant)
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        self.0.checked_sub_duration(other).map(Instant)
    }
}

impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instant")
            .field("tv_sec", &self.0.sec())
            .field("tv_nsec", &self.0.nsec())
            .finish()
    }
}
