//! The C library's per-thread reentrancy structure.
//!
//! Every C entry point in the device operation table receives a pointer to this structure. It is
//! opaque here: the crate never reads a field, never constructs one, and only ever writes the
//! leading error number when reporting failure back across the boundary.
//!
//! The type is part of the operation table's signatures ([`super::devoptab`]), so it is available
//! whether or not the C-facing surface is compiled in. Reporting failures through it is not; that
//! belongs to the boundary alone.
//!
//! # References
//!
//! - newlib/libc/include/sys/reent.h

use core::ffi::c_int;

/// Opaque handle to the C library's `struct _reent`.
#[repr(C)]
pub struct Reent {
    errno: c_int,
}

impl Reent {
    /// Records `errno` as this thread's failure reason.
    pub(crate) fn set_errno(&mut self, errno: c_int) {
        self.errno = errno;
    }
}
