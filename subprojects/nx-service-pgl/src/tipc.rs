//! TIPC protocol operations for the PGL service.
//!
//! Used on HOS 12.0.0+.

use core::{mem::size_of, ptr};

use nx_sf::{cmif, hipc::BufferMode, service::Session, tipc};
use nx_svc::ipc::{self, Handle};

use crate::{
    proto,
    types::{
        ContentMetaInfo, LaunchProgramTipcIn, NcmProgramLocation, PglLaunchFlag, ProcessEventInfo,
    },
};

// ---------------------------------------------------------------------------
// Dispatch helpers
// ---------------------------------------------------------------------------

fn dispatch_in_u64(session: Handle, cmd_id: u32, value: u64) -> Result<(), DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcBuilder::new(&mut buf, cmd_id)
            .data_size(size_of::<u64>())
            .send()
            .map_err(DispatchError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), value) };
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    tipc::parse_response(&buf, 0).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_in_bool(session: Handle, cmd_id: u32, value: bool) -> Result<(), DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcBuilder::new(&mut buf, cmd_id)
            .data_size(size_of::<u8>())
            .send()
            .map_err(DispatchError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u8>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u8>(), value as u8) };
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    tipc::parse_response(&buf, 0).map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_out_u64(session: Handle, cmd_id: u32) -> Result<u64, DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        tipc::TipcBuilder::new(&mut buf, cmd_id)
            .send()
            .map_err(DispatchError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        tipc::parse_response(&buf, size_of::<u64>()).map_err(DispatchError::ParseResponse)?;

    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(value)
}

fn dispatch_out_bool(session: Handle, cmd_id: u32) -> Result<bool, DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        tipc::TipcBuilder::new(&mut buf, cmd_id)
            .send()
            .map_err(DispatchError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, size_of::<u8>()).map_err(DispatchError::ParseResponse)?;

    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(value & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
}

// ---------------------------------------------------------------------------
// Root service commands
// ---------------------------------------------------------------------------

/// Launches a program (cmd 0, TIPC).
pub fn launch_program(
    session: Handle,
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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcBuilder::new(&mut buf, proto::LAUNCH_PROGRAM)
            .data_size(size_of::<LaunchProgramTipcIn>())
            .send()
            .map_err(DispatchError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<LaunchProgramTipcIn>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<LaunchProgramTipcIn>(), input) };
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        tipc::parse_response(&buf, size_of::<u64>()).map_err(DispatchError::ParseResponse)?;

    let pid = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(pid)
}

/// Terminates a process (cmd 1).
pub fn terminate_process(session: Handle, pid: u64) -> Result<(), DispatchError> {
    dispatch_in_u64(session, proto::TERMINATE_PROCESS, pid)
}

/// Launches a program from a host content path (cmd 2, TIPC).
pub fn launch_program_from_host(
    session: Handle,
    content_path: &[u8],
    pm_launch_flags: u32,
) -> Result<u64, DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcBuilder::new(&mut buf, proto::LAUNCH_PROGRAM_FROM_HOST)
            .data_size(size_of::<u32>())
            .add_in_buffer(
                content_path.as_ptr(),
                content_path.len(),
                BufferMode::Normal,
            )
            .send()
            .map_err(DispatchError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u32>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u32>(), pm_launch_flags) };
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp =
        tipc::parse_response(&buf, size_of::<u64>()).map_err(DispatchError::ParseResponse)?;

    let pid = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(pid)
}

/// Gets host content meta info (cmd 4, TIPC).
pub fn get_host_content_meta_info(
    session: Handle,
    content_path: &[u8],
) -> Result<ContentMetaInfo, DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        tipc::TipcBuilder::new(&mut buf, proto::GET_HOST_CONTENT_META_INFO)
            .add_in_buffer(
                content_path.as_ptr(),
                content_path.len(),
                BufferMode::Normal,
            )
            .send()
            .map_err(DispatchError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, size_of::<ContentMetaInfo>())
        .map_err(DispatchError::ParseResponse)?;

    let info = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<ContentMetaInfo>()) };

    Ok(info)
}

