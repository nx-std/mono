//! CMIF protocol operations for the sfdnsres service.
//!
//! Each function maps one-to-one to a libnx `sfdnsres*Request` entry point.
//! Output payloads that live in the caller-provided byte buffer (hostent /
//! addrinfo wire format) are returned as a serialized byte count; decoding
//! the wire format is the caller's responsibility.

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        InputBuffer,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};

use crate::proto::{
    CMD_CANCEL,
    CMD_GET_ADDR_INFO,
    CMD_GET_CANCEL_HANDLE,
    CMD_GET_GAI_STRING_ERROR,
    CMD_GET_HOST_BY_ADDR,
    CMD_GET_HOST_BY_NAME,
    CMD_GET_HOST_STRING_ERROR,
    CMD_GET_NAME_INFO,
    CancelHandle,
    CancelIn,
    GetAddrInfoIn,
    GetAddrInfoOut,
    GetHostByAddrIn,
    GetHostByAddrOut,
    GetHostByNameIn,
    GetHostByNameOut,
    GetNameInfoIn,
    GetNameInfoOut,
};

/// Encoded `0` for "no cancel token".
const NO_CANCEL: u32 = 0;

/// Result of `GetHostByNameRequest` (cmd 2).
#[derive(Debug, Clone, Copy)]
pub struct GetHostByNameResult {
    /// `h_errno` value from the resolver.
    pub h_errno: u32,
    /// `errno` value from the resolver.
    pub errno: u32,
    /// Number of bytes written to the output buffer.
    pub serialized_size: u32,
}

