//! Functions to read and write control registers
//!
//! This module provides functions for interacting with the CPU control registers.

use core::arch::naked_asm;

/// Read the `cntpct_el0` system register.
///
/// This function reads the `cntpct_el0` system register, which holds the current value of the
/// CPU counter-timer.
///
/// Returns the current system tick as a `u64`.
///
/// # Counter-timer Physical Count register - EL0
///
/// Holds the 64-bit physical count value.
///
/// # References
///
/// - [ARM CNTPCT-EL0 Register](https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/CNTPCT-EL0--Counter-timer-Physical-Count-Register)
/// - [rust-embedded/aarch64-cpu: cntpct_el0](https://github.com/rust-embedded/aarch64-cpu/blob/f8bf731f0d0bda084302f04adb5b3a0a2c448d9e/src/registers/cntpct_el0.rs)
///
/// # SAFETY
///
/// This function is `naked`, and its body is written in assembly.
/// The assembly code reads the `cntpct_el0` system register and returns
/// its value in `x0`, according to the AArch64 procedure call standard.
/// The `noreturn` option is used to prevent the compiler from generating
/// a function prologue and epilogue.
#[unsafe(naked)]
pub unsafe extern "C" fn cntpct_el0() -> u64 {
    naked_asm!(
        "mrs x0, cntpct_el0", // Move the value of `cntpct_el0` into the return register `x0`
        "ret",
    );
}

/// Read the `cntfrq_el0` system register.
///
/// This function reads the `cntfrq_el0` system register, which holds the
/// frequency of the system counter-timer.
///
/// Returns the system counter-timer frequency, in Hz.
///
/// # Counter-timer Frequency register - EL0
///
/// This register is provided so that software can discover the frequency of the system counter.
/// It must be programmed with this value as part of system initialization. The value of the
/// register is not interpreted by hardware.
///
/// # References
///
/// - [ARM CNTFRQ-EL0 Register](https://developer.arm.com/documentation/ddi0601/2020-12/AArch64-Registers/CNTFRQ-EL0--Counter-timer-Frequency-register)
/// - [rust-embedded/aarch64-cpu: cntfrq_el0.rs](https://github.com/rust-embedded/aarch64-cpu/blob/f8bf731f0d0bda084302f04adb5b3a0a2c448d9e/src/registers/cntfrq_el0.rs)
///
/// # SAFETY
///
/// This function is `naked`, and its body is written in assembly.
/// The assembly code reads the `cntfrq_el0` system register and returns
/// its value in `x0`, according to the AArch64 procedure call standard.
/// The `noreturn` option is used to prevent the compiler from generating
/// a function prologue and epilogue.
#[unsafe(naked)]
pub unsafe extern "C" fn cntfrq_el0() -> u64 {
    naked_asm!(
        "mrs x0, cntfrq_el0", // Move the value of `cntfrq_el0` into the return register `x0`
        "ret",
    );
}

/// Read the `tpidrro_el0` system register.
///
/// This function reads the `tpidrro_el0` system register, which holds the read-only thread pointer
/// for the current thread.
///
/// Returns the base address of the Thread-Local Storage (TLS) buffer.
///
/// # References
///
/// - [ARM TPIDRRO_ELO Register](https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/TPIDRRO-EL0--EL0-Read-Only-Software-Thread-ID-Register)
/// - [rust-embedded/aarch64-cpu: tpidrro_el0.rs](https://github.com/rust-embedded/aarch64-cpu/blob/main/src/registers/tpidrro_el0.rs)
///
/// # SAFETY
///
/// This function is `naked`, and its body is written in assembly.
/// The assembly code reads the `tpidrro_el0` system register and returns
/// its value in `x0`, according to the AArch64 procedure call standard.
/// The `noreturn` option is used to prevent the compiler from generating
/// a function prologue and epilogue.
#[unsafe(naked)]
pub unsafe extern "C" fn tpidrro_el0() -> usize {
    naked_asm!(
        "mrs x0, tpidrro_el0", // Move the value of `tpidrro_el0` into the return register `x0`
        "ret",
    );
}
