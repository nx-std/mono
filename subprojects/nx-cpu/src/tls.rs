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
//! - [ARM TPIDRRO_ELO Register](https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/TPIDRRO-EL0--EL0-Read-Only-Software-Thread-ID-Register)
//! - [rust-embedded/aarch64-cpu: tpidrro_el0.rs](https://github.com/rust-embedded/aarch64-cpu/blob/main/src/registers/tpidrro_el0.rs)
//! - [switchbrew/libnx: tls.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/arm/tls.h)

use core::{arch::asm, ffi::c_void};

/// Gets the thread-local storage (TLS) buffer.
///
/// This function reads the `tpidrro_el0` system register, which holds the
/// read-only thread pointer for the current thread.
///
/// Returns a pointer to the thread-local storage buffer.
///
/// ## References
/// - [ARM TPIDRRO_ELO Register](https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/TPIDRRO-EL0--EL0-Read-Only-Software-Thread-ID-Register)
#[inline]
#[unsafe(no_mangle)]
pub fn __nx_cpu_get_tls() -> *mut c_void {
    let tls_ptr: *mut c_void;
    unsafe {
        asm!(
            "mrs {:x}, tpidrro_el0", // Move the value of tpidrro_el0 into tls_ptr
            out(reg) tls_ptr,        // Output: tls_ptr will hold the value of tpidrro_el0
            options(nostack, nomem), // No stack or memory operands
        );
    }
    tls_ptr
}
