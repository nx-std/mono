//! Wire-layout types for the PGL service.

use core::mem::size_of;

use static_assertions::const_assert_eq;

bitflags::bitflags! {
    /// PGL launch flags controlling crash report behavior.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct PglLaunchFlag: u8 {
        const NONE = 0;
        const ENABLE_DETAILED_CRASH_REPORT = 1 << 0;
        const ENABLE_CRASH_REPORT_SCREENSHOT_FOR_PRODUCTION = 1 << 1;
        const ENABLE_CRASH_REPORT_SCREENSHOT_FOR_DEVELOP = 1 << 2;
    }
}

/// Snapshot dump type for `trigger_application_snapshot_dumper`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SnapShotDumpType {
    None = 0,
    Auto = 1,
    Full = 2,
}

/// Content meta information returned by `get_host_content_meta_info`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ContentMetaInfo {
    pub id: u64,
    pub version: u32,
    pub content_type: u8,
    pub id_offset: u8,
    pub reserved: [u8; 2],
}

const_assert_eq!(size_of::<ContentMetaInfo>(), 0x10);

/// Program location identifying a program by ID and storage.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NcmProgramLocation {
    pub program_id: u64,
    pub storage_id: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<NcmProgramLocation>(), 0x10);

/// Process event type returned by `get_process_event_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProcessEvent {
    None = 0,
    Exit = 1,
    Start = 2,
    Crash = 3,
    DebugStart = 4,
    DebugBreak = 5,
}

/// Process event info returned by the event observer.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessEventInfo {
    pub event: ProcessEvent,
    pub process_id: u64,
}

const_assert_eq!(size_of::<ProcessEventInfo>(), 0x10);

// CMIF-specific input layout for LaunchProgram (cmd 0).
// Field order differs from TIPC: pgl_flags first, then pm_flags, then loc.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LaunchProgramCmifIn {
    pub pgl_flags: PglLaunchFlag,
    pub pad: [u8; 3],
    pub pm_flags: u32,
    pub loc: NcmProgramLocation,
}

const_assert_eq!(size_of::<LaunchProgramCmifIn>(), 0x18);

// TIPC-specific input layout for LaunchProgram (cmd 0).
// Field order differs from CMIF: loc first, then pm_flags, then pgl_flags.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct LaunchProgramTipcIn {
    pub loc: NcmProgramLocation,
    pub pm_flags: u32,
    pub pgl_flags: PglLaunchFlag,
    pub pad: [u8; 3],
}

const_assert_eq!(size_of::<LaunchProgramTipcIn>(), 0x18);
