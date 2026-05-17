//! ELF TLS segment management.
//!
//! This module turns the linker-emitted ELF TLS segment into per-thread state.
//! It reads the TLS linker symbols, reports the sizes a thread's TLS allocation
//! must reserve, copies the static `.tdata`/`.tbss` image into a freshly
//! allocated TLS block, and builds the [`Tcb`]/[`Dtv`] header that sits at the
//! thread pointer.
//!
//! # Per-thread TLS layout
//!
//! ```text
//! TP (= __aarch64_read_tp()) →  ┌──────────┐
//!                               │ TCB      │  tls_start_offset() bytes (≥16)
//! __tls_start ───────────────→  ├──────────┤
//!                               │ .tdata   │  copied from __tdata_lma
//!                               ├──────────┤
//!                               │ .tbss    │  zero-initialized
//!                               └──────────┘
//! ```
//!
//! The [`Dtv`] is allocated separately on the heap (a single 16-byte node)
//! rather than embedded in the contiguous stack+TLS block: statically linked
//! Horizon homebrew never grows the DTV, so the simpler layout wins. The owning
//! thread reclaims it on close.

use alloc::boxed::Box;
use core::ptr;

use crate::tcb::{Dtv, Tcb};

// Linker-emitted symbols delimiting the ELF TLS segment.
//
// `__tls_start`/`__tls_end` bound the runtime TLS block, `__tdata_lma` and
// `__tdata_lma_end` bound the initialized-data image in the executable, and
// `__tls_align` is an absolute symbol carrying the segment's alignment.
unsafe extern "C" {
    static __tls_start: u8;
    static __tls_end: u8;
    static __tls_align: usize;
    static __tdata_lma: u8;
    static __tdata_lma_end: u8;
}

/// Alignment, in bytes, applied to a thread's TLS block size.
const TLS_BLOCK_ALIGN: usize = 16;

/// Returns the size, in bytes, of a thread's TLS block (`.tdata` + `.tbss`).
///
/// The raw `__tls_end - __tls_start` extent is rounded up to
/// [`TLS_BLOCK_ALIGN`] so the block following the TCB stays aligned.
pub fn tls_size() -> usize {
    let start = &raw const __tls_start as usize;
    let end = &raw const __tls_end as usize;
    let raw = end - start;
    (raw + TLS_BLOCK_ALIGN - 1) & !(TLS_BLOCK_ALIGN - 1)
}

/// Returns the offset, in bytes, from the thread pointer (TP) to `__tls_start`.
///
/// This is the space reserved for the [`Tcb`] and matches libnx's
/// `getTlsStartOffset()`: the larger of the TCB size and the TLS segment
/// alignment.
pub fn tls_start_offset() -> usize {
    // SAFETY: `__tls_align` is a linker-provided absolute symbol; reading its
    // value is always sound.
    let align = unsafe { __tls_align };
    size_of::<Tcb>().max(align)
}

/// Returns the base address of the linker-emitted `__tls_start` symbol.
///
/// This is the base of the *main thread's* static ELF TLS block: the loader
/// places the main thread's `.tdata`/`.tbss` image at this fixed address, so —
/// unlike a spawned thread, whose block [`crate::thread::create`] allocates and
/// fills — the main thread's TLS block needs no copy. Walking back from here by
/// [`tls_start_offset`] yields the main thread's TCB address.
pub fn tls_start() -> *mut u8 {
    (&raw const __tls_start).cast_mut()
}

/// Returns the size, in bytes, of the initialized `.tdata` image.
pub fn tdata_size() -> usize {
    (&raw const __tdata_lma_end as usize) - (&raw const __tdata_lma as usize)
}

/// Returns the size, in bytes, of the zero-initialized `.tbss` region.
///
/// This is the TLS block remainder after the `.tdata` image, including any
/// padding introduced by rounding [`tls_size`] up to [`TLS_BLOCK_ALIGN`].
pub fn tbss_size() -> usize {
    tls_size() - tdata_size()
}

/// Initializes a thread's ELF TLS block: copies `.tdata`, zeros `.tbss`.
///
/// # Safety
///
/// `dst` must point to a writable, [`tls_size`]-byte region owned by the
/// thread being initialized, and must not alias the `.tdata` image.
pub unsafe fn init_tls_block(dst: *mut u8) {
    let tdata = tdata_size();
    if tdata > 0 {
        // SAFETY: `__tdata_lma` delimits the initialized-TLS image in the
        // executable; the caller guarantees `dst` owns at least `tls_size()`
        // bytes and `tdata <= tls_size()`. The image and `dst` do not overlap.
        unsafe { ptr::copy_nonoverlapping(&raw const __tdata_lma, dst, tdata) };
    }

    let tbss = tbss_size();
    if tbss > 0 {
        // SAFETY: `.tbss` begins right after the copied `.tdata` bytes and
        // stays within the caller-owned `tls_size()`-byte block.
        unsafe { ptr::write_bytes(dst.add(tdata), 0, tbss) };
    }
}

/// Initializes the [`Tcb`] at `tcb_ptr` and its heap-allocated [`Dtv`].
///
/// The DTV is created as a single static node ([`Dtv::new_static`]) pointing at
/// `tls_start`, and its pointer is written into the TCB's `dtv` slot. The TCB's
/// `thread` slot is left null; the thread creation flow fills it once the
/// `ThreadControl` address is known. The owning thread reclaims the DTV with
/// `Box::from_raw` on close.
///
/// # Safety
///
/// `tcb_ptr` must point to a writable, suitably aligned [`Tcb`] slot, and
/// `tls_start` must be the address of that thread's TLS block.
pub unsafe fn init_tcb_and_dtv(tcb_ptr: *mut Tcb, tls_start: *mut u8) {
    let dtv = Box::into_raw(Box::new(Dtv::new_static(tls_start.cast())));

    // SAFETY: the caller guarantees `tcb_ptr` is a valid, writable, suitably
    // aligned `Tcb` slot.
    unsafe { tcb_ptr.write(Tcb::new(dtv, ptr::null_mut())) };
}
