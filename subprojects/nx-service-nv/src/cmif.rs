//! CMIF protocol operations for NV service.
//!
//! This module implements NV commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use core::{mem::size_of, ptr};

use nx_service_applet::aruid::Aruid;
use nx_sf::{
    cmif,
    hipc::BufferMode,
    ipc::{self, Handle as SessionHandle},
};
use nx_svc::{
    mem::tmem::Handle as TmemHandle, process::Handle as ProcessHandle, raw::Handle as RawHandle,
};

use crate::{
    fd::Fd,
    proto::nv_cmds,
    types::{CloseNvError, IoctlNvError, OpenNvError, QueryEventNvError},
};

/// Opens a device by path.
///
/// This is INvDrvServices command 0.
pub fn open(session: SessionHandle, device_path: &[u8]) -> Result<Fd, OpenError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(nv_cmds::OPEN)
        .add_in_buffer(device_path.as_ptr(), device_path.len(), BufferMode::Normal)
        .build();
    req.write_to(&mut buf).map_err(OpenError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(OpenError::SendRequest)?;

    // Response contains: fd (u32), error (u32).
    #[repr(C)]
    struct Output {
        fd: u32,
        error: u32,
    }

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<Output>()).map_err(OpenError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<Output>()` bytes.
    let output = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<Output>()) };

    if output.error != 0 {
        return Err(OpenError::NvError(OpenNvError::from_raw(output.error)));
    }

    // SAFETY: The fd was just returned by the NV driver via IPC.
    Ok(unsafe { Fd::new_unchecked(output.fd) })
}

/// Performs an ioctl operation.
///
/// This is INvDrvServices command 1.
pub fn ioctl(
    session: SessionHandle,
    fd: Fd,
    request: u32,
    in_size: usize,
    out_size: usize,
    argp: *mut u8,
) -> Result<(), IoctlError> {
    // Write fd and request.
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct Input {
        fd: u32,
        request: u32,
    }

    let input = Input {
        fd: fd.to_raw(),
        request,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    {
        let mut builder = cmif::CmifRequestBuilder::new(nv_cmds::IOCTL).data_value(&input);
        if in_size > 0 {
            builder = builder.add_in_auto_buffer(argp, in_size, BufferMode::Normal);
        }
        if out_size > 0 {
            builder = builder.add_out_auto_buffer(argp, out_size, BufferMode::Normal);
        }
        builder
            .build()
            .write_to(&mut buf)
            .map_err(IoctlError::BuildRequest)?
    };

    ipc::send_sync_request(&mut buf, session).map_err(IoctlError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<u32>()).map_err(IoctlError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u32>()` bytes.
    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    if error != 0 {
        return Err(IoctlError::NvError(IoctlNvError::from_raw(error)));
    }

    Ok(())
}

