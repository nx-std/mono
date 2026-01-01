//! FFI bindings for the *wait for exit* thread API.
//!
//! This module exposes a C-compatible wrapper around the high-level
//! [`wait_thread_exit`] helper so it can be invoked from
//! C code via the libnx-style interface (`threadWaitForExit`).
//!
//! [`wait_thread_exit`]: crate::wait_thread_exit

use nx_svc::error::ToRawResultCode;

use crate::thread_impl as sys;

/// Blocks the caller until the target thread has terminated.
///
/// Mirrors libnx's `threadWaitForExit` function.
///
/// # Safety
/// * `t` must be non-null and point to a valid [`Thread`] instance.
/// * The pointed-to thread must outlive this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_wait_for_exit(t: *const sys::Thread) -> u32 {
    // SAFETY: The caller is responsible for ensuring `t` is non-null and valid.
    let thread = unsafe { &*t };

    sys::wait_thread_exit(thread).map_or_else(|err| err.to_rc(), |_| 0)
}
