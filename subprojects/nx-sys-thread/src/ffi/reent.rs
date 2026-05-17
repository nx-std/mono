//! newlib `_reent` support for the FFI surface.
//!
//! Spawned threads need a real newlib reentrancy block so `errno` and stdio
//! calls work on them, matching libnx `threadCreate` (IC-16). `sizeof(struct
//! _reent)` and the `_REENT_INIT_PTR` initializer are devkitA64 newlib ABI
//! details a pure-Rust crate cannot reproduce, so they live in the C shim
//! `csrc/reent_shim.c`. This module is the thin Rust side that calls into it;
//! [`thread::create`](crate::thread) reserves the block in the mapped stack
//! mirror and initializes it through [`init_block`] while the new thread is
//! still suspended.
//!
//! The shim also exposes [`set_errno`]: writing newlib `errno` likewise routes
//! through `__errno()` into the calling thread's `_reent`, so that ABI detail
//! stays in C next to the provisioning code.

use core::{
    ffi::{c_int, c_void},
    ptr::NonNull,
};

unsafe extern "C" {
    /// 16-byte-aligned `sizeof(struct _reent)` — see `csrc/reent_shim.c`.
    static __nx_sys_thread_reent_size: usize;

    /// Runs `_REENT_INIT_PTR` over `child` and inherits the creating thread's
    /// standard streams — see `csrc/reent_shim.c`.
    fn __nx_sys_thread_reent_init(child: *mut c_void);

    /// Writes `code` into the calling thread's newlib `errno` — see
    /// `csrc/reent_shim.c`.
    fn __nx_sys_thread_set_errno(code: c_int);
}

/// Returns the byte size of a per-thread newlib `_reent` block.
///
/// Sourced from the C shim's `sizeof(struct _reent)` so the Rust layout cannot
/// transcribe — and drift from — the newlib ABI.
pub(crate) fn block_size() -> usize {
    // SAFETY: `__nx_sys_thread_reent_size` is a `const size_t` defined by the
    // C shim that is linked into every FFI build; reading it is a plain load.
    unsafe { __nx_sys_thread_reent_size }
}

/// Initializes a freshly reserved `_reent` block for a spawned thread.
///
/// Delegates to the C shim, which mirrors libnx `threadCreate`: it runs
/// `_REENT_INIT_PTR` over the block and inherits the creating thread's
/// `stdin`/`stdout`/`stderr` handles.
///
/// # Safety
///
/// `child` must point at [`block_size`] writable bytes reserved for this
/// thread's `_reent`, and the call must run while that thread is suspended so
/// no newlib code observes the block mid-initialization.
pub(crate) unsafe fn init_block(child: NonNull<c_void>) {
    // SAFETY: by the contract `child` is a writable `_reent`-sized block; the
    // C shim only memsets and field-writes within it.
    unsafe { __nx_sys_thread_reent_init(child.as_ptr()) }
}

/// Sets the calling thread's newlib `errno`.
///
/// Delegates to the C shim, which writes through newlib's `__errno()` so the
/// value lands in the calling thread's `_reent`. The FFI build provisions a
/// real per-thread `_reent` (see [`init_block`]), so this is sound on every
/// thread that can reach the override surface.
pub(crate) fn set_errno(code: c_int) {
    // SAFETY: `__nx_sys_thread_set_errno` is defined by the C shim linked into
    // every FFI build; it performs a single write through newlib's `__errno()`.
    unsafe { __nx_sys_thread_set_errno(code) }
}
