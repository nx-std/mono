//! Play Data Manager service (`pdm:qry`) implementation.
//!
//! Provides access to play activity data on the Nintendo Switch, including
//! applet events, play statistics, last play times, and account play events.
//!
//! ## Architecture
//!
//! The service is non-domain. [`connect_cmif`] obtains the root session and
//! all commands operate directly on it.
//!
//! ## Hosversion-aware wire formats
//!
//! Several commands changed wire format across HOS versions:
//!
//! - **AppletEvent**: V1 (1.0.0–15.0.1) uses minute-based timestamps;
//!   current format (16.0.0+) uses POSIX timestamps.
//! - **PlayStatistics**: V1 (1.0.0–15.0.1) uses minute-based timestamps and
//!   playtime-in-minutes; current format (16.0.0+) uses POSIX timestamps and
//!   playtime-in-nanoseconds.
//! - **AccountEvent**: V3 (3.0.0–9.2.0), V10 (10.0.0–15.0.1), current
//!   (16.0.0+) differ in field presence and size.
//!
//! Per IC-4 (hosversion-unaware design), this crate exposes separate method
//! variants for each wire format and lets the caller select based on the
//! target HOS version.
//!
//! ## Divergence from libnx
//!
//! libnx's `pdm.c` auto-detects the HOS version and performs in-place
//! conversion between wire formats (e.g., converting V1 timestamps to POSIX,
//! converting V10 account events to the latest format). This crate does not
//! perform automatic conversion — each method returns exactly what the IPC
//! call produces. Use [`play_timestamp_to_posix`] for manual timestamp
//! conversion from V1 minute-based format.

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

use nx_service_sm::SmService;
use nx_sf::service::{DispatchError, Session};
use nx_svc::ipc::Handle as SessionHandle;

mod cmif;
mod dispatch;
mod proto;
mod types;

pub use self::{
    cmif::GetUpdateEventError,
    proto::SERVICE_NAME,
    types::{
        AccountEvent, AccountEventV3, AccountEventV10, AccountPlayEvent, AccountUid, AppletEvent,
        AppletEventType, AppletEventV1, ApplicationPlayStatistics, LastPlayTime, PlayEvent,
        PlayEventRange, PlayEventType, PlayLogPolicy, PlayStatistics, PlayStatisticsV1,
        play_timestamp_to_posix,
    },
};

/// PDM query service wrapper.
#[repr(transparent)]
pub struct PdmService(Session);

// SAFETY: all operations go through the kernel which serializes
// `svcSendSyncRequest` per session handle.
unsafe impl Send for PdmService {}
unsafe impl Sync for PdmService {}

impl PdmService {
    /// Returns the underlying session handle.
    #[inline]
    pub fn session(&self) -> SessionHandle {
        self.0.handle()
    }

    // -----------------------------------------------------------------------
    // QueryAppletEvent (cmd 0)
    // -----------------------------------------------------------------------

    /// Queries applet events (pre-10.0.0).
    ///
    /// Returns V1 wire-format entries with minute-based timestamps.
    #[inline]
    pub fn query_applet_event_v1_legacy(
        &self,
        entry_index: i32,
        events: &mut [AppletEventV1],
    ) -> Result<i32, DispatchError> {
        cmif::query_applet_event_v1_legacy(&self.0, entry_index, events)
    }

    /// Queries applet events (10.0.0–15.0.1).
    ///
    /// Returns V1 wire-format entries with minute-based timestamps.
    #[inline]
    pub fn query_applet_event_v1(
        &self,
        entry_index: i32,
        flag: bool,
        events: &mut [AppletEventV1],
    ) -> Result<i32, DispatchError> {
        cmif::query_applet_event_v1(&self.0, entry_index, flag, events)
    }

    /// Queries applet events (16.0.0+).
    ///
    /// Returns full-size entries with POSIX timestamps.
    #[inline]
    pub fn query_applet_event(
        &self,
        entry_index: i32,
        flag: bool,
        events: &mut [AppletEvent],
    ) -> Result<i32, DispatchError> {
        cmif::query_applet_event(&self.0, entry_index, flag, events)
    }

