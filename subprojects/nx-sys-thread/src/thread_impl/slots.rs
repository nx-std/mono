//! Dynamic Thread-Local Storage (TLS) slot management.
//!
//! Horizon allocates a small **user-TLS** area inside every thread's Thread-
//! Local Storage block. libnx exposes four C helpers around that region:
//! `threadTlsAlloc`, `threadTlsFree`, `threadTlsGet`, and `threadTlsSet`. This
//! Rust module re-implements the same functionality while preserving the exact
//! ABI expected by Horizon and by C code linking against libnx.

use core::{ffi::c_void, ptr, slice};

use crate::tls::{self, NUM_TLS_SLOTS, USER_TLS_BEGIN};

/// Reads the raw pointer stored in the dynamic TLS slot with the given `slot_id`.
///
/// Mirrors libnx's `threadTlsGet`.
///
/// # Safety
/// - The caller must ensure `slot_id < NUM_TLS_SLOTS`.
/// - The caller must ensure the slots slice is not aliased mutably elsewhere.
#[inline]
pub unsafe fn slot_get(slot_id: usize) -> *mut c_void {
    #[cfg(debug_assertions)]
    {
        use nx_svc::debug::{BreakReason, break_event};
        if slot_id >= NUM_TLS_SLOTS {
            // TODO: Add a proper error message here.
            // panic!("TLS slot out of bounds: {}", slot_id);
            break_event(BreakReason::Assert, 0, 0);
        }
    }

    // SAFETY: index validated above; slice lives as long as the function call.
    unsafe { ptr::read_volatile(&slots()[slot_id]) }
}

/// Writes `value` into the dynamic TLS slot with the given `slot_id`.
///
/// Mirrors libnx's `threadTlsSet`.
///
/// # Safety
/// - The caller must ensure `slot_id < NUM_TLS_SLOTS`.
/// - The caller must ensure the slice is not aliased mutably elsewhere.
#[inline]
pub unsafe fn slot_set(slot_id: usize, value: *mut c_void) {
    #[cfg(debug_assertions)]
    {
        use nx_svc::debug::{BreakReason, break_event};
        if slot_id >= NUM_TLS_SLOTS {
            // TODO: Add a proper error message here.
            // panic!("TLS slot out of bounds: {}", slot_id);
            break_event(BreakReason::Assert, 0, 0);
        }
    }

    // SAFETY: index validated above.
    unsafe { ptr::write_volatile(&mut slots_mut()[slot_id], value) }
}

/// Returns a slice covering the dynamic TLS slot array for the **current thread**.
///
/// # Safety
/// * The returned slice is valid **only** for the lifetime of the current call on the
///   current thread. Callers must **not** store it for later use, and it must never be
///   sent to or accessed from another thread.
/// * The caller must ensure the returned slice is not aliased mutably elsewhere.
#[inline(always)]
unsafe fn slots() -> &'static [*mut c_void] {
    let tls_ptr = tls::get_ptr();

    // SAFETY: The caller must ensure the returned slice is not aliased mutably elsewhere.
    unsafe {
        let slots_ptr = tls_ptr.add(USER_TLS_BEGIN);
        slice::from_raw_parts(slots_ptr as *mut *mut c_void, NUM_TLS_SLOTS)
    }
}

/// Returns a mutable slice covering the dynamic TLS slot array for the **current thread**.
///
/// # Safety
/// * The returned slice is valid **only** for the lifetime of the current call on the
///   current thread. Callers must **not** store it for later use, and it must never be
///   sent to or accessed from another thread.
/// * The caller must ensure the returned slice is not aliased mutably elsewhere.
#[inline(always)]
unsafe fn slots_mut() -> &'static mut [*mut c_void] {
    let tls_ptr = tls::get_ptr();

    // SAFETY: The caller must ensure the returned slice is not aliased mutably elsewhere.
    unsafe {
        let slots_ptr = tls_ptr.add(USER_TLS_BEGIN);
        slice::from_raw_parts_mut(slots_ptr as *mut *mut c_void, NUM_TLS_SLOTS)
    }
}
