//! CMIF protocol operations for NV service.
//!
//! This module implements NV commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::ptr;

use nx_service_applet::aruid::Aruid;
use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::{
    ipc::{self, Handle as SessionHandle},
    mem::tmem::Handle as TmemHandle,
    process::Handle as ProcessHandle,
};

use crate::proto::nv_cmds;

/// Opens a device by path.
///
/// This is INvDrvServices command 0.
pub fn open(session: SessionHandle, device_path: &[u8]) -> Result<(u32, u32), OpenError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::OPEN)
        .in_buffers(1) // Device path (Type A / HipcMapAlias)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Add the device path as a Type A buffer (send buffer)
    req.add_in_buffer(device_path.as_ptr(), device_path.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(OpenError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp =
        unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(OpenError::ParseResponse)?;

    // Response contains: fd (u32), error (u32)
    #[repr(C)]
    struct Output {
        fd: u32,
        error: u32,
    }

    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };
    Ok((output.fd, output.error))
}

/// Performs an ioctl operation.
///
/// This is INvDrvServices command 1.
pub fn ioctl(
    session: SessionHandle,
    fd: u32,
    request: u32,
    in_size: usize,
    out_size: usize,
    argp: *mut u8,
) -> Result<u32, IoctlError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let num_in_auto = if in_size > 0 { 1 } else { 0 };
    let num_out_auto = if out_size > 0 { 1 } else { 0 };

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::IOCTL)
        .data_size(8) // fd + request
        .in_auto_buffers(num_in_auto)
        .out_auto_buffers(num_out_auto)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write fd and request
    #[repr(C)]
    struct Input {
        fd: u32,
        request: u32,
    }

    let input = Input { fd, request };
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    // Add auto-select buffers
    if in_size > 0 {
        req.add_in_auto_buffer(argp, in_size, BufferMode::Normal);
    }
    if out_size > 0 {
        req.add_out_auto_buffer(argp, out_size, BufferMode::Normal);
    }

    ipc::send_sync_request(session).map_err(IoctlError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp =
        unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(IoctlError::ParseResponse)?;

    // Response contains error code
    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    Ok(error)
}

/// Performs an ioctl2 operation (with extra input buffer).
///
/// This is INvDrvServices command 11 (3.0.0+).
#[allow(clippy::too_many_arguments)]
pub fn ioctl2(
    session: SessionHandle,
    fd: u32,
    request: u32,
    in_size: usize,
    out_size: usize,
    argp: *mut u8,
    extra_in: *const u8,
    extra_in_size: usize,
) -> Result<u32, Ioctl2Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    // Auto buffers: argp in (if dir & write), inbuf, argp out (if dir & read)
    let num_in_auto = if in_size > 0 { 1 } else { 0 } + 1; // +1 for extra_in
    let num_out_auto = if out_size > 0 { 1 } else { 0 };

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::IOCTL2)
        .data_size(8) // fd + request
        .in_auto_buffers(num_in_auto)
        .out_auto_buffers(num_out_auto)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write fd and request
    #[repr(C)]
    struct Input {
        fd: u32,
        request: u32,
    }

    let input = Input { fd, request };
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    // Add auto-select buffers in order: argp in, extra in, argp out
    if in_size > 0 {
        req.add_in_auto_buffer(argp, in_size, BufferMode::Normal);
    }
    req.add_in_auto_buffer(extra_in, extra_in_size, BufferMode::Normal);
    if out_size > 0 {
        req.add_out_auto_buffer(argp, out_size, BufferMode::Normal);
    }

    ipc::send_sync_request(session).map_err(Ioctl2Error::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp =
        unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(Ioctl2Error::ParseResponse)?;

    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    Ok(error)
}