    // -----------------------------------------------------------------------
    // QueryPlayStatisticsByApplicationId (cmd 4)
    // -----------------------------------------------------------------------

    /// Queries play statistics by application ID (pre-10.0.0). Returns V1 format.
    #[inline]
    pub fn query_play_statistics_by_app_id_v1_legacy(
        &self,
        application_id: u64,
    ) -> Result<PlayStatisticsV1, DispatchError> {
        cmif::query_play_statistics_by_app_id_v1_legacy(&self.0, application_id)
    }

    /// Queries play statistics by application ID (10.0.0–15.0.1). Returns V1 format.
    #[inline]
    pub fn query_play_statistics_by_app_id_v1(
        &self,
        application_id: u64,
        flag: bool,
    ) -> Result<PlayStatisticsV1, DispatchError> {
        cmif::query_play_statistics_by_app_id_v1(&self.0, application_id, flag)
    }

    /// Queries play statistics by application ID (16.0.0+). Returns full format.
    #[inline]
    pub fn query_play_statistics_by_app_id(
        &self,
        application_id: u64,
        flag: bool,
    ) -> Result<PlayStatistics, DispatchError> {
        cmif::query_play_statistics_by_app_id(&self.0, application_id, flag)
    }

    // -----------------------------------------------------------------------
    // QueryPlayStatisticsByApplicationIdAndUserAccountId (cmd 5)
    // -----------------------------------------------------------------------

    /// Queries play statistics by application ID and user (pre-10.0.0). Returns V1 format.
    #[inline]
    pub fn query_play_statistics_by_app_id_and_user_v1_legacy(
        &self,
        application_id: u64,
        uid: AccountUid,
    ) -> Result<PlayStatisticsV1, DispatchError> {
        cmif::query_play_statistics_by_app_id_and_user_v1_legacy(&self.0, application_id, uid)
    }

    /// Queries play statistics by application ID and user (10.0.0–15.0.1). Returns V1 format.
    #[inline]
    pub fn query_play_statistics_by_app_id_and_user_v1(
        &self,
        application_id: u64,
        uid: AccountUid,
        flag: bool,
    ) -> Result<PlayStatisticsV1, DispatchError> {
        cmif::query_play_statistics_by_app_id_and_user_v1(&self.0, application_id, uid, flag)
    }

    /// Queries play statistics by application ID and user (16.0.0+). Returns full format.
    #[inline]
    pub fn query_play_statistics_by_app_id_and_user(
        &self,
        application_id: u64,
        uid: AccountUid,
        flag: bool,
    ) -> Result<PlayStatistics, DispatchError> {
        cmif::query_play_statistics_by_app_id_and_user(&self.0, application_id, uid, flag)
    }

    // -----------------------------------------------------------------------
    // QueryLastPlayTime (cmd 7 / cmd 17)
    // -----------------------------------------------------------------------

    /// Queries last play time for applications (pre-10.0.0, cmd 7).
    #[inline]
    pub fn query_last_play_time_legacy(
        &self,
        playtimes: &mut [LastPlayTime],
        application_ids: &[u64],
    ) -> Result<i32, DispatchError> {
        cmif::query_last_play_time_legacy(&self.0, playtimes, application_ids)
    }

    /// Queries last play time for applications (10.0.0+, cmd 17).
    #[inline]
    pub fn query_last_play_time(
        &self,
        flag: bool,
        playtimes: &mut [LastPlayTime],
        application_ids: &[u64],
    ) -> Result<i32, DispatchError> {
        cmif::query_last_play_time(&self.0, flag, playtimes, application_ids)
    }

    // -----------------------------------------------------------------------
    // QueryPlayEvent (cmd 8)
    // -----------------------------------------------------------------------

    /// Queries raw play events.
    #[inline]
    pub fn query_play_event(
        &self,
        entry_index: i32,
        events: &mut [PlayEvent],
    ) -> Result<i32, DispatchError> {
        cmif::query_play_event(&self.0, entry_index, events)
    }

    // -----------------------------------------------------------------------
    // GetAvailablePlayEventRange (cmd 9)
    // -----------------------------------------------------------------------

