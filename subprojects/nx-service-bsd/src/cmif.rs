//! CMIF protocol operations for the BSD socket service.
//!
//! Every command maps one-to-one to a `_bsd*` entry point in libnx's
//! `bsd.c`. Each function is `pub(crate)` so [`crate::BsdService`] can compose
//! them after acquiring a session from the pool.
//!
//! Non-init commands all share the same response prefix `{ int ret; int errno }`
//! (see libnx's `_bsdDispatchImpl`). When the service returns `ret < 0`, the
//! caller is informed via the corresponding `Service { ret, errno }` variant in
//! the per-command error enum — no thread-local `errno` is maintained.

use core::{mem::size_of, ptr};

use nx_sf::{
    cmif::{self, ParseRespBytesError},
    hipc::BufferMode,
};
use nx_svc::{
    ipc::{self, Handle as SessionHandle, SendSyncError},
    mem::tmem::Handle as TmemHandle,
};

use crate::{
    fd::BsdSockFd,
    proto::{
        BsdServiceConfigWire, CallResponse, FcntlIn, IoctlIn, ListenIn, PollIn, RegisterClientIn,
        SelectIn, SelectTimeval, ShutdownIn, SockOptIn, SocketIn, SockfdFlagsIn, Timeval, cmds,
    },
    types::BsdConfig,
};

// ---------------------------------------------------------------------------
// Init / handshake (cmd 0 and 1)
// ---------------------------------------------------------------------------

/// Sends `IBsdServices::RegisterClient` (cmd 0) on `session`.
///
/// Returns the client PID the service assigned, which must subsequently be
/// passed to [`start_monitoring`] on the monitor session.
pub(crate) fn register_client(
    session: SessionHandle,
    config: &BsdConfig,
    tmem_handle: TmemHandle,
    tmem_size: u64,
) -> Result<u64, RegisterClientError> {
    let payload = RegisterClientIn {
        config: BsdServiceConfigWire {
            version: config.version,
            tcp_tx_buf_size: config.tcp_tx_buf_size,
            tcp_rx_buf_size: config.tcp_rx_buf_size,
            tcp_tx_buf_max_size: config.tcp_tx_buf_max_size,
            tcp_rx_buf_max_size: config.tcp_rx_buf_max_size,
            udp_tx_buf_size: config.udp_tx_buf_size,
            udp_rx_buf_size: config.udp_rx_buf_size,
            sb_efficiency: config.sb_efficiency,
        },
        pid_placeholder: 0,
        tmem_size,
    };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::REGISTER_CLIENT)
            .data_size(size_of::<RegisterClientIn>())
            .send_pid()
            .add_copy_handle(tmem_handle.to_raw())
            .send(&mut buf)
            .map_err(RegisterClientError::BuildRequest)?;
        // SAFETY: `req.data` is exactly `size_of::<RegisterClientIn>()` bytes.
        unsafe {
            ptr::write_unaligned(req.data.as_mut_ptr().cast::<RegisterClientIn>(), payload);
        }
    }

    ipc::send_sync_request(session).map_err(RegisterClientError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    // The reply is a CMIF `u64 pid` payload — no extra `{ ret; errno; }` prefix here
    // because cmd 0 / cmd 1 don't use the dispatch path that adds it.
    let resp = cmif::parse_response_bytes(&buf, size_of::<u64>())
        .map_err(RegisterClientError::ParseResponse)?;

    // SAFETY: resp.data points to at least size_of::<u64>() bytes.
    let pid = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<u64>()) };
    Ok(pid)
}

