//! FFI bindings for the thread activity API.

use nx_svc::error::ToRawResultCode;

use crate::thread_impl as sys;

/// Starts the execution of a thread.
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_start(t: *const sys::Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t };

    sys::start(thread).map_or_else(|err| err.to_rc(), |_| 0)
}

/// Pauses the execution of a thread.
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_pause(t: *const sys::Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t };

    sys::pause(thread).map_or_else(|err| err.to_rc(), |_| 0)
}

/// Resumes the execution of a previously paused thread.
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_resume(t: *const sys::Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t };

    sys::resume(thread).map_or_else(|err| err.to_rc(), |_| 0)
}