    /// Gets the available play event range.
    #[inline]
    pub fn get_available_play_event_range(&self) -> Result<PlayEventRange, DispatchError> {
        cmif::get_available_play_event_range(&self.0)
    }

    // -----------------------------------------------------------------------
    // QueryAccountEvent (cmd 10)
    // -----------------------------------------------------------------------

    /// Queries account events returning V3 wire format (3.0.0–9.2.0).
    #[inline]
    pub fn query_account_event_v3(
        &self,
        entry_index: i32,
        events: &mut [AccountEventV3],
    ) -> Result<i32, DispatchError> {
        cmif::query_account_event_v3(&self.0, entry_index, events)
    }

    /// Queries account events returning V10 wire format (10.0.0–15.0.1).
    #[inline]
    pub fn query_account_event_v10(
        &self,
        entry_index: i32,
        events: &mut [AccountEventV10],
    ) -> Result<i32, DispatchError> {
        cmif::query_account_event_v10(&self.0, entry_index, events)
    }

    /// Queries account events returning latest wire format (16.0.0+).
    #[inline]
    pub fn query_account_event(
        &self,
        entry_index: i32,
        events: &mut [AccountEvent],
    ) -> Result<i32, DispatchError> {
        cmif::query_account_event(&self.0, entry_index, events)
    }

    // -----------------------------------------------------------------------
    // QueryAccountPlayEvent (cmd 11)
    // -----------------------------------------------------------------------

    /// Queries account play events (4.0.0+).
    #[inline]
    pub fn query_account_play_event(
        &self,
        entry_index: i32,
        uid: AccountUid,
        events: &mut [AccountPlayEvent],
    ) -> Result<i32, DispatchError> {
        cmif::query_account_play_event(&self.0, entry_index, uid, events)
    }

    // -----------------------------------------------------------------------
    // GetAvailableAccountPlayEventRange (cmd 12)
    // -----------------------------------------------------------------------

    /// Gets the available account play event range (4.0.0+).
    #[inline]
    pub fn get_available_account_play_event_range(
        &self,
        uid: AccountUid,
    ) -> Result<PlayEventRange, DispatchError> {
        cmif::get_available_account_play_event_range(&self.0, uid)
    }

    // -----------------------------------------------------------------------
    // QueryRecentlyPlayedApplication (cmd 14)
    // -----------------------------------------------------------------------

    /// Queries recently played applications (6.0.0–9.2.0).
    #[inline]
    pub fn query_recently_played_application_legacy(
        &self,
        uid: AccountUid,
        application_ids: &mut [u64],
    ) -> Result<i32, DispatchError> {
        cmif::query_recently_played_application_legacy(&self.0, uid, application_ids)
    }

    /// Queries recently played applications (10.0.0–14.1.2).
    #[inline]
    pub fn query_recently_played_application(
        &self,
        uid: AccountUid,
        flag: bool,
        application_ids: &mut [u64],
    ) -> Result<i32, DispatchError> {
        cmif::query_recently_played_application(&self.0, uid, flag, application_ids)
    }

    // -----------------------------------------------------------------------
    // GetRecentlyPlayedApplicationUpdateEvent (cmd 15)
    // -----------------------------------------------------------------------

    /// Gets the event signaled on new play events for account event type 0
    /// (6.0.0–14.1.2).
    ///
    /// Returns the raw copy-handle value for the event.
    #[inline]
    pub fn get_recently_played_application_update_event(&self) -> Result<u32, GetUpdateEventError> {
        cmif::get_recently_played_application_update_event(&self.0)
    }
}

/// Connects to the PDM query service (`pdm:qry`) using CMIF.
pub fn connect_cmif(sm: &SmService) -> Result<PdmService, ConnectCmifError> {
    let handle = sm
        .get_service_handle_cmif(SERVICE_NAME)
        .map_err(ConnectCmifError)?;

    let service = Session::from_handle(handle, 0);

    Ok(PdmService(service))
}

/// Error returned by [`connect_cmif`].
#[derive(Debug, thiserror::Error)]
#[error("failed to get pdm:qry service")]
pub struct ConnectCmifError(#[source] pub nx_service_sm::GetServiceCmifError);