/// Sends `IBsdServices::StartMonitoring` (cmd 1) on `monitor_session`.
pub(crate) fn start_monitoring(
    monitor_session: SessionHandle,
    pid: u64,
) -> Result<(), StartMonitoringError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::START_MONITORING)
            .data_size(size_of::<u64>())
            .send_pid()
            .send(&mut buf)
            .map_err(StartMonitoringError::BuildRequest)?;
        // SAFETY: req.data has 8 bytes reserved for the pid payload.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), pid) };
    }

    ipc::send_sync_request(monitor_session).map_err(StartMonitoringError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer during the SVC above, and
    // no other borrow of the buffer is live on this thread.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    cmif::parse_response_bytes(&buf, 0).map_err(StartMonitoringError::ParseResponse)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared response decoding for `_bsdDispatchImpl`-style commands
// ---------------------------------------------------------------------------

/// Outcome of decoding the `{ ret; errno; [extra u32] }` reply.
struct ServiceOutcome {
    ret: i32,
    /// Optional extra `u32` trailing the `CallResponse` prefix (e.g. addrlen,
    /// optlen). `None` when the command emits no trailing payload.
    extra_u32: Option<u32>,
}

/// Reads the standard BSD response prefix from the TLS buffer.
///
/// On `ret >= 0`, returns `ServiceOutcome`. On `ret < 0`, returns
/// `Err(ServiceResponseFailure)`.
///
/// `has_extra_u32` controls whether a trailing `u32` after the
/// `CallResponse` header is read and returned in `ServiceOutcome::extra_u32`.
///
/// # Safety
///
/// Must be called right after a successful `send_sync_request`. No other
/// borrow of the TLS IPC buffer may be live when this is called.
unsafe fn read_service_response(
    has_extra_u32: bool,
) -> Result<ServiceOutcome, ServiceResponseFailure> {
    let extra_size = if has_extra_u32 { size_of::<u32>() } else { 0 };
    // SAFETY: caller upholds preconditions — no other borrow is live and the
    // kernel has populated the TLS IPC buffer during the SVC.
    let buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
    let resp = cmif::parse_response_bytes(&buf, size_of::<CallResponse>() + extra_size)
        .map_err(ServiceResponseFailure::Parse)?;

    // SAFETY: parse_response_bytes guaranteed at least size_of::<CallResponse>()
    // bytes are valid in resp.data.
    let prefix = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<CallResponse>()) };

    if prefix.ret < 0 {
        return Err(ServiceResponseFailure::Service {
            ret: prefix.ret,
            errno: prefix.errno,
        });
    }

    let extra_u32 = if has_extra_u32 {
        let extra_bytes = &resp.data[size_of::<CallResponse>()..];
        // SAFETY: parse_response_bytes guarantees `extra_size` bytes are valid.
        Some(unsafe { ptr::read_unaligned(extra_bytes.as_ptr().cast::<u32>()) })
    } else {
        None
    };

    Ok(ServiceOutcome {
        ret: prefix.ret,
        extra_u32,
    })
}

enum ServiceResponseFailure {
    Parse(ParseRespBytesError),
    Service { ret: i32, errno: i32 },
}

// ---------------------------------------------------------------------------
// Socket creation / teardown
// ---------------------------------------------------------------------------

/// `bsdSocket` (cmd 2).
pub(crate) fn socket(
    session: SessionHandle,
    domain: i32,
    type_: i32,
    protocol: i32,
) -> Result<BsdSockFd, SocketError> {
    let payload = SocketIn {
        domain,
        type_,
        protocol,
    };
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SOCKET)
            .data_size(size_of::<SocketIn>())
            .send(&mut buf)
            .map_err(SocketError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SocketIn.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SocketIn>(), payload) };
    }

    ipc::send_sync_request(session).map_err(SocketError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SocketError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SocketError::Service { ret, errno },
    })?;
    Ok(BsdSockFd::from_raw(outcome.ret))
}

