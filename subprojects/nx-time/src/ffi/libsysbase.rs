//! FFI bindings for libsysbase syscalls (newlib integration).
//!
//! Provides implementations for newlib's time syscalls.
//!
//! # References
//!
//! - libgloss/libsysbase/syscall_support.c
//! - newlib/libc/include/sys/time.h

use core::ffi::c_int;

use crate::sys::{clock::aarch64::NSEC_PER_TICK, timespec::ClockId};

/// C struct timespec
#[repr(C)]
pub struct CTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

// Error codes
const EFAULT: c_int = 14;
const EINVAL: c_int = 22;

/// Get clock resolution.
///
/// Corresponds to libsysbase's `__syscall_clock_getres`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_time__libsysbase_syscall_clock_getres(
    clock_id: c_int,
    tp: *mut CTimespec,
) -> c_int {
    // The decoded clock is discarded: every clock this platform implements is driven by the
    // same counter-timer, so they share one resolution. Decoding is still how the argument is
    // validated, rather than re-listing the accepted values here.
    if ClockId::try_from(clock_id).is_err() {
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

/// Sets the thread-local `errno` value
#[inline]
fn set_errno(code: c_int) {
    unsafe extern "C" {
        // This is a newlib/libc function
        fn __errno() -> *mut c_int;
    }

    unsafe { *__errno() = code };
}