/// Performs an ioctl3 operation (with extra output buffer).
///
/// This is INvDrvServices command 12 (3.0.0+).
#[allow(clippy::too_many_arguments)]
pub fn ioctl3(
    session: SessionHandle,
    fd: u32,
    request: u32,
    in_size: usize,
    out_size: usize,
    argp: *mut u8,
    extra_out: *mut u8,
    extra_out_size: usize,
) -> Result<u32, Ioctl3Error> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let num_in_auto = if in_size > 0 { 1 } else { 0 };
    let num_out_auto = if out_size > 0 { 1 } else { 0 } + 1; // +1 for extra_out

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::IOCTL3)
        .data_size(8) // fd + request
        .in_auto_buffers(num_in_auto)
        .out_auto_buffers(num_out_auto)
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write fd and request
    #[repr(C)]
    struct Input {
        fd: u32,
        request: u32,
    }

    let input = Input { fd, request };
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    // Add auto-select buffers in order: argp in, argp out, extra out
    if in_size > 0 {
        req.add_in_auto_buffer(argp, in_size, BufferMode::Normal);
    }
    if out_size > 0 {
        req.add_out_auto_buffer(argp, out_size, BufferMode::Normal);
    }
    req.add_out_auto_buffer(extra_out, extra_out_size, BufferMode::Normal);

    ipc::send_sync_request(session).map_err(Ioctl3Error::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp =
        unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(Ioctl3Error::ParseResponse)?;

    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    Ok(error)
}

/// Closes a device file descriptor.
///
/// This is INvDrvServices command 2.
pub fn close(session: SessionHandle, fd: u32) -> Result<u32, CloseError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::CLOSE)
        .data_size(4) // fd
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write fd
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), fd);
    }

    ipc::send_sync_request(session).map_err(CloseError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp =
        unsafe { cmif::parse_response(ipc_buf, false, 0) }.map_err(CloseError::ParseResponse)?;

    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    Ok(error)
}

/// Initializes the NV service with transfer memory.
///
/// This is INvDrvServices command 3.
pub fn initialize(
    session: SessionHandle,
    process_handle: ProcessHandle,
    tmem_handle: TmemHandle,
    tmem_size: u32,
) -> Result<(), InitializeError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::INITIALIZE)
        .data_size(4) // tmem_size
        .handles(2) // process handle + tmem handle
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write tmem_size
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u32>().cast_mut(), tmem_size);
    }

    // Add copy handles
    req.add_handle(process_handle.to_raw());
    req.add_handle(tmem_handle.to_raw());

    ipc::send_sync_request(session).map_err(InitializeError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(InitializeError::ParseResponse)?;

    Ok(())
}

/// Queries an event for a device.
///
/// This is INvDrvServices command 4.
pub fn query_event(
    session: SessionHandle,
    fd: u32,
    event_id: u32,
) -> Result<(u32, u32), QueryEventError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::QUERY_EVENT)
        .data_size(8) // fd + event_id
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write fd and event_id
    #[repr(C)]
    struct Input {
        fd: u32,
        event_id: u32,
    }

    let input = Input { fd, event_id };
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<Input>().cast_mut(), input);
    }

    ipc::send_sync_request(session).map_err(QueryEventError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(QueryEventError::ParseResponse)?;

    // Response contains error code, and a copy handle for the event
    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };
    let event_handle = resp
        .copy_handles
        .first()
        .copied()
        .ok_or(QueryEventError::MissingHandle)?;

    Ok((event_handle, error))
}

/// Sets the client PID (ARUID).
///
/// This is INvDrvServices command 8.
pub fn set_client_pid(session: SessionHandle, aruid: Aruid) -> Result<(), SetClientPidError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(nv_cmds::SET_CLIENT_PID)
        .data_size(8) // ARUID
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };

    // Write ARUID
    unsafe {
        ptr::write_unaligned(req.data.as_ptr().cast::<u64>().cast_mut(), aruid.to_raw());
    }

    ipc::send_sync_request(session).map_err(SetClientPidError::SendRequest)?;

    // SAFETY: Response is in TLS buffer after successful send.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(SetClientPidError::ParseResponse)?;

    Ok(())
}

/// Error returned by open operation.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by ioctl operation.
#[derive(Debug, thiserror::Error)]
pub enum IoctlError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by ioctl2 operation.
#[derive(Debug, thiserror::Error)]
pub enum Ioctl2Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by ioctl3 operation.
#[derive(Debug, thiserror::Error)]
pub enum Ioctl3Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by close operation.
#[derive(Debug, thiserror::Error)]
pub enum CloseError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by initialize operation.
#[derive(Debug, thiserror::Error)]
pub enum InitializeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}

/// Error returned by query_event operation.
#[derive(Debug, thiserror::Error)]
pub enum QueryEventError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
    /// Missing event handle in response.
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Error returned by set_client_pid operation.
#[derive(Debug, thiserror::Error)]
pub enum SetClientPidError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseResponseError),
}
