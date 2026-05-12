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
    cmif::{self, ParseResponseError},
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::REGISTER_CLIENT)
        .data_size(size_of::<RegisterClientIn>())
        .handles(1)
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to the valid TLS IPC buffer; data_size accounts
    // for the entire RegisterClientIn payload.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

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
    // SAFETY: req.data has data_size bytes reserved for RegisterClientIn.
    unsafe {
        ptr::write_unaligned(req.data.as_mut_ptr().cast::<RegisterClientIn>(), payload);
    }
    req.add_handle(tmem_handle.to_raw());

    ipc::send_sync_request(session).map_err(RegisterClientError::SendRequest)?;

    // SAFETY: Response is in the TLS buffer after a successful send. The reply
    // is a CMIF `u64 pid` payload — no extra `{ ret; errno; }` prefix here
    // because cmd 0 / cmd 1 don't use the dispatch path that adds it.
    let resp = unsafe { cmif::parse_response(ipc_buf, false, size_of::<u64>()) }
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();

    let fmt = cmif::RequestFormatBuilder::new(cmds::START_MONITORING)
        .data_size(size_of::<u64>())
        .send_pid()
        .build();

    // SAFETY: ipc_buf points to the valid TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };
    // SAFETY: req.data has 8 bytes reserved for the pid payload.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<u64>(), pid) };

    ipc::send_sync_request(monitor_session).map_err(StartMonitoringError::SendRequest)?;

    // SAFETY: Response in the TLS buffer; no response payload.
    let _resp = unsafe { cmif::parse_response(ipc_buf, false, 0) }
        .map_err(StartMonitoringError::ParseResponse)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared response decoding for `_bsdDispatchImpl`-style commands
// ---------------------------------------------------------------------------

/// Outcome of decoding the `{ ret; errno; [extra...] }` reply.
struct ServiceOutcome<'a> {
    ret: i32,
    extra: &'a [u8],
}

/// Reads the standard BSD response prefix from the TLS buffer.
///
/// On `ret >= 0`, returns the value and a slice over the trailing
/// command-specific payload. On `ret < 0`, returns `Err((ret, errno))` so
/// callers can map it into the right per-command `Service` variant.
///
/// # Safety
///
/// `ipc_buf` must be the live TLS IPC buffer right after a successful
/// `send_sync_request`. `extra_out_size` must match what the command actually
/// emitted in addition to the 8-byte prefix.
unsafe fn read_service_response<'a>(
    ipc_buf: core::ptr::NonNull<u8>,
    extra_out_size: usize,
) -> Result<ServiceOutcome<'a>, ServiceResponseFailure> {
    // SAFETY: caller upholds preconditions.
    let resp =
        unsafe { cmif::parse_response(ipc_buf, false, size_of::<CallResponse>() + extra_out_size) }
            .map_err(ServiceResponseFailure::Parse)?;

    // SAFETY: parse_response guaranteed at least size_of::<CallResponse>()
    // bytes are valid in resp.data.
    let prefix = unsafe { ptr::read_unaligned(resp.data.as_ptr().cast::<CallResponse>()) };

    if prefix.ret < 0 {
        return Err(ServiceResponseFailure::Service {
            ret: prefix.ret,
            errno: prefix.errno,
        });
    }

    let extra = &resp.data[size_of::<CallResponse>()..];
    Ok(ServiceOutcome {
        ret: prefix.ret,
        extra,
    })
}

enum ServiceResponseFailure {
    Parse(ParseResponseError),
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::SOCKET)
        .data_size(size_of::<SocketIn>())
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer; data_size matches SocketIn.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };
    let payload = SocketIn {
        domain,
        type_,
        protocol,
    };
    // SAFETY: req.data has data_size bytes reserved for SocketIn.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SocketIn>(), payload) };

    ipc::send_sync_request(session).map_err(SocketError::SendRequest)?;
    // SAFETY: response in TLS buffer; no command-specific extra payload.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SocketError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SocketError::Service { ret, errno },
    })?;
    Ok(BsdSockFd::from_raw(outcome.ret))
}

/// `bsdClose` (cmd 26).
pub(crate) fn close(session: SessionHandle, fd: BsdSockFd) -> Result<(), CloseError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::CLOSE)
        .data_size(size_of::<i32>())
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };
    // SAFETY: req.data has 4 bytes reserved for the fd.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), fd.raw()) };

    ipc::send_sync_request(session).map_err(CloseError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let _ = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
        ConnectError::SendRequest,
        ConnectError::ParseResponse,
        |ret, errno| ConnectError::Service { ret, errno },
    )
    .map(|_| ())
}

