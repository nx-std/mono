//! TIPC protocol operations for the PGL service.
//!
//! Used on HOS 12.0.0+.

use core::ptr;

use nx_sf::{hipc::BufferMode, service::Session, tipc};
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: cmd_id,
        data_size: size_of::<u64>(),
        ..Default::default()
    };

    let req = unsafe { tipc::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), value);
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    unsafe { tipc::parse_response(ipc_buf, 0) }.map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_in_bool(session: Handle, cmd_id: u32, value: bool) -> Result<(), DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: cmd_id,
        data_size: size_of::<u8>(),
        ..Default::default()
    };

    let req = unsafe { tipc::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u8>().cast_mut(), value as u8);
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    unsafe { tipc::parse_response(ipc_buf, 0) }.map_err(DispatchError::ParseResponse)?;

    Ok(())
}

fn dispatch_out_u64(session: Handle, cmd_id: u32) -> Result<u64, DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: cmd_id,
        ..Default::default()
    };

    unsafe { tipc::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { tipc::parse_response(ipc_buf, size_of::<u64>()) }
        .map_err(DispatchError::ParseResponse)?;

    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(value)
}

fn dispatch_out_bool(session: Handle, cmd_id: u32) -> Result<bool, DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: cmd_id,
        ..Default::default()
    };

    unsafe { tipc::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { tipc::parse_response(ipc_buf, size_of::<u8>()) }
        .map_err(DispatchError::ParseResponse)?;

    let value = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u8>()) };

    Ok(value & 1 != 0)
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
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

    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::LAUNCH_PROGRAM,
        data_size: size_of::<LaunchProgramTipcIn>(),
        ..Default::default()
    };

    let req = unsafe { tipc::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(
            req.data.as_ptr().cast::<LaunchProgramTipcIn>().cast_mut(),
            input,
        );
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { tipc::parse_response(ipc_buf, size_of::<u64>()) }
        .map_err(DispatchError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::LAUNCH_PROGRAM_FROM_HOST,
        data_size: size_of::<u32>(),
        num_in_buffers: 1,
        ..Default::default()
    };

    let mut req = unsafe { tipc::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), pm_launch_flags);
    }

    req.add_in_buffer(
        content_path.as_ptr(),
        content_path.len(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { tipc::parse_response(ipc_buf, size_of::<u64>()) }
        .map_err(DispatchError::ParseResponse)?;

    let pid = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };

    Ok(pid)
}

/// Gets host content meta info (cmd 4, TIPC).
pub fn get_host_content_meta_info(
    session: Handle,
    content_path: &[u8],
) -> Result<ContentMetaInfo, DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::GET_HOST_CONTENT_META_INFO,
        num_in_buffers: 1,
        ..Default::default()
    };

    let mut req = unsafe { tipc::make_request(ipc_buf, fmt) };

    req.add_in_buffer(
        content_path.as_ptr(),
        content_path.len(),
        BufferMode::Normal,
    );

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { tipc::parse_response(ipc_buf, size_of::<ContentMetaInfo>()) }
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::IS_PROCESS_TRACKED,
        data_size: size_of::<u64>(),
        ..Default::default()
    };

    let req = unsafe { tipc::make_request(ipc_buf, fmt) };

    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), pid);
    }

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { tipc::parse_response(ipc_buf, size_of::<u8>()) }
        .map_err(DispatchError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::GET_EVENT_OBSERVER,
        ..Default::default()
    };

    // SAFETY: `ipc_buf` is the live TLS IPC buffer for this thread.
    unsafe { tipc::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetEventObserverError::SendRequest)?;

    // SAFETY: response sits in the TLS buffer after a successful send.
    let resp = unsafe { tipc::parse_response(ipc_buf, 0) }
        .map_err(GetEventObserverError::ParseResponse)?;

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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::OBSERVER_GET_PROCESS_EVENT,
        ..Default::default()
    };

    unsafe { tipc::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(GetProcessEventError::SendRequest)?;

    let resp =
        unsafe { tipc::parse_response(ipc_buf, 0) }.map_err(GetProcessEventError::ParseResponse)?;

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
    SendRequest(#[source] ipc::SendSyncError),
    #[error("failed to parse response")]
    ParseResponse(#[source] tipc::ParseResponseError),
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Gets the process event info from the observer (cmd 1).
pub fn observer_get_process_event_info(session: Handle) -> Result<ProcessEventInfo, DispatchError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = tipc::RequestFormat {
        request_id: proto::OBSERVER_GET_PROCESS_EVENT_INFO,
        ..Default::default()
    };

    unsafe { tipc::make_request(ipc_buf, fmt) };

    ipc::send_sync_request(session).map_err(DispatchError::SendRequest)?;

    let resp = unsafe { tipc::parse_response(ipc_buf, size_of::<ProcessEventInfo>()) }
        .map_err(DispatchError::ParseResponse)?;

    let info = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<ProcessEventInfo>()) };

    Ok(info)
}