/// `bsdClose` (cmd 26).
pub(crate) fn close(session: SessionHandle, fd: BsdSockFd) -> Result<(), CloseError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::CLOSE)
            .data_size(size_of::<i32>())
            .send(&mut buf)
            .map_err(CloseError::BuildRequest)?;
        // SAFETY: req.data has 4 bytes reserved for the fd.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), fd.raw()) };
    }

    ipc::send_sync_request(session).map_err(CloseError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let _ = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => CloseError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => CloseError::Service { ret, errno },
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Address-binding (bind/connect) and accept/getsockname/getpeername
// ---------------------------------------------------------------------------

/// `bsdBind` (cmd 13).
pub(crate) fn bind(
    session: SessionHandle,
    sockfd: BsdSockFd,
    addr: &[u8],
) -> Result<(), BindError> {
    bsd_send_recv_no_buffer_in(
        session,
        cmds::BIND,
        sockfd,
        addr,
        BindError::BuildRequest,
        BindError::SendRequest,
        BindError::ParseResponse,
        |ret, errno| BindError::Service { ret, errno },
    )
    .map(|_| ())
}

/// `bsdConnect` (cmd 14).
pub(crate) fn connect(
    session: SessionHandle,
    sockfd: BsdSockFd,
    addr: &[u8],
) -> Result<(), ConnectError> {
    bsd_send_recv_no_buffer_in(
        session,
        cmds::CONNECT,
        sockfd,
        addr,
        ConnectError::BuildRequest,
        ConnectError::SendRequest,
        ConnectError::ParseResponse,
        |ret, errno| ConnectError::Service { ret, errno },
    )
    .map(|_| ())
}

/// Shared `bind`/`connect` body — sends `sockfd` plus an in-only sockaddr
/// buffer, then drops the standard `{ ret; errno }` reply on the floor.
#[allow(clippy::too_many_arguments)]
fn bsd_send_recv_no_buffer_in<E>(
    session: SessionHandle,
    cmd_id: u32,
    sockfd: BsdSockFd,
    addr: &[u8],
    mk_build: fn(cmif::RequestLayoutError) -> E,
    mk_send: fn(SendSyncError) -> E,
    mk_parse: fn(ParseRespBytesError) -> E,
    mk_service: fn(i32, i32) -> E,
) -> Result<i32, E> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmd_id)
            .data_size(size_of::<i32>())
            .add_in_auto_buffer(addr.as_ptr(), addr.len(), BufferMode::Normal)
            .send(&mut buf)
            .map_err(mk_build)?;
        // SAFETY: req.data has 4 bytes reserved for sockfd.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), sockfd.raw()) };
    }

    ipc::send_sync_request(session).map_err(mk_send)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => mk_parse(err),
        ServiceResponseFailure::Service { ret, errno } => mk_service(ret, errno),
    })?;
    Ok(outcome.ret)
}

/// Shared `accept`/`getsockname`/`getpeername` body — sends `sockfd` and an
/// out-only sockaddr buffer, then reads back the actual `socklen_t` length.
#[allow(clippy::too_many_arguments)]
fn bsd_cmd_in_sockfd_out_sockaddr<E>(
    session: SessionHandle,
    cmd_id: u32,
    sockfd: BsdSockFd,
    addr_buf: &mut [u8],
    mk_build: fn(cmif::RequestLayoutError) -> E,
    mk_send: fn(SendSyncError) -> E,
    mk_parse: fn(ParseRespBytesError) -> E,
    mk_service: fn(i32, i32) -> E,
) -> Result<(i32, u32), E> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmd_id)
            .data_size(size_of::<i32>())
            .add_out_auto_buffer(addr_buf.as_mut_ptr(), addr_buf.len(), BufferMode::Normal)
            .send(&mut buf)
            .map_err(mk_build)?;
        // SAFETY: req.data has 4 bytes reserved for sockfd.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), sockfd.raw()) };
    }

    ipc::send_sync_request(session).map_err(mk_send)?;
    // SAFETY: the kernel populated the TLS IPC buffer; out_data carries one u32 with the addrlen.
    let outcome = unsafe { read_service_response(true) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => mk_parse(err),
        ServiceResponseFailure::Service { ret, errno } => mk_service(ret, errno),
    })?;

    let addrlen = outcome.extra_u32.unwrap_or(0);
    Ok((outcome.ret, addrlen))
}

/// `bsdAccept` (cmd 12). Returns the new socket fd and the actual `socklen_t`
/// length written into `addr_buf`.
pub(crate) fn accept(
    session: SessionHandle,
    sockfd: BsdSockFd,
    addr_buf: &mut [u8],
) -> Result<(BsdSockFd, u32), AcceptError> {
    let (ret, addrlen) = bsd_cmd_in_sockfd_out_sockaddr(
        session,
        cmds::ACCEPT,
        sockfd,
        addr_buf,
        AcceptError::BuildRequest,
        AcceptError::SendRequest,
        AcceptError::ParseResponse,
        |ret, errno| AcceptError::Service { ret, errno },
    )?;
    Ok((BsdSockFd::from_raw(ret), addrlen))
}

