//! C FFI bindings for compatibility with existing C code
//!
//! This module provides `#[no_mangle]` C functions that follow the nx-rand
//! naming convention for internal random operations.

use core::{
    ffi::c_void,
    slice,
};

use super::entropy;

/// Fills a buffer with random data.
///
/// This function is thread-safe and uses the ChaCha20 algorithm for generating
/// random numbers. The entropy is sourced from the kernel's TRNG.
///
/// # Safety
///
/// `buf` must point to a writable region of at least `len` bytes that stays
/// valid for the duration of the call.
///
/// # Panics
///
/// Panics on the process's first draw if the kernel refuses to report its entropy. The C
/// signature returns nothing, so there is no channel to report the failure through, and handing
/// back a buffer the caller will treat as random is the one outcome worse than ending the process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rand__random_get(buf: *mut c_void, len: usize) {
    let slice = unsafe { slice::from_raw_parts_mut(buf as *mut u8, len) };
    match entropy::fill(slice) {
        Ok(()) => (),
        Err(err) => panic!("{err}"),
    }
}

/// Returns a random 64-bit value.
///
/// This function is thread-safe and uses the ChaCha20 algorithm for generating
/// random numbers. The entropy is sourced from the kernel's TRNG.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
///
/// # Panics
///
/// Panics on the process's first draw if the kernel refuses to report its entropy. Every `u64` is
/// a valid return, so the C signature leaves no value to report the failure with.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rand__random_get64() -> u64 {
    match entropy::next_u64() {
        Ok(value) => value,
        Err(err) => panic!("{err}"),
    }
}
