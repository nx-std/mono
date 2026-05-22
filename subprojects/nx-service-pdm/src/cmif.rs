//! CMIF protocol operations for the PDM query service.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, Session};

use crate::{
    dispatch::{dispatch_in_out, dispatch_out},
    proto,
    types::{
        AccountEvent, AccountEventV3, AccountEventV10, AccountPlayEvent, AccountUid, AppletEvent,
        AppletEventV1, LastPlayTime, PlayEvent, PlayEventRange, PlayEventRangeOut, PlayStatistics,
        PlayStatisticsV1, QueryAccountPlayEventIn, QueryAppletEventIn, QueryAppletEventLegacyIn,
        QueryLastPlayTimeIn, QueryPlayStatsByAppIdAndUserIn, QueryPlayStatsByAppIdAndUserLegacyIn,
        QueryPlayStatsByAppIdIn, QueryPlayStatsByAppIdLegacyIn, QueryRecentlyPlayedAppIn,
        QueryRecentlyPlayedAppLegacyIn,
    },
};

// ---------------------------------------------------------------------------
// QueryAppletEvent (cmd 0)
// ---------------------------------------------------------------------------

/// Queries applet events (pre-10.0.0).
///
/// Returns V1 wire-format entries in the provided buffer (timestamps are
/// minute-based). Use [`play_timestamp_to_posix`](crate::play_timestamp_to_posix)
/// to convert timestamps.
pub(crate) fn query_applet_event_v1_legacy(
    service: &Session,
    entry_index: i32,
    events: &mut [AppletEventV1],
) -> Result<i32, DispatchError> {
    let input = QueryAppletEventLegacyIn { entry_index };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<QueryAppletEventLegacyIn>(),
        )
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_APPLET_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Queries applet events (10.0.0–15.0.1).
///
/// Returns V1 wire-format entries.
pub(crate) fn query_applet_event_v1(
    service: &Session,
    entry_index: i32,
    flag: bool,
    events: &mut [AppletEventV1],
) -> Result<i32, DispatchError> {
    let input = QueryAppletEventIn {
        flag: u8::from(flag),
        pad: [0; 3],
        entry_index,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<QueryAppletEventIn>(),
        )
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_APPLET_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Queries applet events (16.0.0+).
///
/// Returns full-size `AppletEvent` entries with POSIX timestamps.
pub(crate) fn query_applet_event(
    service: &Session,
    entry_index: i32,
    flag: bool,
    events: &mut [AppletEvent],
) -> Result<i32, DispatchError> {
    let input = QueryAppletEventIn {
        flag: u8::from(flag),
        pad: [0; 3],
        entry_index,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<QueryAppletEventIn>(),
        )
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_APPLET_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// ---------------------------------------------------------------------------
// QueryPlayStatisticsByApplicationId (cmd 4)
// ---------------------------------------------------------------------------

/// Queries play statistics by application ID (pre-10.0.0). Returns V1 format.
pub(crate) fn query_play_statistics_by_app_id_v1_legacy(
    service: &Session,
    application_id: u64,
) -> Result<PlayStatisticsV1, DispatchError> {
    let input = QueryPlayStatsByAppIdLegacyIn { application_id };
    dispatch_in_out(
        service,
        proto::QUERY_PLAY_STATISTICS_BY_APPLICATION_ID,
        input,
    )
}

/// Queries play statistics by application ID (10.0.0–15.0.1). Returns V1 format.
pub(crate) fn query_play_statistics_by_app_id_v1(
    service: &Session,
    application_id: u64,
    flag: bool,
) -> Result<PlayStatisticsV1, DispatchError> {
    let input = QueryPlayStatsByAppIdIn {
        flag: u8::from(flag),
        pad: [0; 7],
        application_id,
    };
    dispatch_in_out(
        service,
        proto::QUERY_PLAY_STATISTICS_BY_APPLICATION_ID,
        input,
    )
}

/// Queries play statistics by application ID (16.0.0+). Returns full format.
pub(crate) fn query_play_statistics_by_app_id(
    service: &Session,
    application_id: u64,
    flag: bool,
) -> Result<PlayStatistics, DispatchError> {
    let input = QueryPlayStatsByAppIdIn {
        flag: u8::from(flag),
        pad: [0; 7],
        application_id,
    };
    dispatch_in_out(
        service,
        proto::QUERY_PLAY_STATISTICS_BY_APPLICATION_ID,
        input,
    )
}

// ---------------------------------------------------------------------------
// QueryPlayStatisticsByApplicationIdAndUserAccountId (cmd 5)
// ---------------------------------------------------------------------------

/// Queries play statistics by application ID and user (pre-10.0.0). Returns V1 format.
pub(crate) fn query_play_statistics_by_app_id_and_user_v1_legacy(
    service: &Session,
    application_id: u64,
    uid: AccountUid,
) -> Result<PlayStatisticsV1, DispatchError> {
    let input = QueryPlayStatsByAppIdAndUserLegacyIn {
        application_id,
        uid,
    };
    dispatch_in_out(
        service,
        proto::QUERY_PLAY_STATISTICS_BY_APPLICATION_ID_AND_USER,
        input,
    )
}

/// Queries play statistics by application ID and user (10.0.0–15.0.1). Returns V1 format.
pub(crate) fn query_play_statistics_by_app_id_and_user_v1(
    service: &Session,
    application_id: u64,
    uid: AccountUid,
    flag: bool,
) -> Result<PlayStatisticsV1, DispatchError> {
    let input = QueryPlayStatsByAppIdAndUserIn {
        flag: u8::from(flag),
        pad: [0; 7],
        application_id,
        uid,
    };
    dispatch_in_out(
        service,
        proto::QUERY_PLAY_STATISTICS_BY_APPLICATION_ID_AND_USER,
        input,
    )
}

/// Queries play statistics by application ID and user (16.0.0+). Returns full format.
pub(crate) fn query_play_statistics_by_app_id_and_user(
    service: &Session,
    application_id: u64,
    uid: AccountUid,
    flag: bool,
) -> Result<PlayStatistics, DispatchError> {
    let input = QueryPlayStatsByAppIdAndUserIn {
        flag: u8::from(flag),
        pad: [0; 7],
        application_id,
        uid,
    };
    dispatch_in_out(
        service,
        proto::QUERY_PLAY_STATISTICS_BY_APPLICATION_ID_AND_USER,
        input,
    )
}

// ---------------------------------------------------------------------------
// QueryLastPlayTime (cmd 7 / cmd 17)
// ---------------------------------------------------------------------------

/// Queries last play time (pre-10.0.0, cmd 7).
pub(crate) fn query_last_play_time_legacy(
    service: &Session,
    playtimes: &mut [LastPlayTime],
    application_ids: &[u64],
) -> Result<i32, DispatchError> {
    let count = playtimes.len().min(application_ids.len());
    // SAFETY: `playtimes` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `playtimes`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            playtimes.as_mut_ptr().cast::<u8>(),
            count * size_of::<LastPlayTime>(),
        )
    };
    // SAFETY: `application_ids` is a valid `&` slice; viewing it as a byte
    // slice for the IN buffer is sound.
    let in_buf_bytes = unsafe {
        core::slice::from_raw_parts(
            application_ids.as_ptr().cast::<u8>(),
            count * size_of::<u64>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_LAST_PLAY_TIME_LEGACY)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(in_buf_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Queries last play time (10.0.0+, cmd 17).
pub(crate) fn query_last_play_time(
    service: &Session,
    flag: bool,
    playtimes: &mut [LastPlayTime],
    application_ids: &[u64],
) -> Result<i32, DispatchError> {
    let count = playtimes.len().min(application_ids.len());
    let input = QueryLastPlayTimeIn {
        flag: u8::from(flag),
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<QueryLastPlayTimeIn>(),
        )
    };
    // SAFETY: `playtimes` is a valid `&mut` slice; viewing it as a byte slice
    // for the OUT buffer is sound, and the byte slice borrows `playtimes`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            playtimes.as_mut_ptr().cast::<u8>(),
            count * size_of::<LastPlayTime>(),
        )
    };
    // SAFETY: `application_ids` is a valid `&` slice; viewing it as a byte
    // slice for the IN buffer is sound.
    let in_buf_bytes = unsafe {
        core::slice::from_raw_parts(
            application_ids.as_ptr().cast::<u8>(),
            count * size_of::<u64>(),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_LAST_PLAY_TIME)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(in_buf_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// ---------------------------------------------------------------------------
// QueryPlayEvent (cmd 8)
// ---------------------------------------------------------------------------

/// Queries raw play events.
pub(crate) fn query_play_event(
    service: &Session,
    entry_index: i32,
    events: &mut [PlayEvent],
) -> Result<i32, DispatchError> {
    // SAFETY: `entry_index` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const entry_index).cast::<u8>(), size_of::<i32>())
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_PLAY_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// ---------------------------------------------------------------------------
// GetAvailablePlayEventRange (cmd 9)
// ---------------------------------------------------------------------------

/// Gets the available play event range.
pub(crate) fn get_available_play_event_range(
    service: &Session,
) -> Result<PlayEventRange, DispatchError> {
    let out: PlayEventRangeOut = dispatch_out(service, proto::GET_AVAILABLE_PLAY_EVENT_RANGE)?;
    Ok(PlayEventRange {
        total_entries: out.total_entries,
        start_entry_index: out.start_entry_index,
        end_entry_index: out.end_entry_index,
    })
}

// ---------------------------------------------------------------------------
// QueryAccountEvent (cmd 10)
// ---------------------------------------------------------------------------

/// Queries account events returning V3 wire format (3.0.0–9.2.0).
pub(crate) fn query_account_event_v3(
    service: &Session,
    entry_index: i32,
    events: &mut [AccountEventV3],
) -> Result<i32, DispatchError> {
    // SAFETY: `entry_index` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const entry_index).cast::<u8>(), size_of::<i32>())
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_ACCOUNT_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Queries account events returning V10 wire format (10.0.0–15.0.1).
pub(crate) fn query_account_event_v10(
    service: &Session,
    entry_index: i32,
    events: &mut [AccountEventV10],
) -> Result<i32, DispatchError> {
    // SAFETY: `entry_index` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const entry_index).cast::<u8>(), size_of::<i32>())
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_ACCOUNT_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Queries account events returning latest wire format (16.0.0+).
pub(crate) fn query_account_event(
    service: &Session,
    entry_index: i32,
    events: &mut [AccountEvent],
) -> Result<i32, DispatchError> {
    // SAFETY: `entry_index` is a `Copy` value on the stack, valid until
    // `.send()` returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts((&raw const entry_index).cast::<u8>(), size_of::<i32>())
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_ACCOUNT_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// ---------------------------------------------------------------------------
// QueryAccountPlayEvent (cmd 11)
// ---------------------------------------------------------------------------

/// Queries account play events (4.0.0+).
pub(crate) fn query_account_play_event(
    service: &Session,
    entry_index: i32,
    uid: AccountUid,
    events: &mut [AccountPlayEvent],
) -> Result<i32, DispatchError> {
    let input = QueryAccountPlayEventIn {
        entry_index,
        pad: 0,
        uid,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<QueryAccountPlayEventIn>(),
        )
    };
    // SAFETY: `events` is a valid `&mut` slice; viewing it as a byte slice for
    // the OUT buffer is sound, and the byte slice borrows `events`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            events.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(events),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_ACCOUNT_PLAY_EVENT)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// ---------------------------------------------------------------------------
// GetAvailableAccountPlayEventRange (cmd 12)
// ---------------------------------------------------------------------------

/// Gets the available account play event range (4.0.0+).
pub(crate) fn get_available_account_play_event_range(
    service: &Session,
    uid: AccountUid,
) -> Result<PlayEventRange, DispatchError> {
    let out: PlayEventRangeOut =
        dispatch_in_out(service, proto::GET_AVAILABLE_ACCOUNT_PLAY_EVENT_RANGE, uid)?;
    Ok(PlayEventRange {
        total_entries: out.total_entries,
        start_entry_index: out.start_entry_index,
        end_entry_index: out.end_entry_index,
    })
}

// ---------------------------------------------------------------------------
// QueryRecentlyPlayedApplication (cmd 14)
// ---------------------------------------------------------------------------

/// Queries recently played applications (6.0.0–9.2.0).
pub(crate) fn query_recently_played_application_legacy(
    service: &Session,
    uid: AccountUid,
    application_ids: &mut [u64],
) -> Result<i32, DispatchError> {
    let input = QueryRecentlyPlayedAppLegacyIn { uid };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<QueryRecentlyPlayedAppLegacyIn>(),
        )
    };
    // SAFETY: `application_ids` is a valid `&mut` slice; viewing it as a byte
    // slice for the OUT buffer is sound, and the byte slice borrows
    // `application_ids`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            application_ids.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(application_ids),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_RECENTLY_PLAYED_APPLICATION)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Queries recently played applications (10.0.0–14.1.2).
pub(crate) fn query_recently_played_application(
    service: &Session,
    uid: AccountUid,
    flag: bool,
    application_ids: &mut [u64],
) -> Result<i32, DispatchError> {
    let input = QueryRecentlyPlayedAppIn {
        flag: u8::from(flag),
        pad: [0; 7],
        uid,
    };
    // SAFETY: `input` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes = unsafe {
        core::slice::from_raw_parts(
            (&raw const input).cast::<u8>(),
            size_of::<QueryRecentlyPlayedAppIn>(),
        )
    };
    // SAFETY: `application_ids` is a valid `&mut` slice; viewing it as a byte
    // slice for the OUT buffer is sound, and the byte slice borrows
    // `application_ids`.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            application_ids.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(application_ids),
        )
    };
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::QUERY_RECENTLY_PLAYED_APPLICATION)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(i32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

// ---------------------------------------------------------------------------
// GetRecentlyPlayedApplicationUpdateEvent (cmd 15)
// ---------------------------------------------------------------------------

/// Gets the event for recently-played application updates (6.0.0–14.1.2).
///
/// Returns the raw copy-handle value for the event.
pub(crate) fn get_recently_played_application_update_event(
    service: &Session,
) -> Result<u32, GetUpdateEventError> {
    let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let result = service
        .dispatch(proto::GET_RECENTLY_PLAYED_APPLICATION_UPDATE_EVENT)
        .send(&mut ipc_buf)
        .map_err(GetUpdateEventError::Dispatch)?;

    if result.copy_handles.is_empty() {
        return Err(GetUpdateEventError::MissingHandle);
    }
    Ok(result.copy_handles[0])
}

/// Error returned by [`get_recently_played_application_update_event`].
#[derive(Debug, thiserror::Error)]
pub enum GetUpdateEventError {
    /// IPC dispatch failed.
    #[error("failed to dispatch GetRecentlyPlayedApplicationUpdateEvent")]
    Dispatch(#[source] DispatchError),
    /// Response did not include the expected copy handle.
    #[error("GetRecentlyPlayedApplicationUpdateEvent response missing copy handle")]
    MissingHandle,
}
