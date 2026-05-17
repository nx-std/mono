//! Thread Control Block (TCB) and Dynamic Thread Vector (DTV).
//!
//! These two `#[repr(C)]` types implement the AArch64 TLS *variant I* model
//! used by musl libc. They describe the small fixed-layout header that lives at
//! a thread's thread pointer (TP) address and the indirection vector used to
//! reach the thread's ELF TLS block.
//!
//! # Per-thread layout
//!
//! ```text
//! TP (= __aarch64_read_tp()) →  ┌──────────┐
//!                               │ TCB      │  tls_start_offset() bytes (≥16)
//!                               │  .dtv    │  slot 0: pointer to the DTV
//!                               │  .thread │  slot 1: pointer to ThreadControl
//! __tls_start ───────────────→  ├──────────┤
//!                               │ .tdata   │  initialized TLS data
//!                               ├──────────┤
//!                               │ .tbss    │  zero-initialized TLS data
//!                               └──────────┘
//! ```
//!
//! The [`Tcb`] is exactly two pointer-sized slots (16 bytes on AArch64), so it
//! fits within `tls_start_offset()`. Slot 1 (`thread`) is musl libc's
//! `.private` slot, repurposed by `nx-sys-thread` to reach the authoritative
//! [`ThreadControl`] through the TP. `ThreadVars.thread_info_ptr` points at the
//! same core [`ThreadControl`]; this TCB slot is the alternate access path
//! through the thread pointer.
//!
//! The [`Dtv`] is a *single-entry* vector: Horizon homebrew is statically
//! linked, so there are no dynamically loaded TLS modules. `generation` is
//! therefore always `0` and `static_tls` always points at the one and only
//! static TLS block.
//!
//! # Intentional dead state
//!
//! The DTV is *built and reclaimed but never read*. AArch64 TLS variant I
//! resolves a `thread_local` access TP-relatively (`tpidr_el0 + offset`); on a
//! statically linked Horizon binary there is no `__tls_get_addr` and no
//! dynamic TLS module, so nothing dereferences [`Tcb::dtv`]. It is retained
//! deliberately for musl variant-I layout fidelity — the musl variant-I layout
//! fixes `dtv` at TCB slot 0, which in turn fixes `thread` at slot 1 — and as
//! the extension point a future dynamic-TLS implementation would consume.
//! Removing it would move `thread` to slot 0 and diverge from the musl
//! variant-I layout.

use core::ffi::c_void;

use static_assertions::const_assert_eq;

use crate::thread::ThreadControl;

/// Thread Control Block: the fixed-layout header at a thread's TP address.
///
/// Two pointer-sized slots following the AArch64 TLS variant I model:
/// `dtv` reaches the thread's ELF TLS block through the [`Dtv`], and `thread`
/// reaches the authoritative [`ThreadControl`] without consulting the kernel
/// TLS page.
#[repr(C)]
pub struct Tcb {
    /// Pointer to this thread's Dynamic Thread Vector.
    pub dtv: *mut Dtv,
    /// Pointer to the authoritative core thread state.
    pub thread: *mut ThreadControl,
}

impl Tcb {
    /// Builds a fully-initialized TCB from its two slot pointers.
    ///
    /// The result is written into the per-thread TLS allocation at the TP
    /// address; a `Tcb` carries no uninitialized state, so construction sets
    /// both slots at once rather than exposing a partial value.
    pub fn new(dtv: *mut Dtv, thread: *mut ThreadControl) -> Self {
        Self { dtv, thread }
    }
}

// AArch64 TLS variant I: the TCB is exactly two pointer-sized slots and must
// fit within tls_start_offset().
const_assert_eq!(size_of::<Tcb>(), 2 * size_of::<usize>());

/// Dynamic Thread Vector: the indirection vector reaching a thread's TLS block.
///
/// Single-entry for statically linked Horizon homebrew — there are no
/// dynamically loaded TLS modules, so [`generation`](Dtv::generation) is always
/// `0` and [`static_tls`](Dtv::static_tls) always points at the sole static
/// TLS block.
#[repr(C)]
pub struct Dtv {
    /// TLS generation counter; always `0` for the static-only DTV.
    pub generation: usize,
    /// Pointer to the thread's static ELF TLS block (`__tls_start`).
    pub static_tls: *mut c_void,
}

impl Dtv {
    /// Builds the static-only DTV pointing at a thread's TLS block.
    ///
    /// `tls_ptr` is the address of the thread's `.tdata`/`.tbss` block
    /// (`__tls_start`). `generation` is fixed at `0` because no dynamic TLS
    /// modules exist.
    pub fn new_static(tls_ptr: *mut c_void) -> Self {
        Self {
            generation: 0,
            static_tls: tls_ptr,
        }
    }
}

// The DTV is also two pointer-sized fields (16 bytes on AArch64).
const_assert_eq!(size_of::<Dtv>(), 2 * size_of::<usize>());