/// Resolves a host name into the wire-format hostent buffer.
///
/// The caller-supplied `out_buffer` receives the serialized hostent;
/// `serialized_size` indicates how many bytes are valid.
///
/// `name` is passed as a NUL-terminated byte slice (libnx sends `strlen + 1`).
/// Pass `None` to send a zero-length buffer (libnx allows a null pointer).
pub fn get_host_by_name(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    use_nsd: bool,
    name: Option<&[u8]>,
    out_buffer: &mut [u8],
) -> Result<GetHostByNameResult, GetHostByNameError> {
    let input = GetHostByNameIn {
        use_nsd: u32::from(use_nsd),
        cancel_handle: cancel_token(cancel_handle),
        pid_placeholder: 0,
    };

    let name_slice = name.unwrap_or(&[]);

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(CMD_GET_HOST_BY_NAME)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer(InputBuffer::new(name_slice, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(out_buffer, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetHostByNameError::SendRequest)?;

    let resp = cmif::parse_response::<&GetHostByNameOut>(&buf)
        .map_err(GetHostByNameError::ParseResponse)?;

    let out = *resp.payload;

    Ok(GetHostByNameResult {
        h_errno: out.h_errno,
        errno: out.errno,
        serialized_size: out.serialized_size,
    })
}

/// Error returned by [`get_host_by_name`].
#[derive(Debug, thiserror::Error)]
pub enum GetHostByNameError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Result of `GetHostByAddrRequest` (cmd 3).
#[derive(Debug, Clone, Copy)]
pub struct GetHostByAddrResult {
    /// `h_errno` value from the resolver.
    pub h_errno: u32,
    /// `errno` value from the resolver.
    pub errno: u32,
    /// Number of bytes written to the output buffer.
    pub serialized_size: u32,
}

/// Reverse-resolves an address into the wire-format hostent buffer.
pub fn get_host_by_addr(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    addr_type: u32,
    addr: &[u8],
    out_buffer: &mut [u8],
) -> Result<GetHostByAddrResult, GetHostByAddrError> {
    let input = GetHostByAddrIn {
        addr_len: addr.len() as u32,
        addr_type,
        cancel_handle: cancel_token(cancel_handle),
        _padding: 0,
        pid_placeholder: 0,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(CMD_GET_HOST_BY_ADDR)
        .with_data_value(&input)
        .add_input_buffer(InputBuffer::new(addr, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(out_buffer, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetHostByAddrError::SendRequest)?;

    let resp = cmif::parse_response::<&GetHostByAddrOut>(&buf)
        .map_err(GetHostByAddrError::ParseResponse)?;

    let out = *resp.payload;

    Ok(GetHostByAddrResult {
        h_errno: out.h_errno,
        errno: out.errno,
        serialized_size: out.serialized_size,
    })
}

/// Error returned by [`get_host_by_addr`].
#[derive(Debug, thiserror::Error)]
pub enum GetHostByAddrError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Writes the textual description of an `h_errno` value into `out_str`.
pub fn get_host_string_error(
    session: BorrowedSessionHandle<'_>,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), GetHostStringErrorError> {
    string_error_impl(session, CMD_GET_HOST_STRING_ERROR, err, out_str).map_err(|err| match err {
        StringErrorError::SendRequest(err) => GetHostStringErrorError::SendRequest(err),
        StringErrorError::ParseResponse(err) => GetHostStringErrorError::ParseResponse(err),
    })
}

/// Error returned by [`get_host_string_error`].
#[derive(Debug, thiserror::Error)]
pub enum GetHostStringErrorError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Writes the textual description of a `getaddrinfo` error code into `out_str`.
pub fn get_gai_string_error(
    session: BorrowedSessionHandle<'_>,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), GetGaiStringErrorError> {
    string_error_impl(session, CMD_GET_GAI_STRING_ERROR, err, out_str).map_err(|err| match err {
        StringErrorError::SendRequest(err) => GetGaiStringErrorError::SendRequest(err),
        StringErrorError::ParseResponse(err) => GetGaiStringErrorError::ParseResponse(err),
    })
}

/// Error returned by [`get_gai_string_error`].
#[derive(Debug, thiserror::Error)]
pub enum GetGaiStringErrorError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Result of `GetAddrInfoRequest` (cmd 6).
#[derive(Debug, Clone, Copy)]
pub struct GetAddrInfoResult {
    /// `errno` value from the resolver.
    pub errno: u32,
    /// `getaddrinfo` return code.
    pub ret: i32,
    /// Number of bytes written to the output buffer.
    pub serialized_size: u32,
}

/// Performs a `getaddrinfo`-style resolution.
///
/// `node`, `service` are NUL-terminated byte slices (or `None` for a null
/// pointer with zero length). `hints` is a serialized addrinfo template.
pub fn get_addr_info(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    use_nsd: bool,
    node: Option<&[u8]>,
    service: Option<&[u8]>,
    hints: Option<&[u8]>,
    out_buffer: &mut [u8],
) -> Result<GetAddrInfoResult, GetAddrInfoError> {
    let input = GetAddrInfoIn {
        use_nsd: u32::from(use_nsd),
        cancel_handle: cancel_token(cancel_handle),
        pid_placeholder: 0,
    };

    let node_slice = node.unwrap_or(&[]);
    let svc_slice = service.unwrap_or(&[]);
    let hints_slice = hints.unwrap_or(&[]);

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(CMD_GET_ADDR_INFO)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer(InputBuffer::new(node_slice, BufferMode::Normal))
        .add_input_buffer(InputBuffer::new(svc_slice, BufferMode::Normal))
        .add_input_buffer(InputBuffer::new(hints_slice, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(out_buffer, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetAddrInfoError::SendRequest)?;

    let resp =
        cmif::parse_response::<&GetAddrInfoOut>(&buf).map_err(GetAddrInfoError::ParseResponse)?;

    let out = *resp.payload;

    Ok(GetAddrInfoResult {
        errno: out.errno,
        ret: out.ret,
        serialized_size: out.serialized_size,
    })
}

/// Error returned by [`get_addr_info`].
#[derive(Debug, thiserror::Error)]
pub enum GetAddrInfoError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Result of `GetNameInfoRequest` (cmd 7).
#[derive(Debug, Clone, Copy)]
pub struct GetNameInfoResult {
    /// `errno` value from the resolver.
    pub errno: u32,
    /// `getnameinfo` return code.
    pub ret: i32,
}

/// Performs a `getnameinfo`-style reverse lookup, populating `host` and `serv`.
pub fn get_name_info(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    flags: u32,
    sockaddr: &[u8],
    host: &mut [u8],
    serv: &mut [u8],
) -> Result<GetNameInfoResult, GetNameInfoError> {
    let input = GetNameInfoIn {
        flags,
        cancel_handle: cancel_token(cancel_handle),
        pid_placeholder: 0,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(CMD_GET_NAME_INFO)
        .with_data_value(&input)
        .with_send_pid()
        .add_input_buffer(InputBuffer::new(sockaddr, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(host, BufferMode::Normal))
        .add_output_buffer(OutputBuffer::new(serv, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(GetNameInfoError::SendRequest)?;

    let resp =
        cmif::parse_response::<&GetNameInfoOut>(&buf).map_err(GetNameInfoError::ParseResponse)?;

    let out = *resp.payload;

    Ok(GetNameInfoResult {
        errno: out.errno,
        ret: out.ret,
    })
}

/// Error returned by [`get_name_info`].
#[derive(Debug, thiserror::Error)]
pub enum GetNameInfoError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Allocates a fresh cancel-token from the service.
pub fn get_cancel_handle(
    session: BorrowedSessionHandle<'_>,
) -> Result<CancelHandle, GetCancelHandleError> {
    // libnx encodes the input as a `u64 pid_placeholder` so the request still
    // carries an 8-byte payload alongside the send-PID flag.
    // SAFETY: IPC operations are serialized on this thread, so no other
    // borrow of the TLS IPC buffer is live.
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let pid_placeholder: u64 = 0;
    let req = cmif::CmifRequestBuilder::new(CMD_GET_CANCEL_HANDLE)
        .with_data_value(&pid_placeholder)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(GetCancelHandleError::SendRequest)?;

    let resp = cmif::parse_response::<&u32>(&buf).map_err(GetCancelHandleError::ParseResponse)?;

    let raw = *resp.payload;

    Ok(CancelHandle::from_raw(raw))
}

/// Error returned by [`get_cancel_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetCancelHandleError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

/// Cancels any pending resolver call tagged with `handle`.
pub fn cancel(session: BorrowedSessionHandle<'_>, handle: CancelHandle) -> Result<(), CancelError> {
    let input = CancelIn {
        cancel_handle: handle.to_raw(),
        _padding: 0,
        pid_placeholder: 0,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(CMD_CANCEL)
        .with_data_value(&input)
        .with_send_pid()
        .build();
    req.send(&mut buf, session)
        .map_err(CancelError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(CancelError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`cancel`].
#[derive(Debug, thiserror::Error)]
pub enum CancelError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
}

#[inline]
fn cancel_token(handle: Option<CancelHandle>) -> u32 {
    match handle {
        Some(h) => h.to_raw(),
        None => NO_CANCEL,
    }
}

enum StringErrorError {
    SendRequest(cmif::SendError),
    ParseResponse(cmif::ParseError),
}

fn string_error_impl(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), StringErrorError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&err)
        .add_output_buffer(OutputBuffer::new(out_str, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(StringErrorError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(StringErrorError::ParseResponse)?;

    Ok(())
}
