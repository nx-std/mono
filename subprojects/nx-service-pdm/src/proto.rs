//! PDM query service protocol constants.

use nx_sf::ServiceName;

/// Service name for the query interface (`pdm:qry`).
pub const SERVICE_NAME: ServiceName = ServiceName::new_truncate("pdm:qry");

// ---------------------------------------------------------------------------
// pdm:qry commands
// ---------------------------------------------------------------------------

/// Gets a list of applet events.
///
/// Pre-10.0.0: input is `s32 entry_index`, output buffer is `PdmAppletEvent`.
/// 10.0.0+: input is `(u8 flag, pad[3], s32 entry_index)`, output buffer is
/// `PdmAppletEventV1` (pre-16.0.0) or `PdmAppletEvent` (16.0.0+).
pub const QUERY_APPLET_EVENT: u32 = 0;

/// Gets play statistics by application ID.
///
/// Pre-10.0.0: input is `u64 application_id`, output is `PdmPlayStatisticsV1`.
/// 10.0.0+: input is `(u8 flag, pad[7], u64 application_id)`, output is
/// `PdmPlayStatisticsV1` (pre-16.0.0) or `PdmPlayStatistics` (16.0.0+).
pub const QUERY_PLAY_STATISTICS_BY_APPLICATION_ID: u32 = 4;

/// Gets play statistics by application ID and user account ID.
///
/// Pre-10.0.0: input is `(u64 application_id, AccountUid)`.
/// 10.0.0+: input is `(u8 flag, pad[7], u64 application_id, AccountUid)`.
pub const QUERY_PLAY_STATISTICS_BY_APPLICATION_ID_AND_USER: u32 = 5;

/// Gets last play time for specified applications (pre-10.0.0).
pub const QUERY_LAST_PLAY_TIME_LEGACY: u32 = 7;

/// Gets a list of raw play events.
pub const QUERY_PLAY_EVENT: u32 = 8;

/// Gets the available play event range.
pub const GET_AVAILABLE_PLAY_EVENT_RANGE: u32 = 9;

/// Gets a list of account events (3.0.0+).
pub const QUERY_ACCOUNT_EVENT: u32 = 10;

/// Gets a list of account play events (4.0.0+).
pub const QUERY_ACCOUNT_PLAY_EVENT: u32 = 11;

/// Gets the available account play event range (4.0.0+).
pub const GET_AVAILABLE_ACCOUNT_PLAY_EVENT_RANGE: u32 = 12;

/// Gets recently played applications by user (6.0.0–14.1.2).
pub const QUERY_RECENTLY_PLAYED_APPLICATION: u32 = 14;

/// Gets event signaled on new play events for account event type 0 (6.0.0–14.1.2).
pub const GET_RECENTLY_PLAYED_APPLICATION_UPDATE_EVENT: u32 = 15;

/// Gets last play time for specified applications (10.0.0+).
pub const QUERY_LAST_PLAY_TIME: u32 = 17;