/// `bsdGetSockName` (cmd 16). Returns the actual `socklen_t` length.
pub(crate) fn get_sock_name(
    session: SessionHandle,
    sockfd: BsdSockFd,
    addr_buf: &mut [u8],
) -> Result<u32, GetSockNameError> {
    let (_ret, addrlen) = bsd_cmd_in_sockfd_out_sockaddr(
        session,
        cmds::GET_SOCK_NAME,
        sockfd,
        addr_buf,
        GetSockNameError::BuildRequest,
        GetSockNameError::SendRequest,
        GetSockNameError::ParseResponse,
        |ret, errno| GetSockNameError::Service { ret, errno },
    )?;
    Ok(addrlen)
}

/// `bsdGetPeerName` (cmd 15). Returns the actual `socklen_t` length.
pub(crate) fn get_peer_name(
    session: SessionHandle,
    sockfd: BsdSockFd,
    addr_buf: &mut [u8],
) -> Result<u32, GetPeerNameError> {
    let (_ret, addrlen) = bsd_cmd_in_sockfd_out_sockaddr(
        session,
        cmds::GET_PEER_NAME,
        sockfd,
        addr_buf,
        GetPeerNameError::BuildRequest,
        GetPeerNameError::SendRequest,
        GetPeerNameError::ParseResponse,
        |ret, errno| GetPeerNameError::Service { ret, errno },
    )?;
    Ok(addrlen)
}

// ---------------------------------------------------------------------------
// Listen / Shutdown
// ---------------------------------------------------------------------------

/// `bsdListen` (cmd 18).
pub(crate) fn listen(
    session: SessionHandle,
    sockfd: BsdSockFd,
    backlog: i32,
) -> Result<(), ListenError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::LISTEN)
            .data_size(size_of::<ListenIn>())
            .send(&mut buf)
            .map_err(ListenError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for ListenIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<ListenIn>(),
                ListenIn {
                    sockfd: sockfd.raw(),
                    backlog,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(ListenError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let _ = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => ListenError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => ListenError::Service { ret, errno },
    })?;
    Ok(())
}

