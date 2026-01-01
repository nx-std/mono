use core::{ffi::c_void, ptr};

/// Reads the raw pointer stored in the dynamic TLS slot `slot_id`.
///
/// Mirrors `threadTlsGet` in libnx's C API.
///
/// TODO: Add support for dynamic TLS slots
/// Currently returns NULL for all slot IDs as dynamic slots are not implemented.
#[unsafe(no_mangle)]
unsafe extern "C" fn __nx_sys_thread__thread_tls_get(_slot_id: i32) -> *mut c_void {
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
unsafe extern "C" fn __nx_sys_thread__thread_tls_set(_slot_id: i32, _value: *mut c_void) {
    // Dynamic TLS slots not supported - silently ignore
}
