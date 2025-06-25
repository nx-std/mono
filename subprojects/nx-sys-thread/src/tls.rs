//! # Thread-Local Storage (TLS)
//!
//! The thread-local region (TLR) is a 0x200-byte area.
//!
//! Its base address is loaded via the ARM thread ID register `tpidrro_el0`. Multiple threads store
//! their TLRs in the same page, with the first TLR typically located at `page + 0x200`, as the
//! first TLR spot is reserved for user-mode exception handling.
//!
//! In threads created by the Nintendo SDK, `tpidr_el0` is assigned to the `ThreadPointer` object
//! from the thread-local region.
//!
//! ## TLS layout overview
//! The **complete** 0x200-byte block, including the ABI-mandated *Thread-Control
//! Block* (TCB), looks like this:
//!
//! ```text
//! TLS base (TPIDRRO_EL0)
//! 0x000  ┌────────────────────────────┐
//!        │ Thread-Control Block (TCB) │ 16 Bytes
//! 0x010  ├────────────────────────────┤
//!        │  .tdata / .tbss            │ ← Compiler-generated static TLS
//! 0x108  ├────────────────────────────┤  ╮  
//!        │  Dynamic TLS *slots*       │  │  
//! 0x1E0  ├────────────────────────────┤  ├  User TLS region
//!        │  ThreadVars  (32 bytes)    │  │
//! 0x200  └────────────────────────────┘  ╯
//! ```
//!
//! The first 16 bytes satisfy the AArch64 TLS ABI so that compiler-generated
//! accesses to `__aarch64_read_tp()` work, while the final 32 bytes are reserved
//! for `ThreadVars`. Everything in between is available for user and linker-
//! provided TLS data.
//!
//! ### User TLS region
//! From TLS base + 0x108 onward the block belongs entirely to user-mode code.
//! The layout is:
//!
//! ```text
//! TLS base (TPIDRRO_EL0) + 0x108
//! 0x108  ┌────────────────────────────┐
//!        │ Slot 0  (*mut c_void)      │ 8 Bytes
//! 0x110  ├────────────────────────────┤
//!        │ Slot 1  (*mut c_void)      │ 8 Bytes
//! 0x118  ├────────────────────────────┤
//!        ┆             …              ┆
//! 0x1D0  ├────────────────────────────┤
//!        │ Slot 25 (*mut c_void)      │ 8 Bytes
//! 0x1D8  ├────────────────────────────┤
//!        │ Slot 26 (*mut c_void)      │ 8 Bytes
//! 0x1E0  ╞════════════════════════════╡
//!        │ ThreadVars (0x20 bytes)    │
//! 0x200  └────────────────────────────┘
//! ```
//!
//! #### Dynamic TLS slots (`0x108` – `0x1DF`)
//!
//! Array of runtime-allocated pointers used by libnx to implement thread-local
//! storage that is *not* known at link-time (e.g. `pthread_key_create`, C
//! locale, etc.).  Each thread has its own copy; slot IDs are process-global.
//!
//! * 27 entries (`NUM_TLS_SLOTS`) of pointer-sized storage. Each slot can be
//!   claimed at runtime with `threadTlsAlloc()`/`threadTlsSet()` (see libnx C
//!   API) or—on the Rust side—via higher-level wrappers.
//! * A process-global bitmask tracks which slot IDs are in use; an optional
//!   *destructor* function may be registered so that per-thread cleanup runs
//!   automatically when the thread exits (`threadExit`).
//! * Access is purely arithmetic: `TPIDRRO_EL0 + 0x108 + slot_id *
//!   size_of::<*mut c_void>()`, no syscalls needed.
//! * Each entry is pointer-sized, so it can hold any `*mut T` or small integral
//!   value cast to `usize`.
//!
//! #### [`ThreadVars`] (`0x1E0` – `0x200`)
//!
//! A fixed 32-byte footer holding per-thread metadata that libnx needs
//! constantly: a *magic* value, the thread's kernel handle, a link to the
//! rust/C thread object, a pointer to the newlib re-entrancy state and a cached
//! copy of the thread-pointer (TP) value.
//!
//! ```text
//! TLS base + 0x1E0
//! 0x1E0 ┌────────────────────────────┐
//!       │ magic       (u32)          │
//! 0x1E4 ├────────────────────────────┤
//!       │ handle      (u32)          │
//! 0x1E8 ├────────────────────────────┤
//!       │ thread_ptr  (*mut c_void)  │
//! 0x1F0 ├────────────────────────────┤
//!       │ reent       (*mut c_void)  │
//! 0x1F8 ├────────────────────────────┤
//!       │ tls_tp      (*mut c_void)  │
//! 0x200 └────────────────────────────┘
//! ```
//!
//! ## References
//! - [Switchbrew Wiki: Thread Local Region](https://switchbrew.org/wiki/Thread_Local_Region)
//! - [switchbrew/libnx: tls.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/tls.h)

