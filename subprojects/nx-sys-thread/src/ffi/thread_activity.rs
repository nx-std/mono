//! FFI bindings for the thread activity API.

use nx_svc::error::ToRawResultCode;

use super::thread_info::Thread;
use crate::thread_impl as sys;

/// Starts the execution of a thread.
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_start(t: *const Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t }.into();

    sys::start(&thread).map_or_else(|err| err.to_rc(), |_| 0)
}

/// Pauses the execution of a thread.
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_pause(t: *const Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t }.into();

    sys::pause(&thread).map_or_else(|err| err.to_rc(), |_| 0)
}

/// Resumes the execution of a previously paused thread.
///
/// # Safety
///
/// The caller must ensure that `t` points to a valid [`Thread`] instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread_resume(t: *const Thread) -> u32 {
    // SAFETY: The caller must ensure that `t` is non-null.
    let thread = unsafe { &*t }.into();

    sys::resume(&thread).map_or_else(|err| err.to_rc(), |_| 0)
}
