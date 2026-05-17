//! C ABI adapter layer for `nx-sys-thread`.
//!
//! This module is only compiled with the `ffi` Cargo feature. It hosts the
//! `__nx_sys_thread__*` override symbols redirected by `sys_thread_override.ld`
//! at link time, replacing libnx and devkitPro/libsysbase thread functions.
//!
//! The adapters are thin: they validate raw pointers and scalars at the edge,
//! call the idiomatic Rust core, and translate results back into the C ABI.
//!
//! - [`libnx`] mirrors the libnx `thread.h` C ABI.
//! - [`libsysbase`] mirrors the devkitPro/libsysbase newlib thread syscall ABI.
//! - `registry` is the shared `C handle -> pinned core` side registry the
//!   thread-lifecycle adapters anchor on (Resolved Question #5).
//! - `reent` provisions spawned threads with a real newlib `_reent` block via
//!   the C shim `csrc/reent_shim.c` (Task 6.6 / Resolved Question #6).

pub mod libnx;
pub mod libsysbase;
pub(crate) mod reent;
mod registry;
