//! FFI bindings for the thread context API.
//!
//! This module exposes a C-compatible interface that mirrors the
//! `threadDumpContext` helper provided by libnx. Internally it delegates to
//! the safe Rust implementation living in `crate::thread_impl::context` and
//! performs the necessary type conversions.
//!
//! The CPU/FPU register unions and the main [`Context`] structure are
//! re-declared here with a `#[repr(C)]` layout so they can be consumed from C
//! code directly.

use nx_svc::error::ToRawResultCode;

use crate::thread_impl as sys;

/// Dumps the CPU/FPU context of a *paused* thread into `ctx`.
///
/// # Safety
/// * `t` must point to a valid [`Thread`] instance.
/// * `ctx` must be non-null and point to writable memory large enough to hold
///   a [`Context`] structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_sys_thread__thread_dump_context(
    ctx: *mut Context,
    t: *const sys::Thread,
) -> u32 {
    // SAFETY: The caller guarantees that the pointers are valid.
    let thread = unsafe { &*t };

    match sys::dump_context(&thread) {
        Ok(sys_ctx) => {
            // Write the converted context back to the caller-provided buffer.
            unsafe { ctx.write(sys_ctx.into()) };
            0
        }
        Err(err) => err.to_rc(),
    }
}

/// CPU/FPU register dump for a paused thread.
#[repr(C)]
pub struct Context {
    /// General-purpose CPU registers X0..X28.
    pub cpu_gprs: [CpuRegister; 29],
    /// Frame pointer (X29).
    pub fp: u64,
    /// Link register (X30).
    pub lr: u64,
    /// Stack pointer.
    pub sp: u64,
    /// Program counter.
    pub pc: CpuRegister,
    /// Processor status register.
    pub psr: u32,
    /// NEON registers V0..V31.
    pub fpu_gprs: [FpuRegister; 32],
    /// Floating-point control register.
    pub fpcr: u32,
    /// Floating-point status register.
    pub fpsr: u32,
    /// EL0 Read/Write Software Thread ID Register.
    pub tpidr: u64,
}

/// 64/32-bit CPU register view as returned by `svcGetThreadContext3`.
#[repr(C)]
pub union CpuRegister {
    /// 64-bit AArch64 view (Xn)
    pub x: u64,
    /// 32-bit AArch64 view (Wn)
    pub w: u32,
    /// AArch32 view (Rn)
    pub r: u32,
}

/// 128/64/32-bit NEON register view.
#[repr(C)]
pub union FpuRegister {
    /// 128-bit vector (Vn)
    pub v: u128,
    /// 64-bit double-precision floating point (Dn)
    pub d: f64,
    /// 32-bit single-precision floating point (Sn)
    pub s: f32,
}

impl From<sys::Context> for Context {
    fn from(value: sys::Context) -> Self {
        // SAFETY: `CpuRegister`/`FpuRegister` are layout-identical to their
        // counterparts in `ctx`, therefore this transmute is sound.
        let cpu_gprs = unsafe { core::mem::transmute(value.cpu_gprs) };
        let fpu_gprs = unsafe { core::mem::transmute(value.fpu_gprs) };
        let pc = unsafe { core::mem::transmute(value.pc) };

        Self {
            cpu_gprs,
            fp: value.fp,
            lr: value.lr,
            sp: value.sp,
            pc,
            psr: value.psr,
            fpu_gprs,
            fpcr: value.fpcr,
            fpsr: value.fpsr,
            tpidr: value.tpidr,
        }
    }
}
