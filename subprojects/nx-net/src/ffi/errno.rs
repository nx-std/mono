//! Writers for the C thread-local error variables a resolver caller observes.
//!
//! A C resolver reports failures through two thread-local globals:
//! newlib's `errno`, and the resolver's own `h_errno` (set by the
//! `gethostby*` family). `nx-net` overrides the resolver *functions* but not
//! these variables, so the FFI must write the *existing* C globals for a C
//! caller to observe them — newlib still owns `errno`, and the C runtime this
//! links against still defines `h_errno`.

use core::ffi::c_int;

/// Sets newlib's thread-local `errno` for the calling thread.
///
/// `__errno` is the newlib accessor that returns a pointer to the current
/// thread's `errno` slot; writing through it is how every C library reports a
/// POSIX error code.
pub fn set_errno(code: c_int) {
    unsafe extern "C" {
        // newlib accessor for the calling thread's `errno` slot.
        fn __errno() -> *mut c_int;
    }

    // SAFETY: `__errno` always returns a valid, writable pointer to the
    // calling thread's `errno` slot.
    unsafe { *__errno() = code };
}

/// Sets the resolver thread-local `h_errno` for the calling thread.
///
/// `h_errno` is a `__thread int` the linked C runtime defines. Because
/// `nx-net` redirects only the resolver functions, that definition survives
/// and remains the variable C callers read after a `gethostby*` call.
pub fn set_h_errno(code: c_int) {
    unsafe extern "C" {
        /// Resolver thread-local error variable, defined by the linked C runtime.
        #[thread_local]
        static mut h_errno: c_int;
    }

    // SAFETY: `h_errno` is the linked C runtime's resolver thread-local; its definition is
    // retained because only the resolver functions are overridden.
    unsafe { h_errno = code };
}

/// Reads the resolver thread-local `h_errno` for the calling thread.
///
/// The companion of [`set_h_errno`]: `herror` reports the description of
/// whatever `h_errno` the most recent `gethostby*` call left behind.
pub fn get_h_errno() -> c_int {
    unsafe extern "C" {
        /// Resolver thread-local error variable, defined by the linked C runtime.
        #[thread_local]
        static h_errno: c_int;
    }

    // SAFETY: `h_errno` is the linked C runtime's resolver thread-local; its definition is
    // retained because only the resolver functions are overridden.
    unsafe { h_errno }
}
