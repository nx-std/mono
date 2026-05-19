//! CMIF protocol operations for the sfdnsres service.
//!
//! Each function maps one-to-one to a libnx `sfdnsres*Request` entry point.
//! Output payloads that live in the caller-provided byte buffer (hostent /
//! addrinfo wire format) are returned as a serialized byte count; decoding
//! the wire format is the caller's responsibility.

use core::{mem::size_of, ptr};

use nx_sf::{cmif, hipc::BufferMode};
use nx_svc::ipc::{self, Handle as SessionHandle};

use crate::proto::{
    CMD_CANCEL, CMD_GET_ADDR_INFO, CMD_GET_CANCEL_HANDLE, CMD_GET_GAI_STRING_ERROR,
    CMD_GET_HOST_BY_ADDR, CMD_GET_HOST_BY_NAME, CMD_GET_HOST_STRING_ERROR, CMD_GET_NAME_INFO,
    CancelHandle, CancelIn, GetAddrInfoIn, GetAddrInfoOut, GetHostByAddrIn, GetHostByAddrOut,
    GetHostByNameIn, GetHostByNameOut, GetNameInfoIn, GetNameInfoOut,
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
    session: SessionHandle,
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

    let (name_ptr, name_len) = ptr_or_null(name);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, CMD_GET_HOST_BY_NAME)
            .data_size(size_of::<GetHostByNameIn>())
            .send_pid()
            .add_in_buffer(name_ptr, name_len, BufferMode::Normal)
            .add_out_buffer(
                out_buffer.as_mut_ptr(),
                out_buffer.len(),
                BufferMode::Normal,
            )
            .send()
            .map_err(GetHostByNameError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<GetHostByNameIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<GetHostByNameIn>(), input);
        }
    }

    ipc::send_sync_request(session).map_err(GetHostByNameError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<GetHostByNameOut>())
        .map_err(GetHostByNameError::ParseResponse)?;

    // SAFETY: resp.data points to a GetHostByNameOut-sized payload.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<GetHostByNameOut>()) };

    Ok(GetHostByNameResult {
        h_errno: out.h_errno,
        errno: out.errno,
        serialized_size: out.serialized_size,
    })
}

/// Error returned by [`get_host_by_name`].
#[derive(Debug, thiserror::Error)]
pub enum GetHostByNameError {
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
    session: SessionHandle,
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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, CMD_GET_HOST_BY_ADDR)
            .data_size(size_of::<GetHostByAddrIn>())
            .add_in_buffer(addr.as_ptr(), addr.len(), BufferMode::Normal)
            .add_out_buffer(
                out_buffer.as_mut_ptr(),
                out_buffer.len(),
                BufferMode::Normal,
            )
            .send()
            .map_err(GetHostByAddrError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<GetHostByAddrIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<GetHostByAddrIn>(), input);
        }
    }

    ipc::send_sync_request(session).map_err(GetHostByAddrError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<GetHostByAddrOut>())
        .map_err(GetHostByAddrError::ParseResponse)?;

    // SAFETY: resp.data points to a GetHostByAddrOut-sized payload.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<GetHostByAddrOut>()) };

    Ok(GetHostByAddrResult {
        h_errno: out.h_errno,
        errno: out.errno,
        serialized_size: out.serialized_size,
    })
}

/// Error returned by [`get_host_by_addr`].
#[derive(Debug, thiserror::Error)]
pub enum GetHostByAddrError {
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

/// Writes the textual description of an `h_errno` value into `out_str`.
pub fn get_host_string_error(
    session: SessionHandle,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), GetHostStringErrorError> {
    string_error_impl(session, CMD_GET_HOST_STRING_ERROR, err, out_str).map_err(|e| match e {
        StringErrorError::BuildRequest(e) => GetHostStringErrorError::BuildRequest(e),
        StringErrorError::SendRequest(e) => GetHostStringErrorError::SendRequest(e),
        StringErrorError::ParseResponse(e) => GetHostStringErrorError::ParseResponse(e),
    })
}

/// Error returned by [`get_host_string_error`].
#[derive(Debug, thiserror::Error)]
pub enum GetHostStringErrorError {
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

/// Writes the textual description of a `getaddrinfo` error code into `out_str`.
pub fn get_gai_string_error(
    session: SessionHandle,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), GetGaiStringErrorError> {
    string_error_impl(session, CMD_GET_GAI_STRING_ERROR, err, out_str).map_err(|e| match e {
        StringErrorError::BuildRequest(e) => GetGaiStringErrorError::BuildRequest(e),
        StringErrorError::SendRequest(e) => GetGaiStringErrorError::SendRequest(e),
        StringErrorError::ParseResponse(e) => GetGaiStringErrorError::ParseResponse(e),
    })
}

/// Error returned by [`get_gai_string_error`].
#[derive(Debug, thiserror::Error)]
pub enum GetGaiStringErrorError {
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
    session: SessionHandle,
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

