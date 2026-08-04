//! FFI bindings for libsysbase syscalls (newlib integration).
//!
//! Provides implementations for newlib's time syscalls.
//!
//! # References
//!
//! - libgloss/libsysbase/syscall_support.c
//! - newlib/libc/include/sys/time.h

use core::ffi::{
    c_int,
    c_ulong,
    c_void,
};

use crate::sys::{
    clock::{
        self,
        aarch64::NSEC_PER_TICK,
    },
    timespec::ClockId,
};

/// Nanoseconds in a microsecond, for narrowing a `timespec` to a `timeval`.
const NSEC_PER_USEC: i64 = 1_000;

// Error codes
const EIO: c_int = 5;
const EFAULT: c_int = 14;
const EINVAL: c_int = 22;

/// C struct timespec
#[repr(C)]
pub struct CTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

/// C struct timeval
#[repr(C)]
pub struct CTimeval {
    tv_sec: i64,
    tv_usec: i64,
}

/// C struct timezone
#[repr(C)]
pub struct CTimezone {
    tz_minuteswest: c_int,
    tz_dsttime: c_int,
}

/// Get clock resolution.
///
/// Corresponds to libsysbase's `__syscall_clock_getres`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__libsysbase_syscall_clock_getres(
    clock_id: c_ulong,
    tp: *mut CTimespec,
) -> c_int {
    // The decoded clock is discarded: every clock this platform implements is driven by the
    // same counter-timer, so they share one resolution. Decoding is still how the argument is
    // validated, rather than re-listing the accepted values here.
    if decode_clock_id(clock_id).is_none() {
        set_errno(EINVAL);
        return -1;
    }
    if tp.is_null() {
        set_errno(EFAULT);
        return -1;
    }

    unsafe {
        (*tp).tv_sec = 0;
        (*tp).tv_nsec = NSEC_PER_TICK as i64;
    }
    0
}

/// Read a clock.
///
/// Corresponds to libsysbase's `__syscall_clock_gettime`.
///
/// # Safety
///
/// `tp` must be null or point to a writable `struct timespec`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__libsysbase_syscall_clock_gettime(
    clock_id: c_ulong,
    tp: *mut CTimespec,
) -> c_int {
    let Some(clock) = decode_clock_id(clock_id) else {
        set_errno(EINVAL);
        return -1;
    };
    if tp.is_null() {
        set_errno(EFAULT);
        return -1;
    }

    let now = match clock {
        ClockId::Monotonic => clock::aarch64::gettime(),
        ClockId::Realtime => match clock::service::gettime() {
            Ok(now) => now,
            // Nothing has anchored the wall clock yet, so there is no realtime reading to give.
            // libnx reports the same `EIO` while its `__boottime` is unset.
            Err(_) => {
                set_errno(EIO);
                return -1;
            }
        },
    };

    unsafe {
        (*tp).tv_sec = now.sec();
        (*tp).tv_nsec = now.nsec();
    }
    0
}

/// Read the wall clock, with microsecond resolution.
///
/// Corresponds to libsysbase's `__syscall_gettod_r`.
///
/// libnx writes `EIO` into the reentrancy structure it is handed; this writes it to the calling
/// thread's `errno` instead, which is the same location for every caller that passes its own
/// reentrancy structure.
///
/// # Safety
///
/// `tp` must be null or point to a writable `struct timeval`, and `tz` must be null or point to a
/// writable `struct timezone`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__libsysbase_syscall_gettod_r(
    _reent: *mut c_void,
    tp: *mut CTimeval,
    tz: *mut CTimezone,
) -> c_int {
    if !tp.is_null() {
        let now = match clock::service::gettime() {
            Ok(now) => now,
            // Nothing has anchored the wall clock yet, so there is no reading to give. The error
            // carries no detail `errno` could convey beyond the `EIO` libnx reports here too.
            Err(_) => {
                set_errno(EIO);
                return -1;
            }
        };

        unsafe {
            (*tp).tv_sec = now.sec();
            (*tp).tv_usec = now.nsec() / NSEC_PER_USEC;
        }
    }

    if !tz.is_null() {
        // The local timezone reaches newlib through the `TZ` environment variable, not through
        // this long-obsolete struct, so both fields stay zero as they do in libnx.
        unsafe {
            (*tz).tz_minuteswest = 0;
            (*tz).tz_dsttime = 0;
        }
    }

    0
}

/// Decodes the `clockid_t` a C caller passed across the FFI boundary.
///
/// Returns `None` if the value names no clock this platform implements.
fn decode_clock_id(clock_id: c_ulong) -> Option<ClockId> {
    // Narrowing is part of the decode: newlib types `clockid_t` as `unsigned long`, and every
    // clock this platform names fits an `i32`, so a value that does not narrow cannot be one of
    // them. Both errors are dropped because `EINVAL` is all POSIX lets these syscalls say about
    // an unknown `clockid_t`, so neither failure carries anything the caller could use.
    i32::try_from(clock_id)
        .ok()
        .and_then(|clock_id| ClockId::try_from(clock_id).ok())
}

/// Sets the thread-local `errno` value
#[inline]
fn set_errno(code: c_int) {
    unsafe extern "C" {
        // This is a newlib/libc function
        fn __errno() -> *mut c_int;
    }

    unsafe { *__errno() = code };
}