use core::{ffi::c_void, mem::size_of, ptr};

use nx_cpu::control_regs;
use nx_svc::thread::Handle;

/// Size of the Thread Local Storage (TLS) region.
pub const TLS_SIZE: usize = 0x200;

/// Start of the user-mode TLS region.
pub const USER_TLS_BEGIN: usize = 0x108;

/// End of the user-mode TLS region.
pub const USER_TLS_END: usize = TLS_SIZE - THREAD_VARS_SIZE;

/// Size of the ThreadVars structure  
///
/// The [`ThreadVars`] structure is exactly 32 bytes (0x20) long and is stored at the end
/// of the thread's TLS segment within the Thread Local Region.
pub const THREAD_VARS_SIZE: usize = 0x20;

/// The number of slots in the TLS region.
///
/// The TLS region is divided into slots of size `core::mem::size_of::<*mut c_void>()`.
///
/// The number of slots is calculated as the difference between the end and the beginning
/// of the user-mode TLS region, divided by the size of the slot.
pub const NUM_TLS_SLOTS: usize = (USER_TLS_END - USER_TLS_BEGIN) / size_of::<*mut c_void>();

/// Magic value used to verify that the [`ThreadVars`] structure is initialised.
///
/// The value corresponds to the ASCII string "!TV$".
pub const THREAD_VARS_MAGIC: u32 = 0x21545624;

// Linker-defined symbols
unsafe extern "C" {
    /// Start (Load Memory Address) of the `.tdata` section as provided by the
    /// linker script.
    ///
    /// In `switch.ld` you will find the following line:
    /// `PROVIDE_HIDDEN( __tdata_lma = ADDR(.tdata) );`
    ///
    /// At runtime this symbol points to the first byte of the initialised
    /// thread-local data that needs to be copied into each thread's TLS area.
    pub static __tdata_lma: u8;

    /// End (one-past-the-last byte) address of the `.tdata` section.
    ///
    /// Defined by the linker via:
    /// `PROVIDE_HIDDEN( __tdata_lma_end = ADDR(.tdata) + SIZEOF(.tdata) );`
    ///
    /// `(__tdata_lma_end as usize - __tdata_lma as usize)` yields the size of
    /// the initialised TLS data block.
    pub static __tdata_lma_end: u8;

    /// Start address of the memory reserved for the main thread's Thread-Local
    /// Storage (TLS) block.
    ///
    /// The linker emits this via:
    /// `PROVIDE_HIDDEN( __tls_start = ADDR(.main.tls) );`
    ///
    /// Together with `__tls_end` this symbol delimits the TLS area that holds
    /// `.tdata` followed by `.tbss` for the initial thread.
    pub static __tls_start: u8;

    /// End address (one-past-the-last byte) of the main thread's TLS block.
    ///
    /// Linker source:
    /// `PROVIDE_HIDDEN( __tls_end = ADDR(.main.tls) + SIZEOF(.main.tls) );`
    pub static __tls_end: u8;

    /// Alignment requirement (in bytes) for a TLS block.
    ///
    /// The value is emitted in the `.tls.align` section using:
    /// `QUAD( MAX( ALIGNOF(.tdata), ALIGNOF(.tbss) ) )`
    /// and then exposed via
    /// `PROVIDE_HIDDEN( __tls_align = ADDR(.tls.align) );`
    ///
    /// Runtime code that allocates TLS for new threads should honour this
    /// alignment.
    pub static __tls_align: usize;
}