    let (node_ptr, node_len) = ptr_or_null(node);
    let (svc_ptr, svc_len) = ptr_or_null(service);
    let (hints_ptr, hints_len) = ptr_or_null(hints);

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, CMD_GET_ADDR_INFO)
            .data_size(size_of::<GetAddrInfoIn>())
            .send_pid()
            .add_in_buffer(node_ptr, node_len, BufferMode::Normal)
            .add_in_buffer(svc_ptr, svc_len, BufferMode::Normal)
            .add_in_buffer(hints_ptr, hints_len, BufferMode::Normal)
            .add_out_buffer(
                out_buffer.as_mut_ptr(),
                out_buffer.len(),
                BufferMode::Normal,
            )
            .send()
            .map_err(GetAddrInfoError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<GetAddrInfoIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<GetAddrInfoIn>(), input);
        }
    }

    ipc::send_sync_request(session).map_err(GetAddrInfoError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<GetAddrInfoOut>())
        .map_err(GetAddrInfoError::ParseResponse)?;

    // SAFETY: resp.data points to a GetAddrInfoOut-sized payload.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<GetAddrInfoOut>()) };

    Ok(GetAddrInfoResult {
        errno: out.errno,
        ret: out.ret,
        serialized_size: out.serialized_size,
    })
}

/// Error returned by [`get_addr_info`].
#[derive(Debug, thiserror::Error)]
pub enum GetAddrInfoError {
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
    session: SessionHandle,
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

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, CMD_GET_NAME_INFO)
            .data_size(size_of::<GetNameInfoIn>())
            .send_pid()
            .add_in_buffer(sockaddr.as_ptr(), sockaddr.len(), BufferMode::Normal)
            .add_out_buffer(host.as_mut_ptr(), host.len(), BufferMode::Normal)
            .add_out_buffer(serv.as_mut_ptr(), serv.len(), BufferMode::Normal)
            .send()
            .map_err(GetNameInfoError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<GetNameInfoIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<GetNameInfoIn>(), input);
        }
    }

    ipc::send_sync_request(session).map_err(GetNameInfoError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<GetNameInfoOut>())
        .map_err(GetNameInfoError::ParseResponse)?;

    // SAFETY: resp.data points to a GetNameInfoOut-sized payload.
    let out = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<GetNameInfoOut>()) };

    Ok(GetNameInfoResult {
        errno: out.errno,
        ret: out.ret,
    })
}

/// Error returned by [`get_name_info`].
#[derive(Debug, thiserror::Error)]
pub enum GetNameInfoError {
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

/// Allocates a fresh cancel-token from the service.
pub fn get_cancel_handle(session: SessionHandle) -> Result<CancelHandle, GetCancelHandleError> {
    // libnx encodes the input as a `u64 pid_placeholder` so the request still
    // carries an 8-byte payload alongside the send-PID flag.
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, CMD_GET_CANCEL_HANDLE)
            .data_size(size_of::<u64>())
            .send_pid()
            .send()
            .map_err(GetCancelHandleError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u64>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), 0u64);
        }
    }

    ipc::send_sync_request(session).map_err(GetCancelHandleError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<u32>())
        .map_err(GetCancelHandleError::ParseResponse)?;

    // SAFETY: resp.data points to a u32 payload.
    let raw = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u32>()) };

    Ok(CancelHandle::from_raw(raw))
}

/// Error returned by [`get_cancel_handle`].
#[derive(Debug, thiserror::Error)]
pub enum GetCancelHandleError {
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

/// Cancels any pending resolver call tagged with `handle`.
pub fn cancel(session: SessionHandle, handle: CancelHandle) -> Result<(), CancelError> {
    let input = CancelIn {
        cancel_handle: handle.to_raw(),
        _padding: 0,
        pid_placeholder: 0,
    };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, CMD_CANCEL)
            .data_size(size_of::<CancelIn>())
            .send_pid()
            .send()
            .map_err(CancelError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<CancelIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<CancelIn>(), input);
        }
    }

    ipc::send_sync_request(session).map_err(CancelError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(CancelError::ParseResponse)?;

    Ok(())
}

/// Error returned by [`cancel`].
#[derive(Debug, thiserror::Error)]
pub enum CancelError {
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

#[inline]
fn cancel_token(handle: Option<CancelHandle>) -> u32 {
    match handle {
        Some(h) => h.to_raw(),
        None => NO_CANCEL,
    }
}

#[inline]
fn ptr_or_null<T>(slice: Option<&[T]>) -> (*const u8, usize) {
    match slice {
        Some(s) => (s.as_ptr().cast::<u8>(), core::mem::size_of_val(s)),
        None => (ptr::null(), 0),
    }
}

enum StringErrorError {
    BuildRequest(cmif::RequestLayoutError),
    SendRequest(ipc::SendSyncError),
    ParseResponse(cmif::ParseRespBytesError),
}

fn string_error_impl(
    session: SessionHandle,
    cmd_id: u32,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), StringErrorError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifBuilder::new(&mut buf, cmd_id)
            .data_size(size_of::<u32>())
            .add_out_buffer(out_str.as_mut_ptr(), out_str.len(), BufferMode::Normal)
            .send()
            .map_err(StringErrorError::BuildRequest)?;

        // SAFETY: `req.data` is exactly `size_of::<u32>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<u32>(), err);
        }
    }

    ipc::send_sync_request(session).map_err(StringErrorError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(StringErrorError::ParseResponse)?;

    Ok(())
}
