//! Memory the caller lent us.
//!
//! A C caller passes a buffer as a pointer and a length that are separate arguments and can
//! disagree, and an output as a pointer it may have declined to supply. This is where both stop
//! being raw: what comes out is a slice bounded by the length the caller declared, or a write that
//! does nothing when the caller wanted no value.

use core::ffi::c_void;

/// Borrows a caller's buffer as bytes.
///
/// Returns an empty slice for a null pointer or a zero length, which together are how C says "no
/// buffer". An empty slice is what the commands take for that case, so the absence needs no
/// separate spelling downstream.
///
/// # Safety
///
/// `ptr` must be null, or point to at least `len` readable bytes that stay valid and unwritten for
/// the lifetime of the returned slice.
pub(super) unsafe fn bytes<'a>(ptr: *const c_void, len: u32) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        return &[];
    }

    // SAFETY: the caller guarantees `len` readable bytes at `ptr` for the returned lifetime.
    unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len as usize) }
}

/// Borrows a caller's buffer as bytes it will read back.
///
/// Returns an empty slice for the same reasons [`bytes`] does.
///
/// # Safety
///
/// `ptr` must be null, or point to at least `len` writable bytes that no other reference addresses
/// for the lifetime of the returned slice.
pub(super) unsafe fn bytes_mut<'a>(ptr: *mut c_void, len: u32) -> &'a mut [u8] {
    if ptr.is_null() || len == 0 {
        return &mut [];
    }

    // SAFETY: the caller guarantees `len` writable bytes at `ptr`, exclusively held for the
    // returned lifetime.
    unsafe { core::slice::from_raw_parts_mut(ptr.cast::<u8>(), len as usize) }
}

/// Writes `value` through an out-pointer the caller may have declined to supply.
///
/// A null out-pointer is how C says it does not want the value, and every command below that has
/// one accepts that.
///
/// # Safety
///
/// `out` must be null or point to a writable `T`.
pub(super) unsafe fn write_out<T>(out: *mut T, value: T) {
    if out.is_null() {
        return;
    }

    // SAFETY: the caller guarantees a writable `T` at a non-null `out`.
    unsafe { *out = value };
}
