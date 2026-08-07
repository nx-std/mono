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
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct FatalAarch64Context {
    pub x: [u64; 32],
    pub pstate: u64,
    pub afsr0: u64,
    pub afsr1: u64,
    pub esr: u64,
    pub far: u64,
    pub stack_trace: [u64; 32],
    pub start_address: u64,
    pub register_set_flags: u64,
    pub stack_trace_size: u32,
    /// Trailing padding to the context's 8-byte alignment. Zero on the wire.
    pub _pad: u32,
}

const_assert_eq!(size_of::<FatalAarch64Context>(), 0x240);

/// AArch32 CPU context for fatal errors.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct FatalAarch32Context {
    pub r: [u32; 16],
    pub pstate: u32,
    pub afsr0: u32,
    pub afsr1: u32,
    pub esr: u32,
    pub far: u32,
    pub stack_trace: [u32; 32],
    pub stack_trace_size: u32,
    pub start_address: u32,
    pub register_set_flags: u32,
}

const_assert_eq!(size_of::<FatalAarch32Context>(), 0xE0);

/// Combined CPU context for fatal errors.
///
/// Contains either an AArch64 or AArch32 context, discriminated by
/// `is_aarch32`. The `context_type` field identifies the exception type.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct FatalCpuContext {
    pub ctx: FatalCpuContextUnion,
    pub is_aarch32: bool,
    pub _pad: [u8; 3],
    pub context_type: u32,
}

const_assert_eq!(size_of::<FatalCpuContext>(), 0x248);

/// Union of AArch64 and AArch32 CPU contexts.
#[derive(Clone, Copy)]
#[repr(C)]
pub union FatalCpuContextUnion {
    pub aarch64: FatalAarch64Context,
    pub aarch32: FatalAarch32Context,
}

const_assert_eq!(size_of::<FatalCpuContextUnion>(), 0x240);

/// Wire input for fatal commands: `{ u32 result_code, u32 policy, u64 pid_placeholder }`.
#[derive(Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(C)]
pub(crate) struct ThrowFatalIn {
    pub result_code: u32,
    pub policy: FatalPolicy,
    pub pid_placeholder: u64,
}

const_assert_eq!(size_of::<ThrowFatalIn>(), 0x10);
