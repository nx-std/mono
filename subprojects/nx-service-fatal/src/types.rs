//! Wire-layout types for the fatal service.

use static_assertions::const_assert_eq;

/// Policy controlling fatal error behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(u32)]
pub enum FatalPolicy {
    ErrorReportAndErrorScreen = 0,
    ErrorReport = 1,
    ErrorScreen = 2,
}

/// AArch64 CPU context for fatal errors.
///
/// The register area is 33 words: the 31 general-purpose registers the
/// architecture defines, then the stack pointer and program counter, which are
/// not general-purpose registers and so are named rather than indexed. libnx
/// spells the same area as a union of `x[32]` and a named form running one
/// word longer; C sizes the union by the longer arm, so 33 words is what it
/// puts on the wire.
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct FatalAarch64Context {
    /// General-purpose registers `x0` through `x30`. `x[29]` is the frame
    /// pointer and `x[30]` the link register, per the AArch64 procedure call
    /// standard.
    pub x: [u64; 31],
    /// Stack pointer at the fault.
    pub sp: u64,
    /// Program counter at the fault.
    pub pc: u64,
    /// Processor state at the fault.
    pub pstate: u64,
    /// Auxiliary fault status, register 0.
    pub afsr0: u64,
    /// Auxiliary fault status, register 1.
    pub afsr1: u64,
    /// Exception syndrome: what the exception was.
    pub esr: u64,
    /// Fault address: where it happened, when the syndrome carries one.
    pub far: u64,
    /// Unwound return addresses, most recent first.
    pub stack_trace: [u64; 32],
    /// Address of the first NSO loaded, generally the process entrypoint.
    pub start_address: u64,
    /// Bit `i` is set when `x[i]` holds a value worth reporting.
    pub register_set_flags: u64,
    /// Number of entries of `stack_trace` that are populated.
    pub stack_trace_size: u32,
    /// Trailing padding to the context's 8-byte alignment. Zero on the wire.
    pub _pad: u32,
}

const_assert_eq!(size_of::<FatalAarch64Context>(), 0x248);

/// CPU context reported with a fatal error.
///
/// The wire form discriminates an AArch64 context from an AArch32 one, and
/// reserves the larger of the two. This crate builds only for `aarch64`, and
/// the context describes the process reporting the fault, so only the AArch64
/// arm is reachable: the discriminant is written false and the AArch64 context
/// fills the reservation exactly.
#[derive(Debug, Clone, Copy)]
pub struct FatalCpuContext {
    /// Register state, stack trace and fault registers at the failure.
    pub ctx: FatalAarch64Context,
    /// Exception type.
    pub context_type: u32,
}

/// Wire input for fatal commands: `{ u32 result_code, u32 policy, u64 pid_placeholder }`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ThrowFatalIn {
    pub result_code: u32,
    pub policy: FatalPolicy,
    pub pid_placeholder: u64,
}

const_assert_eq!(size_of::<ThrowFatalIn>(), 0x10);
