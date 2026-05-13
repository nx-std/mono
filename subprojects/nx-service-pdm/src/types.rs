//! PDM wire-layout types.

use static_assertions::const_assert_eq;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Play event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayEventType {
    Applet = 0,
    Account = 1,
    PowerStateChange = 2,
    OperationModeChange = 3,
    Initialize = 4,
}

/// Applet event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AppletEventType {
    Launch = 0,
    Exit = 1,
    InFocus = 2,
    OutOfFocus = 3,
    OutOfFocus4 = 4,
    Exit5 = 5,
    Exit6 = 6,
}

/// Play log policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayLogPolicy {
    All = 0,
    LogOnly = 1,
    None = 2,
    Unknown3 = 3,
}

// ---------------------------------------------------------------------------
// Applet event structs
// ---------------------------------------------------------------------------

/// Applet event for 1.0.0–15.0.1.
///
/// Timestamps are total minutes since epoch UTC 1999/12/31 00:00:00.
/// Use [`play_timestamp_to_posix`] to convert.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AppletEventV1 {
    pub program_id: u64,
    pub entry_index: u32,
    pub timestamp_user: u32,
    pub timestamp_network: u32,
    pub event_type: u8,
    pub pad: [u8; 3],
}

const_assert_eq!(size_of::<AppletEventV1>(), 0x18);

/// Applet event for 16.0.0+.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AppletEvent {
    pub program_id: u64,
    pub entry_index: u32,
    pub pad: u32,
    pub timestamp_user: u64,
    pub timestamp_network: u64,
    pub event_type: u8,
    pub pad2: [u8; 7],
}

const_assert_eq!(size_of::<AppletEvent>(), 0x28);

// ---------------------------------------------------------------------------
// Play statistics structs
// ---------------------------------------------------------------------------

/// Play statistics for 1.0.0–15.0.1.
///
/// Timestamps use the same minute-based format as [`AppletEventV1`].
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PlayStatisticsV1 {
    pub program_id: u64,
    pub first_entry_index: u32,
    pub first_timestamp_user: u32,
    pub first_timestamp_network: u32,
    pub last_entry_index: u32,
    pub last_timestamp_user: u32,
    pub last_timestamp_network: u32,
    pub playtime_minutes: u32,
    pub total_launches: u32,
}

const_assert_eq!(size_of::<PlayStatisticsV1>(), 0x28);

/// Play statistics for 16.0.0+.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PlayStatistics {
    pub program_id: u64,
    pub first_entry_index: u32,
    pub pad: u32,
    pub first_timestamp_user: u64,
    pub first_timestamp_network: u64,
    pub last_entry_index: u32,
    pub pad2: u32,
    pub last_timestamp_user: u64,
    pub last_timestamp_network: u64,
    pub playtime: u64,
    pub total_launches: u32,
    pub pad3: u32,
}

const_assert_eq!(size_of::<PlayStatistics>(), 0x48);

// ---------------------------------------------------------------------------
// Last play time
// ---------------------------------------------------------------------------

/// Last play time for an application.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LastPlayTime {
    pub application_id: u64,
    pub timestamp_user: u32,
    pub timestamp_network: u32,
    pub last_played_minutes: u32,
    pub flag: u8,
    pub pad: [u8; 3],
}

const_assert_eq!(size_of::<LastPlayTime>(), 0x18);

// ---------------------------------------------------------------------------
// Play event (raw)
// ---------------------------------------------------------------------------

/// Raw play event entry read from FS.
///
/// The `event_data` union is 0x1C bytes. The `play_event_type` field
/// determines which variant is active.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PlayEvent {
    pub event_data: [u8; 0x1C],
    pub play_event_type: u8,
    pub pad: [u8; 3],
    pub timestamp_user: u64,
    pub timestamp_network: u64,
    pub timestamp_steady: u64,
}

const_assert_eq!(size_of::<PlayEvent>(), 0x38);

// ---------------------------------------------------------------------------
// Account event structs
// ---------------------------------------------------------------------------

/// Account user ID (matches libnx `AccountUid`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct AccountUid {
    pub uid: [u64; 2],
}

const_assert_eq!(size_of::<AccountUid>(), 0x10);

/// Account event for 3.0.0–9.2.0.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountEventV3 {
    pub uid: AccountUid,
    pub entry_index: u32,
    pub pad: [u8; 4],
    pub timestamp_user: u64,
    pub timestamp_network: u64,
    pub timestamp_steady: u64,
    pub event_type: u8,
    pub pad2: [u8; 7],
}

const_assert_eq!(size_of::<AccountEventV3>(), 0x38);

/// Account event for 10.0.0–15.0.1.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountEventV10 {
    pub uid: AccountUid,
    pub program_id: u64,
    pub entry_index: u32,
    pub pad: [u8; 4],
    pub timestamp_user: u64,
    pub timestamp_network: u64,
    pub timestamp_steady: u64,
    pub event_type: u8,
    pub pad2: [u8; 7],
}

const_assert_eq!(size_of::<AccountEventV10>(), 0x40);