/// Shared `bind`/`connect` body — sends `sockfd` plus an in-only sockaddr
/// buffer, then drops the standard `{ ret; errno }` reply on the floor.
fn bsd_send_recv_no_buffer_in<E>(
    session: SessionHandle,
    cmd_id: u32,
    sockfd: BsdSockFd,
    addr: &[u8],
    mk_send: fn(SendSyncError) -> E,
    mk_parse: fn(ParseResponseError) -> E,
    mk_service: fn(i32, i32) -> E,
) -> Result<i32, E> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<i32>())
        .in_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
    // SAFETY: req.data has 4 bytes reserved for sockfd.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), sockfd.raw()) };
    req.add_in_auto_buffer(addr.as_ptr(), addr.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(mk_send)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => mk_parse(err),
        ServiceResponseFailure::Service { ret, errno } => mk_service(ret, errno),
    })?;
    Ok(outcome.ret)
}

/// Shared `accept`/`getsockname`/`getpeername` body — sends `sockfd` and an
/// out-only sockaddr buffer, then reads back the actual `socklen_t` length.
fn bsd_cmd_in_sockfd_out_sockaddr<E>(
    session: SessionHandle,
    cmd_id: u32,
    sockfd: BsdSockFd,
    addr_buf: &mut [u8],
    mk_send: fn(SendSyncError) -> E,
    mk_parse: fn(ParseResponseError) -> E,
    mk_service: fn(i32, i32) -> E,
) -> Result<(i32, u32), E> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmd_id)
        .data_size(size_of::<i32>())
        .out_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
    // SAFETY: req.data has 4 bytes reserved for sockfd.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), sockfd.raw()) };
    req.add_out_auto_buffer(addr_buf.as_mut_ptr(), addr_buf.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(mk_send)?;
    // SAFETY: response in TLS buffer; out_data carries one u32 with the addrlen.
    let outcome =
        unsafe { read_service_response(ipc_buf, size_of::<u32>()) }.map_err(|err| match err {
            ServiceResponseFailure::Parse(err) => mk_parse(err),
            ServiceResponseFailure::Service { ret, errno } => mk_service(ret, errno),
        })?;

    // SAFETY: read_service_response ensures `extra` is at least size_of::<u32>().
    let addrlen = unsafe { ptr::read_unaligned(outcome.extra.as_ptr().cast::<u32>()) };
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::LISTEN)
        .data_size(size_of::<ListenIn>())
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };
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

    ipc::send_sync_request(session).map_err(ListenError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let _ = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::SHUTDOWN)
        .data_size(size_of::<ShutdownIn>())
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };
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

    ipc::send_sync_request(session).map_err(ShutdownError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let _ = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::RECV)
        .data_size(size_of::<SockfdFlagsIn>())
        .out_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    req.add_out_auto_buffer(buf.as_mut_ptr(), buf.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(RecvError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::RECV_FROM)
        .data_size(size_of::<SockfdFlagsIn>())
        .out_auto_buffers(2)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    req.add_out_auto_buffer(buf.as_mut_ptr(), buf.len(), BufferMode::Normal);
    req.add_out_auto_buffer(src_addr.as_mut_ptr(), src_addr.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(RecvFromError::SendRequest)?;
    // SAFETY: response in TLS buffer; trailing payload is one u32 addrlen.
    let outcome =
        unsafe { read_service_response(ipc_buf, size_of::<u32>()) }.map_err(|err| match err {
            ServiceResponseFailure::Parse(err) => RecvFromError::ParseResponse(err),
            ServiceResponseFailure::Service { ret, errno } => RecvFromError::Service { ret, errno },
        })?;
    // SAFETY: outcome.extra is guaranteed >= size_of::<u32>().
    let addrlen = unsafe { ptr::read_unaligned(outcome.extra.as_ptr().cast::<u32>()) };
    Ok((outcome.ret as usize, addrlen))
}

/// `bsdSend` (cmd 10).
pub(crate) fn send(
    session: SessionHandle,
    sockfd: BsdSockFd,
    buf: &[u8],
    flags: i32,
) -> Result<usize, SendError> {
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::SEND)
        .data_size(size_of::<SockfdFlagsIn>())
        .in_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    req.add_in_auto_buffer(buf.as_ptr(), buf.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(SendError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::SEND_TO)
        .data_size(size_of::<SockfdFlagsIn>())
        .in_auto_buffers(2)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    req.add_in_auto_buffer(buf.as_ptr(), buf.len(), BufferMode::Normal);
    req.add_in_auto_buffer(dest_addr.as_ptr(), dest_addr.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(SendToError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::READ)
        .data_size(size_of::<i32>())
        .out_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
    // SAFETY: req.data has 4 bytes reserved for the fd.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), fd.raw()) };
    req.add_out_auto_buffer(buf.as_mut_ptr(), buf.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(ReadError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::WRITE)
        .data_size(size_of::<i32>())
        .in_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
    // SAFETY: req.data has 4 bytes reserved for the fd.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<i32>(), fd.raw()) };
    req.add_in_auto_buffer(buf.as_ptr(), buf.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(WriteError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::GET_SOCK_OPT)
        .data_size(size_of::<SockOptIn>())
        .out_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    req.add_out_auto_buffer(optval.as_mut_ptr(), optval.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(GetSockOptError::SendRequest)?;
    // SAFETY: response in TLS buffer; one u32 socklen_t trails the prefix.
    let outcome =
        unsafe { read_service_response(ipc_buf, size_of::<u32>()) }.map_err(|err| match err {
            ServiceResponseFailure::Parse(err) => GetSockOptError::ParseResponse(err),
            ServiceResponseFailure::Service { ret, errno } => {
                GetSockOptError::Service { ret, errno }
            }
        })?;
    // SAFETY: outcome.extra is guaranteed >= size_of::<u32>().
    let optlen = unsafe { ptr::read_unaligned(outcome.extra.as_ptr().cast::<u32>()) };
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::SET_SOCK_OPT)
        .data_size(size_of::<SockOptIn>())
        .in_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    req.add_in_auto_buffer(optval.as_ptr(), optval.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(SetSockOptError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let _ = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::FCNTL)
        .data_size(size_of::<FcntlIn>())
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let req = unsafe { cmif::make_request(ipc_buf, fmt) };
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

    ipc::send_sync_request(session).map_err(FcntlError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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

    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::IOCTL)
        .data_size(size_of::<IoctlIn>())
        .in_auto_buffers(if has_in { 1 } else { 0 })
        .out_auto_buffers(if has_out { 1 } else { 0 })
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    if has_in {
        req.add_in_auto_buffer(data.as_ptr(), payload_len, BufferMode::Normal);
    }
    if has_out {
        req.add_out_auto_buffer(data.as_mut_ptr(), payload_len, BufferMode::Normal);
    }

    ipc::send_sync_request(session).map_err(IoctlError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::SELECT)
        .data_size(size_of::<SelectIn>())
        .in_auto_buffers(3)
        .out_auto_buffers(3)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };

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
    // SAFETY: req.data has data_size bytes reserved for SelectIn.
    unsafe { ptr::write_unaligned(req.data.as_mut_ptr().cast::<SelectIn>(), payload) };

    req.add_in_auto_buffer(readfds.as_ptr(), readfds.len(), BufferMode::Normal);
    req.add_in_auto_buffer(writefds.as_ptr(), writefds.len(), BufferMode::Normal);
    req.add_in_auto_buffer(exceptfds.as_ptr(), exceptfds.len(), BufferMode::Normal);
    req.add_out_auto_buffer(readfds.as_mut_ptr(), readfds.len(), BufferMode::Normal);
    req.add_out_auto_buffer(writefds.as_mut_ptr(), writefds.len(), BufferMode::Normal);
    req.add_out_auto_buffer(exceptfds.as_mut_ptr(), exceptfds.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(SelectError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
    let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
    let fmt = cmif::RequestFormatBuilder::new(cmds::POLL)
        .data_size(size_of::<PollIn>())
        .in_auto_buffers(1)
        .out_auto_buffers(1)
        .build();
    // SAFETY: ipc_buf is the live TLS IPC buffer.
    let mut req = unsafe { cmif::make_request(ipc_buf, fmt) };
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
    req.add_in_auto_buffer(fds.as_ptr(), fds.len(), BufferMode::Normal);
    req.add_out_auto_buffer(fds.as_mut_ptr(), fds.len(), BufferMode::Normal);

    ipc::send_sync_request(session).map_err(PollError::SendRequest)?;
    // SAFETY: response in TLS buffer.
    let outcome = unsafe { read_service_response(ipc_buf, 0) }.map_err(|err| match err {
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
// carries three distinct sources: send failure, parse failure, and the
// service-level POSIX errno.

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to send the `RegisterClient` IPC request.
    #[error("failed to send register_client request")]
    SendRequest(#[source] SendSyncError),
    /// The CMIF response could not be parsed.
    #[error("failed to parse register_client response")]
    ParseResponse(#[source] ParseResponseError),
}

/// Error returned by [`start_monitoring`].
#[derive(Debug, thiserror::Error)]
pub enum StartMonitoringError {
    /// Failed to send the `StartMonitoring` IPC request.
    #[error("failed to send start_monitoring request")]
    SendRequest(#[source] SendSyncError),
    /// The CMIF response could not be parsed.
    #[error("failed to parse start_monitoring response")]
    ParseResponse(#[source] ParseResponseError),
}

macro_rules! define_per_command_error {
    ($name:ident, $send_msg:literal, $parse_msg:literal, $svc_msg:literal) => {
        #[doc = concat!("Error returned by the corresponding BSD command.")]
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            /// Failed to send the IPC request.
            #[error($send_msg)]
            SendRequest(#[source] SendSyncError),
            /// Failed to parse the CMIF response.
            #[error($parse_msg)]
            ParseResponse(#[source] ParseResponseError),
            /// The service returned a POSIX-domain failure. `errno` is the libc
            /// errno; `ret` is the raw value returned (typically `-1`).
            #[error($svc_msg)]
            Service { ret: i32, errno: i32 },
        }
    };
}

define_per_command_error!(
    SocketError,
    "failed to send socket request",
    "failed to parse socket response",
    "bsd socket failed (errno={errno})"
);
define_per_command_error!(
    CloseError,
    "failed to send close request",
    "failed to parse close response",
    "bsd close failed (errno={errno})"
);
define_per_command_error!(
    BindError,
    "failed to send bind request",
    "failed to parse bind response",
    "bsd bind failed (errno={errno})"
);
define_per_command_error!(
    ConnectError,
    "failed to send connect request",
    "failed to parse connect response",
    "bsd connect failed (errno={errno})"
);
define_per_command_error!(
    ListenError,
    "failed to send listen request",
    "failed to parse listen response",
    "bsd listen failed (errno={errno})"
);
define_per_command_error!(
    AcceptError,
    "failed to send accept request",
    "failed to parse accept response",
    "bsd accept failed (errno={errno})"
);
define_per_command_error!(
    GetSockNameError,
    "failed to send getsockname request",
    "failed to parse getsockname response",
    "bsd getsockname failed (errno={errno})"
);
define_per_command_error!(
    GetPeerNameError,
    "failed to send getpeername request",
    "failed to parse getpeername response",
    "bsd getpeername failed (errno={errno})"
);
define_per_command_error!(
    ShutdownError,
    "failed to send shutdown request",
    "failed to parse shutdown response",
    "bsd shutdown failed (errno={errno})"
);
define_per_command_error!(
    RecvError,
    "failed to send recv request",
    "failed to parse recv response",
    "bsd recv failed (errno={errno})"
);
define_per_command_error!(
    RecvFromError,
    "failed to send recvfrom request",
    "failed to parse recvfrom response",
    "bsd recvfrom failed (errno={errno})"
);
define_per_command_error!(
    SendError,
    "failed to send send request",
    "failed to parse send response",
    "bsd send failed (errno={errno})"
);
define_per_command_error!(
    SendToError,
    "failed to send sendto request",
    "failed to parse sendto response",
    "bsd sendto failed (errno={errno})"
);
define_per_command_error!(
    ReadError,
    "failed to send read request",
    "failed to parse read response",
    "bsd read failed (errno={errno})"
);
define_per_command_error!(
    WriteError,
    "failed to send write request",
    "failed to parse write response",
    "bsd write failed (errno={errno})"
);
define_per_command_error!(
    GetSockOptError,
    "failed to send getsockopt request",
    "failed to parse getsockopt response",
    "bsd getsockopt failed (errno={errno})"
);
define_per_command_error!(
    SetSockOptError,
    "failed to send setsockopt request",
    "failed to parse setsockopt response",
    "bsd setsockopt failed (errno={errno})"
);
define_per_command_error!(
    FcntlError,
    "failed to send fcntl request",
    "failed to parse fcntl response",
    "bsd fcntl failed (errno={errno})"
);
define_per_command_error!(
    IoctlError,
    "failed to send ioctl request",
    "failed to parse ioctl response",
    "bsd ioctl failed (errno={errno})"
);
define_per_command_error!(
    SelectError,
    "failed to send select request",
    "failed to parse select response",
    "bsd select failed (errno={errno})"
);
define_per_command_error!(
    PollError,
    "failed to send poll request",
    "failed to parse poll response",
    "bsd poll failed (errno={errno})"
);
