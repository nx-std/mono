//! CMIF protocol operations for NV service.
//!
//! This module implements NV commands using the CMIF (Common Message Interface
//! Format) protocol, which is the standard IPC protocol on Horizon OS.

use nx_service_applet::aruid::Aruid;
use nx_sf::{
    cmif,
    error::{
        GENERIC_ERROR,
        ResultCode,
        ToResultCode,
    },
    hipc::{
        BufferMode,
        InOutBuffer,
        InputBuffer,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};
use nx_svc::{
    mem::tmem::Handle as TmemHandle,
    process::Handle as ProcessHandle,
    raw::Handle as RawHandle,
};

use crate::{
    fd::Fd,
    proto::nv_cmds,
    types::{
        CloseNvError,
        IoctlNvError,
        OpenNvError,
        QueryEventNvError,
    },
};

/// Opens a device by path.
///
/// This is INvDrvServices command 0.
pub fn open(session: BorrowedSessionHandle<'_>, device_path: &[u8]) -> Result<Fd, OpenError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(nv_cmds::OPEN)
        .add_input_buffer(InputBuffer::new(device_path, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(OpenError::SendRequest)?;

    // Response contains: fd (u32), error (u32).
    #[derive(zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
    #[repr(C)]
    struct Output {
        fd: u32,
        error: u32,
    }

    let resp = cmif::parse_response::<&Output>(&buf).map_err(OpenError::ParseResponse)?;
    let output = resp.payload;

    if output.error != 0 {
        return Err(OpenError::NvError(OpenNvError::from_raw(output.error)));
    }

    // SAFETY: The descriptor is the one the driver just returned for this open.
    Ok(Fd::from_raw_unchecked(output.fd))
}

/// Performs an ioctl operation.
///
/// This is INvDrvServices command 1.
pub fn ioctl(
    session: BorrowedSessionHandle<'_>,
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    // Both halves target the same `argp` region when in_size == out_size and both are
    // requested - the nv ioctl ABI never mixes an in-only and an out-only size over
    // shared memory, so a single inout descriptor pair covers that case.
    let mut builder = cmif::CmifRequestBuilder::new(nv_cmds::IOCTL).with_data_value(&input);
    builder = match (in_size > 0, out_size > 0) {
        (true, true) => {
            // SAFETY: nv ioctl ABI - the caller of __nx_nv_ioctl* guarantees argp is
            // valid for in_size (== out_size) bytes for the duration of this call, and
            // exclusively borrowed since the request reads then writes through it.
            let buf = unsafe { InOutBuffer::from_raw_parts(argp, in_size, BufferMode::Normal) };
            builder.add_inout_auto_buffer(buf)
        }
        (true, false) => {
            // SAFETY: nv ioctl ABI - the caller of __nx_nv_ioctl* guarantees argp is
            // valid for in_size bytes for the duration of this call.
            let buf = unsafe { InputBuffer::from_raw_parts(argp, in_size, BufferMode::Normal) };
            builder.add_in_auto_buffer(buf)
        }
        (false, true) => {
            // SAFETY: nv ioctl ABI - the caller of __nx_nv_ioctl* guarantees argp is
            // valid for out_size bytes for the duration of this call.
            let buf = unsafe { OutputBuffer::from_raw_parts(argp, out_size, BufferMode::Normal) };
            builder.add_out_auto_buffer(buf)
        }
        (false, false) => builder,
    };

    builder
        .build()
        .send(&mut buf, session)
        .map_err(IoctlError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(IoctlError::ParseResponse)?;
    let error = *resp.payload;

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
    session: BorrowedSessionHandle<'_>,
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    // Auto buffers in order: argp in (if applicable), extra in, argp out (if
    // applicable). The in/out descriptor lists are tracked independently by the
    // builder, so folding argp's in and out halves into one inout descriptor here
    // does not disturb that order.
    let mut builder = cmif::CmifRequestBuilder::new(nv_cmds::IOCTL2).with_data_value(&input);
    builder = match (in_size > 0, out_size > 0) {
        (true, true) => {
            // SAFETY: nv ioctl2 ABI - the caller of __nx_nv_ioctl2* guarantees argp is
            // valid for in_size (== out_size) bytes for the duration of this call, and
            // exclusively borrowed since the request reads then writes through it.
            let buf = unsafe { InOutBuffer::from_raw_parts(argp, in_size, BufferMode::Normal) };
            builder.add_inout_auto_buffer(buf)
        }
        (true, false) => {
            // SAFETY: nv ioctl2 ABI - the caller of __nx_nv_ioctl2* guarantees argp is
            // valid for in_size bytes for the duration of this call.
            let buf = unsafe { InputBuffer::from_raw_parts(argp, in_size, BufferMode::Normal) };
            builder.add_in_auto_buffer(buf)
        }
        (false, true) => {
            // SAFETY: nv ioctl2 ABI - the caller of __nx_nv_ioctl2* guarantees argp is
            // valid for out_size bytes for the duration of this call.
            let buf = unsafe { OutputBuffer::from_raw_parts(argp, out_size, BufferMode::Normal) };
            builder.add_out_auto_buffer(buf)
        }
        (false, false) => builder,
    };

    // SAFETY: nv ioctl2 ABI - the caller of __nx_nv_ioctl2* guarantees extra_in is
    // valid for extra_in_size bytes for the duration of this call.
    let extra_in_buf =
        unsafe { InputBuffer::from_raw_parts(extra_in, extra_in_size, BufferMode::Normal) };
    let req = builder.add_in_auto_buffer(extra_in_buf).build();

    req.send(&mut buf, session)
        .map_err(Ioctl2Error::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(Ioctl2Error::ParseResponse)?;
    let error = *resp.payload;

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
    session: BorrowedSessionHandle<'_>,
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    // Auto buffers in order: argp in (if applicable), argp out (if applicable), extra
    // out. The in/out descriptor lists are tracked independently by the builder, so
    // folding argp's in and out halves into one inout descriptor here does not
    // disturb that order.
    let mut builder = cmif::CmifRequestBuilder::new(nv_cmds::IOCTL3).with_data_value(&input);
    builder = match (in_size > 0, out_size > 0) {
        (true, true) => {
            // SAFETY: nv ioctl3 ABI - the caller of __nx_nv_ioctl3* guarantees argp is
            // valid for in_size (== out_size) bytes for the duration of this call, and
            // exclusively borrowed since the request reads then writes through it.
            let buf = unsafe { InOutBuffer::from_raw_parts(argp, in_size, BufferMode::Normal) };
            builder.add_inout_auto_buffer(buf)
        }
        (true, false) => {
            // SAFETY: nv ioctl3 ABI - the caller of __nx_nv_ioctl3* guarantees argp is
            // valid for in_size bytes for the duration of this call.
            let buf = unsafe { InputBuffer::from_raw_parts(argp, in_size, BufferMode::Normal) };
            builder.add_in_auto_buffer(buf)
        }
        (false, true) => {
            // SAFETY: nv ioctl3 ABI - the caller of __nx_nv_ioctl3* guarantees argp is
            // valid for out_size bytes for the duration of this call.
            let buf = unsafe { OutputBuffer::from_raw_parts(argp, out_size, BufferMode::Normal) };
            builder.add_out_auto_buffer(buf)
        }
        (false, false) => builder,
    };

    // SAFETY: nv ioctl3 ABI - the caller of __nx_nv_ioctl3* guarantees extra_out is
    // valid for extra_out_size bytes for the duration of this call.
    let extra_out_buf =
        unsafe { OutputBuffer::from_raw_parts(extra_out, extra_out_size, BufferMode::Normal) };
    let req = builder.add_out_auto_buffer(extra_out_buf).build();

    req.send(&mut buf, session)
        .map_err(Ioctl3Error::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(Ioctl3Error::ParseResponse)?;
    let error = *resp.payload;

    if error != 0 {
        return Err(Ioctl3Error::NvError(IoctlNvError::from_raw(error)));
    }

    Ok(())
}

/// Closes a device file descriptor.
///
/// This is INvDrvServices command 2.
pub fn close(session: BorrowedSessionHandle<'_>, fd: Fd) -> Result<(), CloseError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let fd_raw = fd.to_raw();
    let req = cmif::CmifRequestBuilder::new(nv_cmds::CLOSE)
        .with_data_value(&fd_raw)
        .build();
    req.send(&mut buf, session)
        .map_err(CloseError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(CloseError::ParseResponse)?;
    let error = *resp.payload;

    if error != 0 {
        return Err(CloseError::NvError(CloseNvError::from_raw(error)));
    }

    Ok(())
}

/// Initializes the NV service with transfer memory.
///
/// This is INvDrvServices command 3.
pub fn initialize(
    session: BorrowedSessionHandle<'_>,
    process_handle: ProcessHandle,
    tmem_handle: TmemHandle,
    tmem_size: u32,
) -> Result<(), InitializeError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(nv_cmds::INITIALIZE)
        .with_data_value(&tmem_size)
        .add_copy_handle(process_handle.to_raw())
        .add_copy_handle(tmem_handle.to_raw())
        .build();
    req.send(&mut buf, session)
        .map_err(InitializeError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(InitializeError::ParseResponse)?;

    Ok(())
}

/// Queries an event for a device.
///
/// This is INvDrvServices command 4.
pub fn query_event(
    session: BorrowedSessionHandle<'_>,
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(nv_cmds::QUERY_EVENT)
        .with_data_value(&input)
        .build();
    req.send(&mut buf, session)
        .map_err(QueryEventError::SendRequest)?;

    // Response contains error code (u32) and a copy handle for the event.
    let resp = cmif::parse_response::<&u32>(&buf).map_err(QueryEventError::ParseResponse)?;
    let error = *resp.payload;

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
pub fn set_client_pid(
    session: BorrowedSessionHandle<'_>,
    aruid: Aruid,
) -> Result<(), SetClientPidError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let aruid_raw = aruid.to_raw();
    let req = cmif::CmifRequestBuilder::new(nv_cmds::SET_CLIENT_PID)
        .with_data_value(&aruid_raw)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(SetClientPidError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(SetClientPidError::ParseResponse)?;

    Ok(())
}

/// Error returned by open operation.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] OpenNvError),
}

impl ToResultCode for OpenError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            Self::NvError(err) => err.to_rc(),
        }
    }
}

/// Error returned by ioctl operation.
#[derive(Debug, thiserror::Error)]
pub enum IoctlError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] IoctlNvError),
}

impl ToResultCode for IoctlError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            Self::NvError(err) => err.to_rc(),
        }
    }
}