/// Account event for 16.0.0+.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountEvent {
    pub uid: AccountUid,
    pub program_id: u64,
    pub entry_index: u32,
    pub pad: [u8; 4],
    pub timestamp_user: u64,
    pub timestamp_network: u64,
    pub event_type: u8,
    pub pad2: [u8; 7],
}

const_assert_eq!(size_of::<AccountEvent>(), 0x38);

// ---------------------------------------------------------------------------
// Account play event (raw)
// ---------------------------------------------------------------------------

/// Raw account play event entry (4.0.0+).
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AccountPlayEvent {
    pub unk_x0: [u8; 4],
    pub application_id: [u32; 2],
    pub unk_xc: [u8; 0xC],
    pub timestamp0: u64,
    pub timestamp1: u64,
}

const_assert_eq!(size_of::<AccountPlayEvent>(), 0x28);

/// Application play statistics.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ApplicationPlayStatistics {
    pub application_id: u64,
    pub playtime: u64,
    pub total_launches: u64,
}

const_assert_eq!(size_of::<ApplicationPlayStatistics>(), 0x18);

// ---------------------------------------------------------------------------
// Wire-layout input structs
// ---------------------------------------------------------------------------

/// Input for `QueryAppletEvent` (pre-10.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryAppletEventLegacyIn {
    pub entry_index: i32,
}

const_assert_eq!(size_of::<QueryAppletEventLegacyIn>(), 0x04);

/// Input for `QueryAppletEvent` (10.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryAppletEventIn {
    pub flag: u8,
    pub pad: [u8; 3],
    pub entry_index: i32,
}

const_assert_eq!(size_of::<QueryAppletEventIn>(), 0x08);

/// Input for `QueryPlayStatisticsByApplicationId` (pre-10.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryPlayStatsByAppIdLegacyIn {
    pub application_id: u64,
}

const_assert_eq!(size_of::<QueryPlayStatsByAppIdLegacyIn>(), 0x08);

/// Input for `QueryPlayStatisticsByApplicationId` (10.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryPlayStatsByAppIdIn {
    pub flag: u8,
    pub pad: [u8; 7],
    pub application_id: u64,
}

const_assert_eq!(size_of::<QueryPlayStatsByAppIdIn>(), 0x10);

/// Input for `QueryPlayStatisticsByApplicationIdAndUserAccountId` (pre-10.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryPlayStatsByAppIdAndUserLegacyIn {
    pub application_id: u64,
    pub uid: AccountUid,
}

const_assert_eq!(size_of::<QueryPlayStatsByAppIdAndUserLegacyIn>(), 0x18);

/// Input for `QueryPlayStatisticsByApplicationIdAndUserAccountId` (10.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryPlayStatsByAppIdAndUserIn {
    pub flag: u8,
    pub pad: [u8; 7],
    pub application_id: u64,
    pub uid: AccountUid,
}

const_assert_eq!(size_of::<QueryPlayStatsByAppIdAndUserIn>(), 0x20);

/// Input for `QueryLastPlayTime` (10.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryLastPlayTimeIn {
    pub flag: u8,
}

const_assert_eq!(size_of::<QueryLastPlayTimeIn>(), 0x01);

/// Input for `QueryAccountPlayEvent` (4.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryAccountPlayEventIn {
    pub entry_index: i32,
    pub pad: u32,
    pub uid: AccountUid,
}

const_assert_eq!(size_of::<QueryAccountPlayEventIn>(), 0x18);

/// Input for `QueryRecentlyPlayedApplication` (pre-10.0.0).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryRecentlyPlayedAppLegacyIn {
    pub uid: AccountUid,
}

const_assert_eq!(size_of::<QueryRecentlyPlayedAppLegacyIn>(), 0x10);

/// Input for `QueryRecentlyPlayedApplication` (10.0.0+).
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct QueryRecentlyPlayedAppIn {
    pub flag: u8,
    pub pad: [u8; 7],
    pub uid: AccountUid,
}

const_assert_eq!(size_of::<QueryRecentlyPlayedAppIn>(), 0x18);

/// Output for `GetAvailablePlayEventRange` / `GetAvailableAccountPlayEventRange`.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct PlayEventRangeOut {
    pub total_entries: i32,
    pub start_entry_index: i32,
    pub end_entry_index: i32,
}

const_assert_eq!(size_of::<PlayEventRangeOut>(), 0x0C);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Converts a Play timestamp (total minutes since 1999/12/31 00:00:00 UTC) to
/// POSIX seconds.
#[inline]
pub const fn play_timestamp_to_posix(timestamp: u32) -> u64 {
    (timestamp as u64) * 60 + 946_598_400
}

/// Available play-event range returned by [`PdmService::get_available_play_event_range`]
/// and [`PdmService::get_available_account_play_event_range`].
#[derive(Debug, Clone, Copy)]
pub struct PlayEventRange {
    pub total_entries: i32,
    pub start_entry_index: i32,
    pub end_entry_index: i32,
}