/// Per-thread variables located at the end of the TLS area.
///
/// The struct occupies exactly [`THREAD_VARS_SIZE`] bytes (0x20) and matches the
/// layout used by the Horizon OS loader as documented on Switchbrew.
#[derive(Debug)]
#[repr(C)]
pub struct ThreadVars {
    /// Magic value used to check if the struct is initialised.
    pub magic: u32,
    /// Kernel handle identifying the thread.
    pub handle: Handle,
    /// Pointer to the current thread object (if any).
    pub thread_info_ptr: *mut c_void,
    /// Pointer to the thread's newlib reentrancy state.
    pub reent: *mut c_void,
    /// Pointer to this thread's thread-local segment (TP).
    ///
    /// This is located at *TLS + 0x1F8*.
    pub tls_tp: *mut c_void,
}

/// Returns the base address of this thread's Thread-Local Storage (TLS) block as a plain
/// `usize`.
///
/// On AArch64 the per-thread TLS pointer is exposed to user-mode code via the
/// read-only system register `TPIDRRO_EL0`. Horizon OS initialises this register
/// during thread creation to point at the first byte of the 0x200-byte TLS
/// block described at the top of this module.
///
/// This function is nothing more than a thin, *safe* wrapper around a single
/// `mrs` instruction that reads that register. Because merely reading the
/// register cannot violate any safety guarantees the function is safe to call;
/// however any *use* of the returned address (e.g. by dereferencing it) must
/// observe the TLS layout documented in this file.
///
/// If you need a raw pointer instead of an integer address, use
/// [`get_ptr`] which performs the cast for you.
#[inline]
pub fn get_base_addr() -> usize {
    unsafe { control_regs::tpidrro_el0() }
}

/// Returns a raw pointer to the 512-byte Thread Local Storage (TLS) for the
/// current thread.
///
/// This is simply [`get_base_addr`] cast to a pointer, so obtaining the value
/// is completely safe. **Dereferencing** the pointer, however, requires `unsafe`
/// code and must respect the TLS layout documented in this module.
#[inline]
pub fn get_ptr() -> *mut c_void {
    get_base_addr() as *mut c_void
}

/// Returns a raw pointer to the [`ThreadVars`] for the current thread.
#[inline]
pub fn thread_vars_ptr() -> *mut ThreadVars {
    let tls_ptr = get_ptr();

    // SAFETY: The TLS area is 0x200 bytes in size, the [`ThreadVars`] sits at
    // the very end of it.
    unsafe { tls_ptr.add(TLS_SIZE - THREAD_VARS_SIZE) as *mut ThreadVars }
}

/// Returns the [`Handle`] of the current thread.
#[inline]
pub fn get_current_thread_handle() -> Handle {
    let tv = thread_vars_ptr();
    // SAFETY: `tv` points to a valid `ThreadVars` inside the current thread's
    // TLS block. The field access is performed with `read_volatile` to avoid
    // the compiler re-ordering or eliminating the read.
    unsafe { ptr::read_volatile(&raw const (*tv).handle) }
}

/// Returns the offset (in bytes) from the beginning of the TLS block to the
/// start of the *static* thread-local data (.tdata / .tbss).
///
/// The TLS area begins with the *Thread Control Block* (TCB), which on Horizon is defined as two
/// pointer-sized fields (16 bytes on AArch64).  
///
/// The actual thread–local data must be placed after this TCB, but it might also require a stricter
/// alignment as communicated by the linker via the [`__tls_align`] symbol. At runtime we therefore
/// take the maximum of the natural TCB size and the linker-supplied alignment value.
#[inline]
pub fn static_tls_data_start_offset() -> usize {
    // The Horizon TCB consists of two pointer-sized slots.
    let tcb_sz = 2 * size_of::<*mut c_void>();

    // SAFETY: `__tls_align` is set up by the linker and guaranteed to point to a valid `usize`
    // that contains the required alignment of the TLS block.
    let align = unsafe { __tls_align };

    if align > tcb_sz { align } else { tcb_sz }
}

#[cfg(test)]
mod tests {
    use static_assertions::const_assert_eq;

    use super::{THREAD_VARS_SIZE, ThreadVars};

    // Ensure the layout stays consistent with Horizon expectations.
    const_assert_eq!(size_of::<ThreadVars>(), THREAD_VARS_SIZE);
}
