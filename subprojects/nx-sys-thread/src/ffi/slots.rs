use core::{ffi::c_void, ptr};

/// Reads the raw pointer stored in the dynamic TLS slot `slot_id`.
///
/// Mirrors `threadTlsGet` in libnx's C API.
///
/// TODO: Add support for dynamic TLS slots
/// Currently returns NULL for all slot IDs as dynamic slots are not implemented.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_tls_get(_slot_id: i32) -> *mut c_void {
    // Dynamic TLS slots not supported - always return NULL
    ptr::null_mut()
}

/// Writes `value` into dynamic TLS slot `slot_id`.
///
/// Mirrors `threadTlsSet` in libnx's C API.
///
/// TODO: Add support for dynamic TLS slots
/// Currently does nothing as dynamic slots are not implemented.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread_tls_set(_slot_id: i32, _value: *mut c_void) {
    // Dynamic TLS slots not supported - silently ignore
}

mod newlib {
    use core::{ffi::c_void, ptr};

    /// POSIX constant for `ENOSYS` (function not implemented).
    /// Standard POSIX error code (38).
    const ENOSYS: i32 = 38;

    /// Rust implementation of `pthread_setspecific` that libgloss/newlib expects.
    ///
    /// TODO: Add support for pthread TLS functions
    /// Currently returns ENOSYS as dynamic slots are not implemented.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_sys_thread_newlib_pthread_setspecific(
        _key: u32,
        _value: *const c_void,
    ) -> i32 {
        // Dynamic TLS slots not supported - return ENOSYS (function not implemented)
        ENOSYS
    }

    /// Rust implementation of `pthread_getspecific` that libgloss/newlib expects.
    ///
    /// TODO: Add support for pthread TLS functions
    /// Currently returns NULL as dynamic slots are not implemented.
    #[unsafe(no_mangle)]
    unsafe extern "C" fn __nx_sys_thread_newlib_pthread_getspecific(_key: u32) -> *mut c_void {
        // Dynamic TLS slots not supported - always return NULL
        ptr::null_mut()
    }
}
