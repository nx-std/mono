//! CMIF protocol operations for Time service.
//!
//! This module implements Time commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::ptr;

use nx_sf::cmif;
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::{
    proto::{static_service_cmds, system_clock_cmds, timezone_service_cmds},
    types::{TimeCalendarAdditionalInfo, TimeCalendarTime},
};

/// Gets the standard user system clock (ISystemClock).
///
/// This is IStaticService command 0.
pub fn get_standard_user_system_clock(
    session: SessionHandle,
) -> Result<SessionHandle, GetSystemClockError> {
    get_clock_session(session, static_service_cmds::GET_STANDARD_USER_SYSTEM_CLOCK)
}

/// Gets the standard network system clock (ISystemClock).
///
/// This is IStaticService command 1.
pub fn get_standard_network_system_clock(
    session: SessionHandle,
) -> Result<SessionHandle, GetSystemClockError> {
    get_clock_session(
        session,
        static_service_cmds::GET_STANDARD_NETWORK_SYSTEM_CLOCK,
    )
}

/// Gets the standard steady clock (ISteadyClock).
///
/// This is IStaticService command 2.
pub fn get_standard_steady_clock(
    session: SessionHandle,
) -> Result<SessionHandle, GetSteadyClockError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: static_service_cmds::GET_STANDARD_STEADY_CLOCK,
        context: 0,
        data_size: 0,
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSteadyClockError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetSteadyClockError::ParseResponse)?;

    // Extract the move handle from response
    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(GetSteadyClockError::MissingHandle)?;

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the time zone service (ITimeZoneService).
///
/// This is IStaticService command 3.
pub fn get_time_zone_service(
    session: SessionHandle,
) -> Result<SessionHandle, GetTimeZoneServiceError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: static_service_cmds::GET_TIME_ZONE_SERVICE,
        context: 0,
        data_size: 0,
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetTimeZoneServiceError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetTimeZoneServiceError::ParseResponse)?;

    // Extract the move handle from response
    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(GetTimeZoneServiceError::MissingHandle)?;

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the shared memory native handle (6.0.0+).
///
/// This is IStaticService command 20.
pub fn get_shared_memory_native_handle(
    session: SessionHandle,
) -> Result<nx_svc::mem::shmem::Handle, GetSharedMemoryError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: static_service_cmds::GET_SHARED_MEMORY_NATIVE_HANDLE,
        context: 0,
        data_size: 0,
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSharedMemoryError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetSharedMemoryError::ParseResponse)?;

    // Extract the copy handle from response
    let handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetSharedMemoryError::MissingHandle)?;

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { nx_svc::mem::shmem::Handle::from_raw(handle) })
}

/// Gets the current time from a system clock.
///
/// This is ISystemClock command 0.
pub fn get_current_time(session: SessionHandle) -> Result<u64, GetCurrentTimeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: system_clock_cmds::GET_CURRENT_TIME,
        context: 0,
        data_size: 0,
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetCurrentTimeError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetCurrentTimeError::ParseResponse)?;

    // Read u64 timestamp from response data
    // SAFETY: resp.data contains at least 8 bytes for u64.
    let timestamp = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(timestamp)
}

/// Converts a POSIX timestamp to calendar time with the device's timezone rule.
///
/// This is ITimeZoneService command 101.
pub fn to_calendar_time_with_my_rule(
    session: SessionHandle,
    timestamp: u64,
) -> Result<(TimeCalendarTime, TimeCalendarAdditionalInfo), ToCalendarTimeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: timezone_service_cmds::TO_CALENDAR_TIME_WITH_MY_RULE,
        context: 0,
        data_size: 8, // u64 timestamp
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write timestamp
    // SAFETY: req.data points to valid payload area with space for u64.
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), timestamp);
    }

    ipc::send_sync_request(session).map_err(ToCalendarTimeError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(ToCalendarTimeError::ParseResponse)?;

    // Read output structure
    // SAFETY: resp.data contains TimeCalendarTime + TimeCalendarAdditionalInfo.
    #[repr(C)]
    struct Output {
        caltime: TimeCalendarTime,
        info: TimeCalendarAdditionalInfo,
    }

    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    Ok((output.caltime, output.info))
}

/// Helper function to get a clock session (used by user and network system clocks).
fn get_clock_session(
    session: SessionHandle,
    command_id: u32,
) -> Result<SessionHandle, GetSystemClockError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormat {
        object_id: None,
        request_id: command_id,
        context: 0,
        data_size: 0,
        server_pointer_size: 0,
        num_in_auto_buffers: 0,
        num_out_auto_buffers: 0,
        num_in_buffers: 0,
        num_out_buffers: 0,
        num_inout_buffers: 0,
        num_in_pointers: 0,
        num_out_pointers: 0,
        num_out_fixed_pointers: 0,
        num_objects: 0,
        num_handles: 0,
        send_pid: false,
    };

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let _req = unsafe { cmif::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetSystemClockError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(GetSystemClockError::ParseResponse)?;

    // Extract the move handle from response
    let handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(GetSystemClockError::MissingHandle)?;

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by system clock retrieval operations.
#[derive(Debug, thiserror::Error)]
pub enum GetSystemClockError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by steady clock retrieval operation.
#[derive(Debug, thiserror::Error)]
pub enum GetSteadyClockError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by timezone service retrieval operation.
#[derive(Debug, thiserror::Error)]
pub enum GetTimeZoneServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

/// Error returned by shared memory retrieval operation.
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing shared memory handle in response.
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

/// Error returned by get current time operation.
#[derive(Debug, thiserror::Error)]
pub enum GetCurrentTimeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Network clock is not available.
    #[error("network clock is not available")]
    NetworkClockUnavailable,
    /// Local clock is not supported in minimal scope.
    #[error("local clock is not supported")]
    LocalClockNotSupported,
    /// Source ID mismatch in shared memory read.
    #[error("source ID mismatch in shared memory read")]
    SourceIdMismatch,
}

/// Error returned by calendar time conversion operation.
#[derive(Debug, thiserror::Error)]
pub enum ToCalendarTimeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
