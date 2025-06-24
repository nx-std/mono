//! Thread-Local Storage (TLS)
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
//! ## References
//! - [Switchbrew Wiki: Thread Local Region](https://switchbrew.org/wiki/Thread_Local_Region)
//! - [switchbrew/libnx: tls.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/tls.h)

use core::{ffi::c_void, mem::size_of};

use nx_cpu::control_regs;

/// Size of the Thread Local Storage (TLS) region.
pub const TLS_SIZE: usize = 0x200;

/// Start of the user-mode TLS region.
pub const USER_TLS_BEGIN: usize = 0x100;

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

/// Get a raw pointer to the Thread Local Storage (TLS) buffer.
///
/// This function reads the `tpidrro_el0` system register, which holds the
/// read-only thread pointer for the current thread. The returned pointer
/// points to a 512-byte (0x200) Thread Local Storage (TLS) region.
///
/// # Returns
///
/// Raw pointer to the 512-byte Thread Local Storage (TLS) for the current thread.
///
/// # Safety
///
/// This function is safe to call, but dereferencing the returned pointer
/// requires careful attention to the TLS memory layout.
#[inline]
pub fn get_tls_ptr() -> *mut c_void {
    unsafe { control_regs::tpidrro_el0() }
}
