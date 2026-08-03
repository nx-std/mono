//! C FFI bindings for compatibility with existing C code
//!
//! This module provides `#[no_mangle]` C functions that follow the nx-rand
//! naming convention for internal random operations.

use core::{ffi::c_void, slice};

use super::sys;

/// Fills a buffer with random data.
///
/// This function is thread-safe and uses the ChaCha20 algorithm for generating
/// random numbers. The entropy is sourced from the kernel's TRNG.
///
/// # Safety
///
/// `buf` must point to a writable region of at least `len` bytes that stays
/// valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rand__random_get(buf: *mut c_void, len: usize) {
    let slice = unsafe { slice::from_raw_parts_mut(buf as *mut u8, len) };
    sys::fill_bytes(slice)
}

/// Returns a random 64-bit value.
///
/// This function is thread-safe and uses the ChaCha20 algorithm for generating
/// random numbers. The entropy is sourced from the kernel's TRNG.
///
/// # Safety
///
/// No special requirements beyond typical FFI safety.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_rand__random_get64() -> u64 {
    sys::next_u64()
}
