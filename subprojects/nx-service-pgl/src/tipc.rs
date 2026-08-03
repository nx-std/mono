//! TIPC protocol operations for the PGL service.
//!
//! Used on HOS 12.0.0+.

use core::{mem::size_of, ptr};

use nx_sf::{
    hipc::{BufferMode, InputBuffer},
    ipc::Handle as RawSessionHandle,
    service::{BorrowedSessionHandle, OwnedSessionHandle, Session},
    tipc,
};

use crate::{
    proto,
    types::{
        ContentMetaInfo, LaunchProgramTipcIn, NcmProgramLocation, PglLaunchFlag, ProcessEventInfo,
    },
};

fn dispatch_in_u64(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: u64,
) -> Result<(), DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut payload = [0u8; size_of::<u64>()];
    // SAFETY: `payload` is exactly `size_of::<u64>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<u64>(), value) };
    let req = tipc::TipcRequestBuilder::new(cmd_id)
        .with_data(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    tipc::parse_response::<()>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_in_bool(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    value: bool,
) -> Result<(), DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut payload = [0u8; size_of::<u8>()];
    // SAFETY: `payload` is exactly `size_of::<u8>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<u8>(), value as u8) };
    let req = tipc::TipcRequestBuilder::new(cmd_id)
        .with_data(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    tipc::parse_response::<()>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_out_u64(session: BorrowedSessionHandle<'_>, cmd_id: u32) -> Result<u64, DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    tipc::TipcRequestBuilder::new(cmd_id)
        .build()
        .send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp = tipc::parse_response::<&u64>(&buf).map_err(DispatchError::ParseResponse)?;

    let value = *resp.payload;

    Ok(value)
}

fn dispatch_out_bool(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
) -> Result<bool, DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    tipc::TipcRequestBuilder::new(cmd_id)
        .build()
        .send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp = tipc::parse_response::<&u8>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(*resp.payload & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

/// Launches a program (cmd 0, TIPC).
pub fn launch_program(
    session: BorrowedSessionHandle<'_>,
    loc: &NcmProgramLocation,
    pm_launch_flags: u32,
    pgl_launch_flags: PglLaunchFlag,
) -> Result<u64, DispatchError> {
    let input = LaunchProgramTipcIn {
        loc: *loc,
        pm_flags: pm_launch_flags,
        pgl_flags: pgl_launch_flags,
        pad: [0; 3],
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut payload = [0u8; size_of::<LaunchProgramTipcIn>()];
    // SAFETY: `payload` is exactly `size_of::<LaunchProgramTipcIn>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<LaunchProgramTipcIn>(), input) };
    let req = tipc::TipcRequestBuilder::new(proto::LAUNCH_PROGRAM)
        .with_data(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp = tipc::parse_response::<&u64>(&buf).map_err(DispatchError::ParseResponse)?;

    let pid = *resp.payload;

    Ok(pid)
}

/// Terminates a process (cmd 1).
pub fn terminate_process(
    session: BorrowedSessionHandle<'_>,
    pid: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64(session, proto::TERMINATE_PROCESS, pid)
}

/// Launches a program from a host content path (cmd 2, TIPC).
pub fn launch_program_from_host(
    session: BorrowedSessionHandle<'_>,
    content_path: &[u8],
    pm_launch_flags: u32,
) -> Result<u64, DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut payload = [0u8; size_of::<u32>()];
    // SAFETY: `payload` is exactly `size_of::<u32>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<u32>(), pm_launch_flags) };
    let req = tipc::TipcRequestBuilder::new(proto::LAUNCH_PROGRAM_FROM_HOST)
        .with_data(&payload)
        .add_input_buffer(InputBuffer::new(content_path, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp = tipc::parse_response::<&u64>(&buf).map_err(DispatchError::ParseResponse)?;

    let pid = *resp.payload;

    Ok(pid)
}

/// Gets host content meta info (cmd 4, TIPC).
pub fn get_host_content_meta_info(
    session: BorrowedSessionHandle<'_>,
    content_path: &[u8],
) -> Result<ContentMetaInfo, DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = tipc::TipcRequestBuilder::new(proto::GET_HOST_CONTENT_META_INFO)
        .add_input_buffer(InputBuffer::new(content_path, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp =
        tipc::parse_response::<&ContentMetaInfo>(&buf).map_err(DispatchError::ParseResponse)?;

    let info = *resp.payload;

    Ok(info)
}

/// Gets the application process ID (cmd 5).
pub fn get_application_process_id(
    session: BorrowedSessionHandle<'_>,
) -> Result<u64, DispatchError> {
    dispatch_out_u64(session, proto::GET_APPLICATION_PROCESS_ID)
}

/// Boosts system memory resource limit (cmd 6).
pub fn boost_system_memory_resource_limit(
    session: BorrowedSessionHandle<'_>,
    size: u64,
) -> Result<(), DispatchError> {
    dispatch_in_u64(session, proto::BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT, size)
}

/// Checks whether a process is tracked (cmd 7, TIPC).
pub fn is_process_tracked(
    session: BorrowedSessionHandle<'_>,
    pid: u64,
) -> Result<bool, DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let mut payload = [0u8; size_of::<u64>()];
    // SAFETY: `payload` is exactly `size_of::<u64>()` bytes.
    unsafe { ptr::write_unaligned(payload.as_mut_ptr().cast::<u64>(), pid) };
    let req = tipc::TipcRequestBuilder::new(proto::IS_PROCESS_TRACKED)
        .with_data(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp = tipc::parse_response::<&u8>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(*resp.payload & 1 != 0)
}

/// Enables/disables application crash reports (cmd 8).
pub fn enable_application_crash_report(
    session: BorrowedSessionHandle<'_>,
    enable: bool,
) -> Result<(), DispatchError> {
    dispatch_in_bool(session, proto::ENABLE_APPLICATION_CRASH_REPORT, enable)
}

/// Checks whether application crash reports are enabled (cmd 9).
pub fn is_application_crash_report_enabled(
    session: BorrowedSessionHandle<'_>,
) -> Result<bool, DispatchError> {
    dispatch_out_bool(session, proto::IS_APPLICATION_CRASH_REPORT_ENABLED)
}

/// Enables/disables all-thread dump on crash (cmd 10).
pub fn enable_application_all_thread_dump_on_crash(
    session: BorrowedSessionHandle<'_>,
    enable: bool,
) -> Result<(), DispatchError> {
    dispatch_in_bool(
        session,
        proto::ENABLE_APPLICATION_ALL_THREAD_DUMP_ON_CRASH,
        enable,
    )
}

/// Gets an event observer sub-object (cmd 20, TIPC).
pub fn get_event_observer(
    session: BorrowedSessionHandle<'_>,
) -> Result<Session, GetEventObserverError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    tipc::TipcRequestBuilder::new(proto::GET_EVENT_OBSERVER)
        .build()
        .send(&mut buf, session)
        .map_err(GetEventObserverError::SendRequest)?;

    let resp = tipc::parse_response::<()>(&buf).map_err(GetEventObserverError::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(GetEventObserverError::MissingHandle)?;

    // SAFETY: the kernel returned a valid move handle for the new observer
    // session; ownership transfers to the new `Session`.
    let handle =
        OwnedSessionHandle::from_handle_unchecked(RawSessionHandle::from_raw_unchecked(raw_handle));

    Ok(Session::new(handle, 0))
}

#[derive(Debug, thiserror::Error)]
pub enum GetEventObserverError {
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
    #[error("missing observer handle in response")]
    MissingHandle,
}

/// Gets the process event handle from the observer (cmd 0, copy handle).
pub fn observer_get_process_event(
    session: BorrowedSessionHandle<'_>,
) -> Result<u32, GetProcessEventError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    tipc::TipcRequestBuilder::new(proto::OBSERVER_GET_PROCESS_EVENT)
        .build()
        .send(&mut buf, session)
        .map_err(GetProcessEventError::SendRequest)?;

    let resp = tipc::parse_response::<()>(&buf).map_err(GetProcessEventError::ParseResponse)?;

    let raw_handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetProcessEventError::MissingHandle)?;

    Ok(raw_handle)
}

#[derive(Debug, thiserror::Error)]
pub enum GetProcessEventError {
    #[error("failed to send request")]
    SendRequest(#[source] tipc::SendError),
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Gets the process event info from the observer (cmd 1).
pub fn observer_get_process_event_info(
    session: BorrowedSessionHandle<'_>,
) -> Result<ProcessEventInfo, DispatchError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    tipc::TipcRequestBuilder::new(proto::OBSERVER_GET_PROCESS_EVENT_INFO)
        .build()
        .send(&mut buf, session)
        .map_err(DispatchError::SendRequest)?;

    let resp =
        tipc::parse_response::<&ProcessEventInfo>(&buf).map_err(DispatchError::ParseResponse)?;

    Ok(*resp.payload)
}
