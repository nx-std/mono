//! Thread context utilities.
//!
//! This module offers safe, Rust-friendly wrappers over the
//! `svcGetThreadContext3` syscall used to dump the CPU and FPU registers of a
//! *paused* thread on Horizon OS.
//!
//! The main entry-point is [`dump_context`], which returns a [`ThreadContext`]
//! populated with the register state of the target thread.  The layout of
//! [`ThreadContext`], together with the [`CpuRegister`] and [`FpuRegister`]
//! unions, matches the C definitions in *libnx*’s `thread_context.h`.
//!
//! **Pre-condition:** the target thread must have been paused beforehand (see
//! [`crate::thread_impl::activity::pause`]) to guarantee a consistent
//! snapshot.

use nx_svc::{raw, thread as svc};

use super::handle::Thread;

/// Dumps the CPU/FPU context of a *paused* thread.
///
/// The target `thread` **must** have been paused beforehand (see
/// [`super::activity::pause`]) otherwise the kernel will refuse the request
/// with an error.
pub fn dump_context(thread: &Thread) -> Result<Context, DumpContextError> {
    svc::get_context3(thread.handle)
        .map(Into::into)
        .map_err(Into::into)
}

/// 64/32-bit CPU register view as returned by `svcGetThreadContext3`.
///
/// Matches the C layout in `thread_context.h`.
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

/// CPU/FPU register dump for a paused thread.
///
/// This mirrors the kernel layout used by `svcGetThreadContext3` and the
/// C definition in libnx's `thread_context.h`. All fields are public so
/// they can be inspected directly.  For convenience a [`From`] impl allows
/// loss-free conversion from the raw representation returned by
/// `nx_svc`.
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
    pub pc: raw::CpuRegister,
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

impl Context {
    /// Returns `true` when the saved context belongs to an AArch64 thread.
    #[inline]
    pub fn is_aarch64(&self) -> bool {
        // Same check performed by the kernel helper in raw::ThreadContext.
        (self.psr & 0x10) == 0
    }

    /// Gives immutable access to the underlying raw context.
    #[inline]
    pub fn as_raw(&self) -> &raw::ThreadContext {
        unsafe { &*(self as *const Context as *const raw::ThreadContext) }
    }

    /// Gives mutable access to the underlying raw context.
    #[inline]
    pub fn as_raw_mut(&mut self) -> &mut raw::ThreadContext {
        unsafe { &mut *(self as *mut Context as *mut raw::ThreadContext) }
    }
}

/// Loss-free conversion from the raw SVC representation.
impl From<raw::ThreadContext> for Context {
    fn from(raw_ctx: raw::ThreadContext) -> Self {
        // SAFETY: `CpuRegister`/`FpuRegister` are layout-identical to their
        // raw counterparts, hence a transmute is sound.
        let cpu_gprs: [CpuRegister; 29] = unsafe { core::mem::transmute(raw_ctx.cpu_gprs) };
        let fpu_gprs: [FpuRegister; 32] = unsafe { core::mem::transmute(raw_ctx.fpu_gprs) };

        // SAFETY: `CpuRegister`/`FpuRegister` are layout-identical to their
        // raw counterparts, hence a transmute is sound.
        let pc = unsafe { core::mem::transmute(raw_ctx.pc) };

        Self {
            cpu_gprs,
            fp: raw_ctx.fp,
            lr: raw_ctx.lr,
            sp: raw_ctx.sp,
            pc,
            psr: raw_ctx.psr,
            fpu_gprs,
            fpcr: raw_ctx.fpcr,
            fpsr: raw_ctx.fpsr,
            tpidr: raw_ctx.tpidr,
        }
    }
}

/// Error type returned by [`dump_context`].
#[derive(Debug, thiserror::Error)]
pub enum DumpContextError {
    /// Supplied handle does not refer to a valid thread.
    #[error("Invalid handle")]
    InvalidHandle,

    /// Any unforeseen kernel error. Contains the original [`nx_svc::result::Error`]
    /// so callers can inspect the raw result code.
    #[error("Unknown error: {0}")]
    Unknown(nx_svc::result::Error),
}

impl From<svc::GetContext3Error> for DumpContextError {
    fn from(value: svc::GetContext3Error) -> Self {
        match value {
            svc::GetContext3Error::InvalidHandle => DumpContextError::InvalidHandle,
            svc::GetContext3Error::Unknown(err) => DumpContextError::Unknown(err),
        }
    }
}

#[cfg(feature = "ffi")]
impl nx_svc::error::ToRawResultCode for DumpContextError {
    fn to_rc(self) -> nx_svc::error::ResultCode {
        match self {
            Self::InvalidHandle => svc::GetContext3Error::InvalidHandle.to_rc(),
            Self::Unknown(err) => err.to_rc(),
        }
    }
}
