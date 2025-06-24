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

use core::ffi::c_void;

use nx_cpu::control_regs;

/// Get a raw pointer to the Thread Local Region (TLR) buffer.
///
/// This function reads the `tpidrro_el0` system register, which holds the
/// read-only thread pointer for the current thread. The returned pointer
/// points to a 512-byte (0x200) Thread Local Region.
///
/// # Returns
///
/// Raw pointer to the 512-byte Thread Local Region (TLR) for the current thread.
///
/// # Safety
///
/// This function is safe to call, but dereferencing the returned pointer
/// requires careful attention to the TLR memory layout.
#[inline]
pub fn get_tlr_ptr() -> *mut c_void {
    unsafe { control_regs::tpidrro_el0() }
}
