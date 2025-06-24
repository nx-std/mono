//! FFI bindings for the thread activity API.

use nx_svc::error::ToRawResultCode;

use crate::thread_impl::{self as sys, Thread};

/// Pauses the execution of a thread (see [`activity::pause`]).
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_pause(t: *const Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t };

    sys::pause(thread).map_or_else(|err| err.to_rc(), |_| 0)
}

/// Resumes the execution of a previously paused thread (see [`activity::resume`]).
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_resume(t: *const Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t };

    sys::resume(thread).map_or_else(|err| err.to_rc(), |_| 0)
}
