use core::{ffi::c_void, ptr};

use crate::thread_impl as sys;

/// Reads the raw pointer stored in the dynamic TLS slot `slot_id`.
///
/// Mirrors `threadTlsGet` in libnx's C API.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_tls_get(slot_id: i32) -> *mut c_void {
    // Negative indices are treated as out-of-bounds and return null, matching C UB-avoidance.
    if slot_id < 0 {
        return ptr::null_mut();
    }
    // SAFETY: Cast is safe because slot_id is non-negative; additional bounds checks are
    // performed inside `slot_get` when debug assertions are enabled.
    unsafe { sys::slot_get(slot_id as usize) }
}

/// Writes `value` into dynamic TLS slot `slot_id`.
///
/// Mirrors `threadTlsSet` in libnx's C API.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_tls_set(slot_id: i32, value: *mut c_void) {
    if slot_id < 0 {
        return; // Silently ignore invalid negative indices to match common C semantics.
    }

    // SAFETY: Same considerations as in `__nx_sys_thread_tls_get`.
    unsafe { sys::slot_set(slot_id as usize, value) };
}

mod newlib {
    use core::{ffi::c_void, ptr};

    use crate::{thread_impl as sys, tls::NUM_TLS_SLOTS};

    /// POSIX constant for `EINVAL` (invalid argument).
    /// Mirrors the value used by newlib on Horizon (22).
    const EINVAL: i32 = 22;

    /// Rust implementation of `pthread_setspecific` that libgloss/newlib expects.
    ///
    /// # Safety
    /// The caller must ensure that `key` was previously obtained via a successful call to
    /// `pthread_key_create` (or equivalent) and therefore lies within the dynamic TLS slot
    /// range `[0, NUM_TLS_SLOTS)`.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_sys_thread_newlib_pthread_setspecific(
        key: u32,
        value: *const c_void,
    ) -> i32 {
        if (key as usize) >= NUM_TLS_SLOTS {
            return EINVAL;
        }

        // SAFETY: Bounds were checked above; we cast away constness because TLS stores a mutable
        // pointer value. The pointer itself may point to immutable data.
        unsafe { sys::slot_set(key as usize, value as *mut c_void) };
        0 // success
    }

    /// Rust implementation of `pthread_getspecific` that libgloss/newlib expects.
    ///
    /// Returns the stored pointer or NULL if `key` is out of range.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_sys_thread_newlib_pthread_getspecific(key: u32) -> *mut c_void {
        if (key as usize) >= NUM_TLS_SLOTS {
            return ptr::null_mut();
        }

        // SAFETY: Bounds were checked above.
        unsafe { sys::slot_get(key as usize) }
    }
}