/// Gets the application process ID (cmd 5).
pub fn get_application_process_id(session: Handle) -> Result<u64, DispatchError> {
    dispatch_out_u64(session, proto::GET_APPLICATION_PROCESS_ID)
}

/// Boosts system memory resource limit (cmd 6).
pub fn boost_system_memory_resource_limit(session: Handle, size: u64) -> Result<(), DispatchError> {
    dispatch_in_u64(session, proto::BOOST_SYSTEM_MEMORY_RESOURCE_LIMIT, size)
}

/// Checks whether a process is tracked (cmd 7, TIPC).
pub fn is_process_tracked(session: Handle, pid: u64) -> Result<bool, DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = tipc::TipcBuilder::new(&mut buf, proto::IS_PROCESS_TRACKED)
            .data_size(size_of::<u64>())
            .send()
            .map_err(DispatchError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), pid) };
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, size_of::<u8>()).map_err(DispatchError::ParseResponse)?;

    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(value & 1 != 0)
}

/// Enables/disables application crash reports (cmd 8).
pub fn enable_application_crash_report(session: Handle, enable: bool) -> Result<(), DispatchError> {
    dispatch_in_bool(session, proto::ENABLE_APPLICATION_CRASH_REPORT, enable)
}

/// Checks whether application crash reports are enabled (cmd 9).
pub fn is_application_crash_report_enabled(session: Handle) -> Result<bool, DispatchError> {
    dispatch_out_bool(session, proto::IS_APPLICATION_CRASH_REPORT_ENABLED)
}

/// Enables/disables all-thread dump on crash (cmd 10).
pub fn enable_application_all_thread_dump_on_crash(
    session: Handle,
    enable: bool,
) -> Result<(), DispatchError> {
    dispatch_in_bool(
        session,
        proto::ENABLE_APPLICATION_ALL_THREAD_DUMP_ON_CRASH,
        enable,
    )
}

/// Gets an event observer sub-object (cmd 20, TIPC).
pub fn get_event_observer(session: Handle) -> Result<Session, GetEventObserverError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        tipc::TipcBuilder::new(&mut buf, proto::GET_EVENT_OBSERVER)
            .send()
            .map_err(GetEventObserverError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetEventObserverError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, 0).map_err(GetEventObserverError::ParseResponse)?;

    let raw_handle = resp
        .move_handles
        .first()
        .copied()
        .ok_or(GetEventObserverError::MissingHandle)?;

    // SAFETY: the kernel returned a valid move handle for the new observer
    // session; ownership transfers to the new `Session`.
    let handle = unsafe { Handle::from_raw(raw_handle) };

    Ok(Session::from_handle(handle, 0))
}

#[derive(Debug, thiserror::Error)]
pub enum GetEventObserverError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
    #[error("missing observer handle in response")]
    MissingHandle,
}

// ---------------------------------------------------------------------------
// EventObserver sub-object commands (TIPC)
// ---------------------------------------------------------------------------

/// Gets the process event handle from the observer (cmd 0, copy handle).
pub fn observer_get_process_event(session: Handle) -> Result<u32, GetProcessEventError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        tipc::TipcBuilder::new(&mut buf, proto::OBSERVER_GET_PROCESS_EVENT)
            .send()
            .map_err(GetProcessEventError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(GetProcessEventError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, 0).map_err(GetProcessEventError::ParseResponse)?;

    let raw_handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(GetProcessEventError::MissingHandle)?;

    Ok(raw_handle)
}

#[derive(Debug, thiserror::Error)]
pub enum GetProcessEventError {
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Gets the process event info from the observer (cmd 1).
pub fn observer_get_process_event_info(session: Handle) -> Result<ProcessEventInfo, DispatchError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        tipc::TipcBuilder::new(&mut buf, proto::OBSERVER_GET_PROCESS_EVENT_INFO)
            .send()
            .map_err(DispatchError::BuildRequest)?;
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = tipc::parse_response(&buf, size_of::<ProcessEventInfo>())
        .map_err(DispatchError::ParseResponse)?;

    let info = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<ProcessEventInfo>()) };

    Ok(info)
}