/// `bsdShutdown` (cmd 22).
pub(crate) fn shutdown(
    session: SessionHandle,
    sockfd: BsdSockFd,
    how: i32,
) -> Result<(), ShutdownError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SHUTDOWN)
            .data_size(size_of::<ShutdownIn>())
            .send(&mut buf)
            .map_err(ShutdownError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for ShutdownIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<ShutdownIn>(),
                ShutdownIn {
                    sockfd: sockfd.raw(),
                    how,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(ShutdownError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let _ = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => ShutdownError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => ShutdownError::Service { ret, errno },
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Recv / Send / Read / Write
// ---------------------------------------------------------------------------

/// `bsdRecv` (cmd 8).
pub(crate) fn recv(
    session: SessionHandle,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    flags: i32,
) -> Result<usize, RecvError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::RECV)
            .data_size(size_of::<SockfdFlagsIn>())
            .add_out_auto_buffer(buf.as_mut_ptr(), buf.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(RecvError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SockfdFlagsIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<SockfdFlagsIn>(),
                SockfdFlagsIn {
                    sockfd: sockfd.raw(),
                    flags,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(RecvError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => RecvError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => RecvError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdRecvFrom` (cmd 9). Returns `(bytes_received, actual_src_addr_len)`.
pub(crate) fn recv_from(
    session: SessionHandle,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    flags: i32,
    src_addr: &mut [u8],
) -> Result<(usize, u32), RecvFromError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::RECV_FROM)
            .data_size(size_of::<SockfdFlagsIn>())
            .add_out_auto_buffer(buf.as_mut_ptr(), buf.len(), BufferMode::Normal)
            .add_out_auto_buffer(src_addr.as_mut_ptr(), src_addr.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(RecvFromError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SockfdFlagsIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<SockfdFlagsIn>(),
                SockfdFlagsIn {
                    sockfd: sockfd.raw(),
                    flags,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(RecvFromError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; trailing payload is one u32 addrlen.
    let outcome = unsafe { read_service_response(true) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => RecvFromError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => RecvFromError::Service { ret, errno },
    })?;
    let addrlen = outcome.extra_u32.unwrap_or(0);
    Ok((outcome.ret as usize, addrlen))
}

/// `bsdSend` (cmd 10).
pub(crate) fn send(
    session: SessionHandle,
    sockfd: BsdSockFd,
    buf: &[u8],
    flags: i32,
) -> Result<usize, SendError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SEND)
            .data_size(size_of::<SockfdFlagsIn>())
            .add_in_auto_buffer(buf.as_ptr(), buf.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(SendError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SockfdFlagsIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<SockfdFlagsIn>(),
                SockfdFlagsIn {
                    sockfd: sockfd.raw(),
                    flags,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(SendError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SendError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SendError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdSendTo` (cmd 11).
pub(crate) fn send_to(
    session: SessionHandle,
    sockfd: BsdSockFd,
    buf: &[u8],
    flags: i32,
    dest_addr: &[u8],
) -> Result<usize, SendToError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SEND_TO)
            .data_size(size_of::<SockfdFlagsIn>())
            .add_in_auto_buffer(buf.as_ptr(), buf.len(), BufferMode::Normal)
            .add_in_auto_buffer(dest_addr.as_ptr(), dest_addr.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(SendToError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SockfdFlagsIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<SockfdFlagsIn>(),
                SockfdFlagsIn {
                    sockfd: sockfd.raw(),
                    flags,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(SendToError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SendToError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SendToError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdRead` (cmd 25).
pub(crate) fn read(
    session: SessionHandle,
    fd: BsdSockFd,
    buf: &mut [u8],
) -> Result<usize, ReadError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::READ)
            .data_size(size_of::<i32>())
            .add_out_auto_buffer(buf.as_mut_ptr(), buf.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(ReadError::BuildRequest)?;
        // SAFETY: req.data has 4 bytes reserved for the fd.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), fd.raw()) };
    }

    ipc::send_sync_request(session).map_err(ReadError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => ReadError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => ReadError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdWrite` (cmd 24).
pub(crate) fn write(
    session: SessionHandle,
    fd: BsdSockFd,
    buf: &[u8],
) -> Result<usize, WriteError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::WRITE)
            .data_size(size_of::<i32>())
            .add_in_auto_buffer(buf.as_ptr(), buf.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(WriteError::BuildRequest)?;
        // SAFETY: req.data has 4 bytes reserved for the fd.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), fd.raw()) };
    }

    ipc::send_sync_request(session).map_err(WriteError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => WriteError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => WriteError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

// ---------------------------------------------------------------------------
// Socket options
// ---------------------------------------------------------------------------

/// `bsdGetSockOpt` (cmd 17). Returns the actual `socklen_t` written.
pub(crate) fn get_sock_opt(
    session: SessionHandle,
    sockfd: BsdSockFd,
    level: i32,
    optname: i32,
    optval: &mut [u8],
) -> Result<u32, GetSockOptError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::GET_SOCK_OPT)
            .data_size(size_of::<SockOptIn>())
            .add_out_auto_buffer(optval.as_mut_ptr(), optval.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(GetSockOptError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SockOptIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<SockOptIn>(),
                SockOptIn {
                    sockfd: sockfd.raw(),
                    level,
                    optname,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(GetSockOptError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; one u32 socklen_t trails the prefix.
    let outcome = unsafe { read_service_response(true) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => GetSockOptError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => GetSockOptError::Service { ret, errno },
    })?;
    let optlen = outcome.extra_u32.unwrap_or(0);
    Ok(optlen)
}

/// `bsdSetSockOpt` (cmd 21).
pub(crate) fn set_sock_opt(
    session: SessionHandle,
    sockfd: BsdSockFd,
    level: i32,
    optname: i32,
    optval: &[u8],
) -> Result<(), SetSockOptError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SET_SOCK_OPT)
            .data_size(size_of::<SockOptIn>())
            .add_in_auto_buffer(optval.as_ptr(), optval.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(SetSockOptError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SockOptIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<SockOptIn>(),
                SockOptIn {
                    sockfd: sockfd.raw(),
                    level,
                    optname,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(SetSockOptError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let _ = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SetSockOptError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SetSockOptError::Service { ret, errno },
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Fcntl / Ioctl
// ---------------------------------------------------------------------------

/// `bsdFcntl` (cmd 20). libnx exposes only `F_GETFL` / `F_SETFL`; emulation of
/// other commands (e.g. returning `EOPNOTSUPP` without a server round-trip)
/// is left to the higher layer that wraps this function.
pub(crate) fn fcntl(
    session: SessionHandle,
    fd: BsdSockFd,
    cmd: i32,
    flags: i32,
) -> Result<i32, FcntlError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::FCNTL)
            .data_size(size_of::<FcntlIn>())
            .send(&mut ipc_buf)
            .map_err(FcntlError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for FcntlIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<FcntlIn>(),
                FcntlIn {
                    fd: fd.raw(),
                    cmd,
                    flags,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(FcntlError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => FcntlError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => FcntlError::Service { ret, errno },
    })?;
    Ok(outcome.ret)
}

/// `bsdIoctl` (cmd 19) — generic case only.
///
/// `request` encodes both the direction (via `IOC_IN` / `IOC_OUT` bits) and the
/// payload size (via `IOCPARM_LEN`). This port handles the **generic** path of
/// libnx's switch table — the `SIOCGIFCONF` / `SIOCGIFMEDIA` / `SIOCGIFXMEDIA`
/// special cases (which reach into the caller's buffer to find sub-buffers)
/// are not implemented yet.
pub(crate) fn ioctl(
    session: SessionHandle,
    fd: BsdSockFd,
    request: i32,
    data: &mut [u8],
) -> Result<i32, IoctlError> {
    // Direction + length encoded in the request code, libc-style.
    const IOC_IN: i32 = 0x4000_0000_u32 as i32;
    const IOC_OUT: i32 = 0x2000_0000_u32 as i32;
    const IOC_INOUT: i32 = IOC_IN | IOC_OUT;
    const IOCPARM_MASK: i32 = 0x1FFF;

    let has_in = (request & IOC_IN) != 0;
    let has_out = (request & IOC_OUT) != 0;
    let has_inout = (request & IOC_INOUT) != 0;
    let payload_len = (request >> 16) & IOCPARM_MASK;
    let payload_len = if has_inout {
        core::cmp::min(payload_len as usize, data.len())
    } else {
        0
    };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let mut builder =
            cmif::CmifRequestBuilder::new(cmds::IOCTL).data_size(size_of::<IoctlIn>());
        if has_in {
            builder = builder.add_in_auto_buffer(data.as_ptr(), payload_len, BufferMode::Normal);
        }
        if has_out {
            builder =
                builder.add_out_auto_buffer(data.as_mut_ptr(), payload_len, BufferMode::Normal);
        }
        let req = builder
            .send(&mut ipc_buf)
            .map_err(IoctlError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for IoctlIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<IoctlIn>(),
                IoctlIn {
                    fd: fd.raw(),
                    request,
                    bufcount: if has_inout { 1 } else { 0 },
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(IoctlError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => IoctlError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => IoctlError::Service { ret, errno },
    })?;
    Ok(outcome.ret)
}

// ---------------------------------------------------------------------------
// Select / Poll
// ---------------------------------------------------------------------------

/// Optional timeout for [`select`] (mirrors libnx's `BsdSelectTimeval`).
#[derive(Debug, Clone, Copy)]
pub struct SelectTimeout {
    /// Seconds component.
    pub sec: i64,
    /// Microseconds component.
    pub usec: i64,
}

/// `bsdSelect` (cmd 5).
///
/// Each `fd_set` buffer is opaque to this crate; callers are expected to use
/// libnx's `fd_set` byte layout. Pass empty slices for fd_sets that should be
/// transmitted as null. Pass `None` for `timeout` to send the libnx
/// `is_null=true` sentinel.
pub(crate) fn select(
    session: SessionHandle,
    nfds: i32,
    readfds: &mut [u8],
    writefds: &mut [u8],
    exceptfds: &mut [u8],
    timeout: Option<SelectTimeout>,
) -> Result<i32, SelectError> {
    let select_timeout = match timeout {
        Some(t) => SelectTimeval {
            tv: Timeval {
                tv_sec: t.sec,
                tv_usec: t.usec,
            },
            is_null: 0,
            _pad: [0; 7],
        },
        None => SelectTimeval {
            tv: Timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            is_null: 1,
            _pad: [0; 7],
        },
    };

    let payload = SelectIn {
        nfds,
        _pad: 0,
        timeout: select_timeout,
    };

    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::SELECT)
            .data_size(size_of::<SelectIn>())
            .add_in_auto_buffer(readfds.as_ptr(), readfds.len(), BufferMode::Normal)
            .add_in_auto_buffer(writefds.as_ptr(), writefds.len(), BufferMode::Normal)
            .add_in_auto_buffer(exceptfds.as_ptr(), exceptfds.len(), BufferMode::Normal)
            .add_out_auto_buffer(readfds.as_mut_ptr(), readfds.len(), BufferMode::Normal)
            .add_out_auto_buffer(writefds.as_mut_ptr(), writefds.len(), BufferMode::Normal)
            .add_out_auto_buffer(exceptfds.as_mut_ptr(), exceptfds.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(SelectError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for SelectIn.
        unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SelectIn>(), payload) };
    }

    ipc::send_sync_request(session).map_err(SelectError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SelectError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SelectError::Service { ret, errno },
    })?;
    Ok(outcome.ret)
}

/// `bsdPoll` (cmd 6). `fds` must have layout matching libnx's `pollfd` array;
/// it is read as input and written back as output.
pub(crate) fn poll(
    session: SessionHandle,
    fds: &mut [u8],
    nfds: u64,
    timeout: i32,
) -> Result<i32, PollError> {
    {
        // SAFETY: IPC operations are serialized on this thread, so no other
        // borrow of the TLS IPC buffer is live.
        let mut ipc_buf = unsafe { nx_sys_thread_tls::ipc_buffer() };
        let req = cmif::CmifRequestBuilder::new(cmds::POLL)
            .data_size(size_of::<PollIn>())
            .add_in_auto_buffer(fds.as_ptr(), fds.len(), BufferMode::Normal)
            .add_out_auto_buffer(fds.as_mut_ptr(), fds.len(), BufferMode::Normal)
            .send(&mut ipc_buf)
            .map_err(PollError::BuildRequest)?;
        // SAFETY: req.data has data_size bytes reserved for PollIn.
        unsafe {
            ptr::write_unaligned(
                req.data.as_mut_ptr().cast::<PollIn>(),
                PollIn {
                    nfds,
                    timeout,
                    _pad: 0,
                },
            );
        }
    }

    ipc::send_sync_request(session).map_err(PollError::SendRequest)?;
    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => PollError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => PollError::Service { ret, errno },
    })?;
    Ok(outcome.ret)
}

// ---------------------------------------------------------------------------
// Per-command error enums
// ---------------------------------------------------------------------------
//
// One enum per fallible function (per `errors-reporting.md` §10). Each enum
// carries three distinct sources: build failure, send failure, parse failure,
// and the service-level POSIX errno.

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to build the `RegisterClient` IPC request.
    #[error("failed to build register_client request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the `RegisterClient` IPC request.
    #[error("failed to send register_client request")]
    SendRequest(#[source] SendSyncError),
    /// The CMIF response could not be parsed.
    #[error("failed to parse register_client response")]
    ParseResponse(#[source] ParseRespBytesError),
}

/// Error returned by [`start_monitoring`].
#[derive(Debug, thiserror::Error)]
pub enum StartMonitoringError {
    /// Failed to build the `StartMonitoring` IPC request.
    #[error("failed to build start_monitoring request")]
    BuildRequest(#[source] cmif::RequestLayoutError),
    /// Failed to send the `StartMonitoring` IPC request.
    #[error("failed to send start_monitoring request")]
    SendRequest(#[source] SendSyncError),
    /// The CMIF response could not be parsed.
    #[error("failed to parse start_monitoring response")]
    ParseResponse(#[source] ParseRespBytesError),
}

macro_rules! define_per_command_error {
    ($name:ident, $build_msg:literal, $send_msg:literal, $parse_msg:literal, $svc_msg:literal) => {
        #[doc = concat!("Error returned by the corresponding BSD command.")]
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            /// Failed to build the IPC request.
            #[error($build_msg)]
            BuildRequest(#[source] cmif::RequestLayoutError),
            /// Failed to send the IPC request.
            #[error($send_msg)]
            SendRequest(#[source] SendSyncError),
            /// Failed to parse the CMIF response.
            #[error($parse_msg)]
            ParseResponse(#[source] ParseRespBytesError),
            /// The service returned a POSIX-domain failure. `errno` is the libc
            /// errno; `ret` is the raw value returned (typically `-1`).
            #[error($svc_msg)]
            Service { ret: i32, errno: i32 },
        }
    };
}

define_per_command_error!(
    SocketError,
    "failed to build socket request",
    "failed to send socket request",
    "failed to parse socket response",
    "bsd socket failed (errno={errno})"
);
define_per_command_error!(
    CloseError,
    "failed to build close request",
    "failed to send close request",
    "failed to parse close response",
    "bsd close failed (errno={errno})"
);
define_per_command_error!(
    BindError,
    "failed to build bind request",
    "failed to send bind request",
    "failed to parse bind response",
    "bsd bind failed (errno={errno})"
);
define_per_command_error!(
    ConnectError,
    "failed to build connect request",
    "failed to send connect request",
    "failed to parse connect response",
    "bsd connect failed (errno={errno})"
);
define_per_command_error!(
    ListenError,
    "failed to build listen request",
    "failed to send listen request",
    "failed to parse listen response",
    "bsd listen failed (errno={errno})"
);
define_per_command_error!(
    AcceptError,
    "failed to build accept request",
    "failed to send accept request",
    "failed to parse accept response",
    "bsd accept failed (errno={errno})"
);
define_per_command_error!(
    GetSockNameError,
    "failed to build getsockname request",
    "failed to send getsockname request",
    "failed to parse getsockname response",
    "bsd getsockname failed (errno={errno})"
);
define_per_command_error!(
    GetPeerNameError,
    "failed to build getpeername request",
    "failed to send getpeername request",
    "failed to parse getpeername response",
    "bsd getpeername failed (errno={errno})"
);
define_per_command_error!(
    ShutdownError,
    "failed to build shutdown request",
    "failed to send shutdown request",
    "failed to parse shutdown response",
    "bsd shutdown failed (errno={errno})"
);
define_per_command_error!(
    RecvError,
    "failed to build recv request",
    "failed to send recv request",
    "failed to parse recv response",
    "bsd recv failed (errno={errno})"
);
define_per_command_error!(
    RecvFromError,
    "failed to build recvfrom request",
    "failed to send recvfrom request",
    "failed to parse recvfrom response",
    "bsd recvfrom failed (errno={errno})"
);
define_per_command_error!(
    SendError,
    "failed to build send request",
    "failed to send send request",
    "failed to parse send response",
    "bsd send failed (errno={errno})"
);
define_per_command_error!(
    SendToError,
    "failed to build sendto request",
    "failed to send sendto request",
    "failed to parse sendto response",
    "bsd sendto failed (errno={errno})"
);
define_per_command_error!(
    ReadError,
    "failed to build read request",
    "failed to send read request",
    "failed to parse read response",
    "bsd read failed (errno={errno})"
);
define_per_command_error!(
    WriteError,
    "failed to build write request",
    "failed to send write request",
    "failed to parse write response",
    "bsd write failed (errno={errno})"
);
define_per_command_error!(
    GetSockOptError,
    "failed to build getsockopt request",
    "failed to send getsockopt request",
    "failed to parse getsockopt response",
    "bsd getsockopt failed (errno={errno})"
);
define_per_command_error!(
    SetSockOptError,
    "failed to build setsockopt request",
    "failed to send setsockopt request",
    "failed to parse setsockopt response",
    "bsd setsockopt failed (errno={errno})"
);
define_per_command_error!(
    FcntlError,
    "failed to build fcntl request",
    "failed to send fcntl request",
    "failed to parse fcntl response",
    "bsd fcntl failed (errno={errno})"
);
define_per_command_error!(
    IoctlError,
    "failed to build ioctl request",
    "failed to send ioctl request",
    "failed to parse ioctl response",
    "bsd ioctl failed (errno={errno})"
);
define_per_command_error!(
    SelectError,
    "failed to build select request",
    "failed to send select request",
    "failed to parse select response",
    "bsd select failed (errno={errno})"
);
define_per_command_error!(
    PollError,
    "failed to build poll request",
    "failed to send poll request",
    "failed to parse poll response",
    "bsd poll failed (errno={errno})"
);