/// Error returned by ioctl2 operation.
#[derive(Debug, thiserror::Error)]
pub enum Ioctl2Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] IoctlNvError),
}

impl ToResultCode for Ioctl2Error {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            Self::NvError(err) => err.to_rc(),
        }
    }
}

/// Error returned by ioctl3 operation.
#[derive(Debug, thiserror::Error)]
pub enum Ioctl3Error {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] IoctlNvError),
}

impl ToResultCode for Ioctl3Error {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            Self::NvError(err) => err.to_rc(),
        }
    }
}

/// Error returned by close operation.
#[derive(Debug, thiserror::Error)]
pub enum CloseError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] CloseNvError),
}

impl ToResultCode for CloseError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            Self::NvError(err) => err.to_rc(),
        }
    }
}

/// Error returned by initialize operation.
#[derive(Debug, thiserror::Error)]
pub enum InitializeError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for InitializeError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}

/// Error returned by query_event operation.
#[derive(Debug, thiserror::Error)]
pub enum QueryEventError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// NV driver returned an error.
    #[error("NV driver error")]
    NvError(#[source] QueryEventNvError),
    /// Missing event handle in response.
    #[error("missing event handle in response")]
    MissingHandle,
}

impl ToResultCode for QueryEventError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
            Self::NvError(err) => err.to_rc(),
            // Rejected locally after a successful reply, so no server
            // named a code for it.
            Self::MissingHandle => GENERIC_ERROR,
        }
    }
}

/// Error returned by set_client_pid operation.
#[derive(Debug, thiserror::Error)]
pub enum SetClientPidError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

impl ToResultCode for SetClientPidError {
    fn to_rc(self) -> ResultCode {
        match self {
            Self::SendRequest(err) => err.to_rc(),
            Self::ParseResponse(err) => err.to_rc(),
        }
    }
}
