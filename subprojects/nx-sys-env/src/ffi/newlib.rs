//! Environment entry points, replacing `newlib`'s.
//!
//! The C library keeps an environment of its own, and leaving it in place would
//! mean two: a binding made from Rust invisible to a C `getenv`, and a C
//! assignment invisible to Rust. These entry points redirect the C surface at
//! the store in [`crate::environment`], so there is one environment and both sides
//! see it.
//!
//! Redirecting the readers alone would not do. The C library's own assignment
//! reallocates the array it finds in `environ`, and that array is a Rust `Vec`
//! here, so the mutators have to come across too or the C library frees memory
//! it does not own.
//!
//! The `environ` symbol itself lives beside the store it mirrors, for the
//! reason given there.
//!
//! ## Where this differs from the C library
//!
//! - **`putenv` copies.** POSIX lets the string a caller passes *become* part of
//!   the environment, so a later write through that pointer changes the binding.
//!   Here the entry is copied into the store, and the caller's string is its own
//!   again once the call returns. The aliasing form has no way to exist over a
//!   Rust-owned store.
//! - **`errno` is not set.** A failing call reports `-1` as it should, but the
//!   reason is not published, because this crate has no route to the C library's
//!   per-thread `errno` that does not reintroduce the dependency the store
//!   exists to remove.

use core::ffi::{
    CStr,
    c_char,
    c_int,
    c_void,
};

use crate::environment;

/// Looks up a name in the environment.
///
/// Returns a pointer into the environment's own storage, which the next
/// assignment may invalidate, exactly as the C library's does.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. `name` must be NULL or a
/// valid nul-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_getenv(name: *const c_char) -> *mut c_char {
    // SAFETY: the caller guarantees NULL or a valid nul-terminated string.
    let Some(name) = (unsafe { name_bytes(name) }) else {
        return core::ptr::null_mut();
    };

    environment::c_value_ptr(name)
}

/// Looks up a name in the environment, ignoring the reentrancy structure.
///
/// The structure carries the C library's per-thread state, and this
/// implementation keeps none of it.
///
/// # Safety
///
/// The same as [`__nx_sys_env__newlib_getenv`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_getenv_r(
    _reent: *mut c_void,
    name: *const c_char,
) -> *mut c_char {
    // SAFETY: the caller carries the obligation forward unchanged.
    unsafe { __nx_sys_env__newlib_getenv(name) }
}

/// Binds a name to a value, replacing an existing binding when `overwrite` is
/// non-zero and leaving it alone otherwise.
///
/// Returns `0`, or `-1` when the name or the value cannot be represented.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. `name` and `value` must be
/// NULL or valid nul-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_setenv(
    name: *const c_char,
    value: *const c_char,
    overwrite: c_int,
) -> c_int {
    // SAFETY: the caller guarantees NULL or a valid nul-terminated string.
    let Some(name) = (unsafe { name_bytes(name) }) else {
        return -1;
    };
    // SAFETY: the caller guarantees NULL or a valid nul-terminated string.
    let Some(value) = (unsafe { name_bytes(value) }) else {
        return -1;
    };

    if overwrite == 0 && environment::get(name).is_some() {
        return 0;
    }

    match environment::set(name, value) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Binds a name to a value, ignoring the reentrancy structure.
///
/// # Safety
///
/// The same as [`__nx_sys_env__newlib_setenv`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_setenv_r(
    _reent: *mut c_void,
    name: *const c_char,
    value: *const c_char,
    overwrite: c_int,
) -> c_int {
    // SAFETY: the caller carries the obligation forward unchanged.
    unsafe { __nx_sys_env__newlib_setenv(name, value, overwrite) }
}

/// Removes a binding, reporting success whether or not one was there.
///
/// Returns `0`, or `-1` when the name cannot be represented.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. `name` must be NULL or a
/// valid nul-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_unsetenv(name: *const c_char) -> c_int {
    // SAFETY: the caller guarantees NULL or a valid nul-terminated string.
    let Some(name) = (unsafe { name_bytes(name) }) else {
        return -1;
    };

    if name.is_empty() || name.contains(&b'=') {
        return -1;
    }
    environment::unset(name);

    0
}

/// Removes a binding, ignoring the reentrancy structure.
///
/// # Safety
///
/// The same as [`__nx_sys_env__newlib_unsetenv`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_unsetenv_r(
    _reent: *mut c_void,
    name: *const c_char,
) -> c_int {
    // SAFETY: the caller carries the obligation forward unchanged.
    unsafe { __nx_sys_env__newlib_unsetenv(name) }
}

/// Binds the `KEY=VALUE` entry a caller supplies.
///
/// Returns `0`, or `-1` when the entry holds no `=` or cannot be represented.
/// The entry is copied rather than adopted; see the module documentation.
///
/// # Safety
///
/// FFI entry point: the caller upholds the C ABI. `entry` must be NULL or a
/// valid nul-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_putenv(entry: *mut c_char) -> c_int {
    // SAFETY: the caller guarantees NULL or a valid nul-terminated string.
    let Some(entry) = (unsafe { name_bytes(entry) }) else {
        return -1;
    };

    let Some(separator) = entry.iter().position(|&byte| byte == b'=') else {
        return -1;
    };

    match environment::set(&entry[..separator], &entry[separator + 1..]) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Binds a `KEY=VALUE` entry, ignoring the reentrancy structure.
///
/// # Safety
///
/// The same as [`__nx_sys_env__newlib_putenv`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_env__newlib_putenv_r(
    _reent: *mut c_void,
    entry: *mut c_char,
) -> c_int {
    // SAFETY: the caller carries the obligation forward unchanged.
    unsafe { __nx_sys_env__newlib_putenv(entry) }
}

/// Borrows a C string's bytes, or `None` when the pointer is NULL.
///
/// # Safety
///
/// `string` must be NULL or address a nul-terminated string that stays valid
/// for as long as the returned bytes are used.
unsafe fn name_bytes<'a>(string: *const c_char) -> Option<&'a [u8]> {
    if string.is_null() {
        return None;
    }

    // SAFETY: the caller guarantees a valid nul-terminated string that outlives
    // the borrow.
    Some(unsafe { CStr::from_ptr(string) }.to_bytes())
}