/// Performs an ioctl2 operation (with extra input buffer).
///
/// This is INvDrvServices command 11 (3.0.0+).
#[allow(clippy::too_many_arguments)]
pub fn ioctl2(
    session: SessionHandle,
    fd: Fd,
    request: u32,
    in_size: usize,
    out_size: usize,
    argp: *mut u8,
    extra_in: *const u8,
    extra_in_size: usize,
) -> Result<(), Ioctl2Error> {
    // Write fd and request.
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct Input {
        fd: u32,
        request: u32,
    }

    let input = Input {
        fd: fd.to_raw(),
        request,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    {
        // Auto buffers in order: argp in (if applicable), extra in, argp out (if applicable).
        let mut builder = cmif::CmifRequestBuilder::new(nv_cmds::IOCTL2).data_value(&input);
        if in_size > 0 {
            builder = builder.add_in_auto_buffer(argp, in_size, BufferMode::Normal);
        }
        builder = builder.add_in_auto_buffer(extra_in, extra_in_size, BufferMode::Normal);
        if out_size > 0 {
            builder = builder.add_out_auto_buffer(argp, out_size, BufferMode::Normal);
        }
        builder
            .build()
            .write_to(&mut buf)
            .map_err(Ioctl2Error::BuildRequest)?
    };

    ipc::send_sync_request(&mut buf, session).map_err(Ioctl2Error::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<u32>()).map_err(Ioctl2Error::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u32>()` bytes.
    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    if error != 0 {
        return Err(Ioctl2Error::NvError(IoctlNvError::from_raw(error)));
    }

    Ok(())
}

/// Performs an ioctl3 operation (with extra output buffer).
///
/// This is INvDrvServices command 12 (3.0.0+).
#[allow(clippy::too_many_arguments)]
pub fn ioctl3(
    session: SessionHandle,
    fd: Fd,
    request: u32,
    in_size: usize,
    out_size: usize,
    argp: *mut u8,
    extra_out: *mut u8,
    extra_out_size: usize,
) -> Result<(), Ioctl3Error> {
    // Write fd and request.
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct Input {
        fd: u32,
        request: u32,
    }

    let input = Input {
        fd: fd.to_raw(),
        request,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    {
        // Auto buffers in order: argp in (if applicable), argp out (if applicable), extra out.
        let mut builder = cmif::CmifRequestBuilder::new(nv_cmds::IOCTL3).data_value(&input);
        if in_size > 0 {
            builder = builder.add_in_auto_buffer(argp, in_size, BufferMode::Normal);
        }
        if out_size > 0 {
            builder = builder.add_out_auto_buffer(argp, out_size, BufferMode::Normal);
        }
        builder = builder.add_out_auto_buffer(extra_out, extra_out_size, BufferMode::Normal);
        builder
            .build()
            .write_to(&mut buf)
            .map_err(Ioctl3Error::BuildRequest)?
    };

    ipc::send_sync_request(&mut buf, session).map_err(Ioctl3Error::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<u32>()).map_err(Ioctl3Error::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u32>()` bytes.
    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    if error != 0 {
        return Err(Ioctl3Error::NvError(IoctlNvError::from_raw(error)));
    }

    Ok(())
}

/// Closes a device file descriptor.
///
/// This is INvDrvServices command 2.
pub fn close(session: SessionHandle, fd: Fd) -> Result<(), CloseError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let fd_raw = fd.to_raw();
    let req = cmif::CmifRequestBuilder::new(nv_cmds::CLOSE)
        .data_value(&fd_raw)
        .build();
    req.write_to(&mut buf).map_err(CloseError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(CloseError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<u32>()).map_err(CloseError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u32>()` bytes.
    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    if error != 0 {
        return Err(CloseError::NvError(CloseNvError::from_raw(error)));
    }

    Ok(())
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
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(nv_cmds::INITIALIZE)
        .data_value(&tmem_size)
        .add_copy_handle(process_handle.to_raw())
        .add_copy_handle(tmem_handle.to_raw())
        .build();
    req.write_to(&mut buf)
        .map_err(InitializeError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(InitializeError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(InitializeError::ParseResponse)?;

    Ok(())
}

/// Queries an event for a device.
///
/// This is INvDrvServices command 4.
pub fn query_event(
    session: SessionHandle,
    fd: Fd,
    event_id: u32,
) -> Result<RawHandle, QueryEventError> {
    // Write fd and event_id.
    #[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
    #[repr(C)]
    struct Input {
        fd: u32,
        event_id: u32,
    }

    let input = Input {
        fd: fd.to_raw(),
        event_id,
    };

    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let req = cmif::CmifRequestBuilder::new(nv_cmds::QUERY_EVENT)
        .data_value(&input)
        .build();
    req.write_to(&mut buf)
        .map_err(QueryEventError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(QueryEventError::SendRequest)?;

    // Response contains error code (u32) and a copy handle for the event.
    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(QueryEventError::ParseResponse)?;

    // SAFETY: `resp.data` is exactly `size_of::<u32>()` bytes.
    let error = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    if error != 0 {
        return Err(QueryEventError::NvError(QueryEventNvError::from_raw(error)));
    }

    let Some(&event_handle) = resp.copy_handles.first() else {
        return Err(QueryEventError::MissingHandle);
    };

    Ok(event_handle)
}

/// Sets the client PID (ARUID).
///
/// This is INvDrvServices command 8.
pub fn set_client_pid(session: SessionHandle, aruid: Aruid) -> Result<(), SetClientPidError> {
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };

    let aruid_raw = aruid.to_raw();
    let req = cmif::CmifRequestBuilder::new(nv_cmds::SET_CLIENT_PID)
        .data_value(&aruid_raw)
        .send_pid()
        .build();
    req.write_to(&mut buf)
        .map_err(SetClientPidError::BuildRequest)?;

    ipc::send_sync_request(&mut buf, session).map_err(SetClientPidError::SendRequest)?;

    cmif::parse_response_bytes(&buf, 0).map_err(SetClientPidError::ParseResponse)?;

    Ok(())
}

/// Error returned by open operation.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] OpenNvError),
}

/// Error returned by ioctl operation.
#[derive(Debug, thiserror::Error)]
pub enum IoctlError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] IoctlNvError),
}

/// Error returned by ioctl2 operation.
#[derive(Debug, thiserror::Error)]
pub enum Ioctl2Error {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] IoctlNvError),
}

/// Error returned by ioctl3 operation.
#[derive(Debug, thiserror::Error)]
pub enum Ioctl3Error {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] IoctlNvError),
}

/// Error returned by close operation.
#[derive(Debug, thiserror::Error)]
pub enum CloseError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] CloseNvError),
}

/// Error returned by initialize operation.
#[derive(Debug, thiserror::Error)]
pub enum InitializeError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}

/// Error returned by query_event operation.
#[derive(Debug, thiserror::Error)]
pub enum QueryEventError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] QueryEventNvError),
    /// Missing event handle in response.
    #[error("missing event handle in response")]
    MissingHandle,
}

/// Error returned by set_client_pid operation.
#[derive(Debug, thiserror::Error)]
pub enum SetClientPidError {
    /// Failed to build the CMIF request.
    #[error("failed to build request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] ipc::SendSyncError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseRespBytesError),
}
