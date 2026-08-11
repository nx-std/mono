//! Wire-layout types for the PGL service.

#![expect(
    clippy::identity_op,
    reason = "the `#[bitfield]` accessors compute each field's bit offset as a `0 + ..` sum, \
              so the lint fires inside generated code this module does not write"
)]

use modular_bitfield::specifiers::B5;
use static_assertions::const_assert_eq;

/// PGL launch flags controlling crash report behavior.
///
/// A default value has every flag clear, which is what the service treats as
/// "no special crash-report handling".
#[modular_bitfield::bitfield]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    zerocopy::FromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
#[repr(C)]
pub struct PglLaunchFlag {
    /// Collect a detailed crash report for the launched program.
    pub enable_detailed_crash_report: bool,
    /// Capture a crash-report screenshot on retail units.
    pub enable_crash_report_screenshot_for_production: bool,
    /// Capture a crash-report screenshot on development units.
    pub enable_crash_report_screenshot_for_develop: bool,
    #[skip]
    __: B5,
}

const_assert_eq!(size_of::<PglLaunchFlag>(), 0x1);

/// Snapshot dump type for `trigger_application_snapshot_dumper`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[repr(u32)]
pub enum SnapShotDumpType {
    None = 0,
    Auto = 1,
    Full = 2,
}

/// Content meta information returned by `get_host_content_meta_info`.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
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
#[derive(Debug, Clone, Copy, zerocopy::IntoBytes, zerocopy::Immutable)]
pub struct NcmProgramLocation {
    pub program_id: u64,
    pub storage_id: u8,
    pub pad: [u8; 7],
}

const_assert_eq!(size_of::<NcmProgramLocation>(), 0x10);

/// Process event type returned by `get_process_event_info`.
///
/// Modeled as a newtype around `u32` rather than a `#[repr(u32)]` enum so
/// the wire bytes can be zero-copy-parsed without UB risk on an unknown
/// discriminant: any `u32` is a valid bit-pattern; callers compare against
/// the associated constants below to discriminate.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    zerocopy::FromBytes,
    zerocopy::Immutable,
    zerocopy::KnownLayout,
)]
pub struct ProcessEvent(pub u32);

impl ProcessEvent {
    pub const NONE: Self = Self(0);
    pub const EXIT: Self = Self(1);
    pub const START: Self = Self(2);
    pub const CRASH: Self = Self(3);
    pub const DEBUG_START: Self = Self(4);
    pub const DEBUG_BREAK: Self = Self(5);
}

/// Process event info returned by the event observer.
///
/// Wire layout: `{ u32 event, u32 pad, u64 process_id }`.
// The padding is left to `#[repr(C)]` rather than spelled as a field: this type
// is only ever decoded from a response, so no uninitialised byte can escape
// through it. Do not add `IntoBytes` here without first giving the gap a field.
#[repr(C)]
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
pub struct ProcessEventInfo {
    pub event: ProcessEvent,
    pub process_id: u64,
}

const_assert_eq!(size_of::<ProcessEventInfo>(), 0x10);
