//! CMIF protocol operations for Time service.
//!
//! This module implements Time commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use nx_sf::{
    cmif,
    error::{GENERIC_ERROR, ResultCode, ToResultCode},
    ipc::Handle as SessionHandle,
};

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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(static_service_cmds::GET_STANDARD_STEADY_CLOCK).build();
    req.send(&mut buf, session)
        .map_err(GetSteadyClockError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetSteadyClockError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetSteadyClockError::MissingHandle);
    };

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the time zone service (ITimeZoneService).
///
/// This is IStaticService command 3.
pub fn get_time_zone_service(
    session: SessionHandle,
) -> Result<SessionHandle, GetTimeZoneServiceError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(static_service_cmds::GET_TIME_ZONE_SERVICE).build();
    req.send(&mut buf, session)
        .map_err(GetTimeZoneServiceError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetTimeZoneServiceError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetTimeZoneServiceError::MissingHandle);
    };

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Gets the shared memory native handle (6.0.0+).
///
/// This is IStaticService command 20.
pub fn get_shared_memory_native_handle(
    session: SessionHandle,
) -> Result<nx_svc::mem::shmem::Handle, GetSharedMemoryError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req =
        cmif::CmifRequestBuilder::new(static_service_cmds::GET_SHARED_MEMORY_NATIVE_HANDLE).build();
    req.send(&mut buf, session)
        .map_err(GetSharedMemoryError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetSharedMemoryError::ParseResponse)?;

    let Some(&handle) = resp.copy_handles.first() else {
        return Err(GetSharedMemoryError::MissingHandle);
    };

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { nx_svc::mem::shmem::Handle::from_raw(handle) })
}

/// Gets the current time from a system clock.
///
/// This is ISystemClock command 0.
pub fn get_current_time(session: SessionHandle) -> Result<u64, GetCurrentTimeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(system_clock_cmds::GET_CURRENT_TIME).build();
    req.send(&mut buf, session)
        .map_err(GetCurrentTimeError::SendRequest)?;

    let resp = cmif::parse_response::<&u64>(&buf).map_err(GetCurrentTimeError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Converts a POSIX timestamp to calendar time with the device's timezone rule.
///
/// This is ITimeZoneService command 101.
pub fn to_calendar_time_with_my_rule(
    session: SessionHandle,
    timestamp: u64,
) -> Result<(TimeCalendarTime, TimeCalendarAdditionalInfo), ToCalendarTimeError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(timezone_service_cmds::TO_CALENDAR_TIME_WITH_MY_RULE)
        .with_data_value(&timestamp)
        .build();
    req.send(&mut buf, session)
        .map_err(ToCalendarTimeError::SendRequest)?;

    #[derive(zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
    #[repr(C)]
    struct Output {
        caltime: TimeCalendarTime,
        info: TimeCalendarAdditionalInfo,
    }

    let resp = cmif::parse_response::<&Output>(&buf).map_err(ToCalendarTimeError::ParseResponse)?;

    Ok((resp.payload.caltime, resp.payload.info))
}

/// Helper function to get a clock session (used by user and network system clocks).
fn get_clock_session(
    session: SessionHandle,
    command_id: u32,
) -> Result<SessionHandle, GetSystemClockError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(command_id).build();
    req.send(&mut buf, session)
        .map_err(GetSystemClockError::SendRequest)?;

    let resp = cmif::parse_response::<()>(&buf).map_err(GetSystemClockError::ParseResponse)?;

    let Some(&handle) = resp.move_handles.first() else {
        return Err(GetSystemClockError::MissingHandle);
    };

    // SAFETY: Handle is from a valid IPC response.
    Ok(unsafe { SessionHandle::from_raw(handle) })
}

/// Error returned by system clock retrieval operations.
#[derive(Debug, thiserror::Error)]
pub enum GetSystemClockError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

impl ToResultCode for GetSystemClockError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by steady clock retrieval operation.
#[derive(Debug, thiserror::Error)]
pub enum GetSteadyClockError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

impl ToResultCode for GetSteadyClockError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by timezone service retrieval operation.
#[derive(Debug, thiserror::Error)]
pub enum GetTimeZoneServiceError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing session handle in response.
    #[error("missing session handle in response")]
    MissingHandle,
}

impl ToResultCode for GetTimeZoneServiceError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by shared memory retrieval operation.
#[derive(Debug, thiserror::Error)]
pub enum GetSharedMemoryError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Missing shared memory handle in response.
    #[error("missing shared memory handle in response")]
    MissingHandle,
}

impl ToResultCode for GetSharedMemoryError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by get current time operation.
#[derive(Debug, thiserror::Error)]
pub enum GetCurrentTimeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
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

impl ToResultCode for GetCurrentTimeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::NetworkClockUnavailable
            | Self::LocalClockNotSupported
            | Self::SourceIdMismatch => GENERIC_ERROR,
        }
    }
}

/// Error returned by calendar time conversion operation.
#[derive(Debug, thiserror::Error)]
pub enum ToCalendarTimeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for ToCalendarTimeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
