//! Wire-layout types for the process manager service.

use core::mem::size_of;

use static_assertions::const_assert_eq;

bitflags::bitflags! {
    /// Launch flags for `pm:shell` `LaunchProgram` (`[5.0.0+]`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct PmLaunchFlag: u32 {
        const NONE = 0;
        const SIGNAL_ON_EXIT = 1 << 0;
        const SIGNAL_ON_START = 1 << 1;
        const SIGNAL_ON_CRASH = 1 << 2;
        const SIGNAL_ON_DEBUG = 1 << 3;
        const START_SUSPENDED = 1 << 4;
        const DISABLE_ASLR = 1 << 5;
    }
}

bitflags::bitflags! {
    /// Launch flags for `pm:shell` `LaunchProgram` (pre-5.0.0).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct PmLaunchFlagOld: u32 {
        const NONE = 0;
        const SIGNAL_ON_EXIT = 1 << 0;
        const START_SUSPENDED = 1 << 1;
        const SIGNAL_ON_CRASH = 1 << 2;
        const DISABLE_ASLR = 1 << 3;
        const SIGNAL_ON_DEBUG = 1 << 4;
        /// Only available on `[2.0.0+]`.
        const SIGNAL_ON_START = 1 << 5;
    }
}

/// Process event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PmProcessEvent {
    None = 0,
    Exit = 1,
    Start = 2,
    Crash = 3,
    DebugStart = 4,
    DebugBreak = 5,
}

/// Process event info returned by `pm:shell` `GetProcessEventInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PmProcessEventInfo {
    pub event: PmProcessEvent,
    pub process_id: u64,
}

const_assert_eq!(size_of::<PmProcessEventInfo>(), 0x10);

/// Boot mode returned by `pm:bm` `GetBootMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PmBootMode {
    Normal = 0,
    Maintenance = 1,
    SafeMode = 2,
}

/// Resource limit values returned by `pm:info` `GetApplet*ResourceLimitValues`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PmResourceLimitValues {
    pub physical_memory: u64,
    pub thread_count: u32,
    pub event_count: u32,
    pub transfer_memory_count: u32,
    pub session_count: u32,
}

const_assert_eq!(size_of::<PmResourceLimitValues>(), 0x18);

/// Program location identifying a program by ID and storage.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NcmProgramLocation {
    pub program_id: u64,
    pub storage_id: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<NcmProgramLocation>(), 0x10);

/// Input for `pm:shell` `LaunchProgram`.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LaunchProgramIn {
    pub launch_flags: u32,
    pub pad: u32,
    pub location: NcmProgramLocation,
}

const_assert_eq!(size_of::<LaunchProgramIn>(), 0x18);
