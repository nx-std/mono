//! CMIF protocol operations for the BSD socket service.
//!
//! Every command maps one-to-one to a `_bsd*` entry point in libnx's
//! `bsd.c`. Each function is `pub(crate)` so [`crate::BsdService`] can compose
//! them after acquiring a session from the pool.
//!
//! Non-init commands all share the same response prefix `{ int ret; int errno }`
//! (see libnx's `_bsdDispatchImpl`). When the service returns `ret < 0`, the
//! caller is informed via the corresponding `Service { ret, errno }` variant in
//! the per-command error enum - no thread-local `errno` is maintained.

use nx_sf::{
    cmif::{
        self,
        ParseError,
    },
    hipc::{
        BufferMode,
        InOutBuffer,
        InputBuffer,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};
use nx_svc::mem::tmem::Handle as TmemHandle;
use nx_sys_thread_tls::IpcBuffer;

use crate::{
    fd::BsdSockFd,
    proto::{
        BsdServiceConfigWire,
        CallResponse,
        CallResponseExtraU32,
        FcntlIn,
        IoctlIn,
        ListenIn,
        PollIn,
        RegisterClientIn,
        SelectIn,
        SelectTimeval,
        ShutdownIn,
        SockOptIn,
        SocketIn,
        SockfdFlagsIn,
        Timeval,
        cmds,
    },
    types::BsdConfig,
};

/// Sends `IBsdServices::RegisterClient` (cmd 0) on `session`.
///
/// Returns the client PID the service assigned, which must subsequently be
/// passed to [`start_monitoring`] on the monitor session.
pub(crate) fn register_client(
    session: BorrowedSessionHandle<'_>,
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

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmds::REGISTER_CLIENT)
        .with_data_value(&payload)
        .with_send_pid()
        .add_copy_handle(tmem_handle.to_raw())
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterClientError::SendRequest)?;

    // The reply is a CMIF `u64 pid` payload - no extra `{ ret; errno; }` prefix here
    // because cmd 0 / cmd 1 don't use the dispatch path that adds it.
    let resp = cmif::parse_response::<&u64>(&buf).map_err(RegisterClientError::ParseResponse)?;

    let pid = *resp.payload;
    Ok(pid)
}

/// Sends `IBsdServices::StartMonitoring` (cmd 1) on `monitor_session`.
pub(crate) fn start_monitoring(
    monitor_session: BorrowedSessionHandle<'_>,
    pid: u64,
) -> Result<(), StartMonitoringError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmds::START_MONITORING)
        .with_data_value(&pid)
        .with_send_pid()
        .build();
    req.send(&mut buf, monitor_session)
        .map_err(StartMonitoringError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(StartMonitoringError::ParseResponse)?;
    Ok(())
}

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
    buf: &IpcBuffer,
    has_extra_u32: bool,
) -> Result<ServiceOutcome, ServiceResponseFailure> {
    if has_extra_u32 {
        let resp = cmif::parse_response::<&CallResponseExtraU32>(buf)
            .map_err(ServiceResponseFailure::Parse)?;
        let payload = resp.payload;

        if payload.prefix.ret < 0 {
            return Err(ServiceResponseFailure::Service {
                ret: payload.prefix.ret,
                errno: payload.prefix.errno,
            });
        }

        Ok(ServiceOutcome {
            ret: payload.prefix.ret,
            extra_u32: Some(payload.extra),
        })
    } else {
        let resp =
            cmif::parse_response::<&CallResponse>(buf).map_err(ServiceResponseFailure::Parse)?;
        let prefix = resp.payload;

        if prefix.ret < 0 {
            return Err(ServiceResponseFailure::Service {
                ret: prefix.ret,
                errno: prefix.errno,
            });
        }

        Ok(ServiceOutcome {
            ret: prefix.ret,
            extra_u32: None,
        })
    }
}

enum ServiceResponseFailure {
    Parse(ParseError),
    Service { ret: i32, errno: i32 },
}

/// `bsdSocket` (cmd 2).
pub(crate) fn socket(
    session: BorrowedSessionHandle<'_>,
    domain: i32,
    type_: i32,
    protocol: i32,
) -> Result<BsdSockFd, SocketError> {
    let payload = SocketIn {
        domain,
        type_,
        protocol,
    };
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(cmds::SOCKET)
        .with_data_value(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(SocketError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SocketError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SocketError::Service { ret, errno },
    })?;
    Ok(BsdSockFd::from_raw(outcome.ret))
}

/// `bsdClose` (cmd 26).
pub(crate) fn close(session: BorrowedSessionHandle<'_>, fd: BsdSockFd) -> Result<(), CloseError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let fd_raw = fd.raw();
    let req = cmif::CmifRequestBuilder::new(cmds::CLOSE)
        .with_data_value(&fd_raw)
        .build();
    req.send(&mut buf, session)
        .map_err(CloseError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    unsafe { read_service_response(&buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => CloseError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => CloseError::Service { ret, errno },
    })?;
    Ok(())
}

/// `bsdBind` (cmd 13).
pub(crate) fn bind(
    session: BorrowedSessionHandle<'_>,
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
    session: BorrowedSessionHandle<'_>,
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

/// Shared `bind`/`connect` body - sends `sockfd` plus an in-only sockaddr
/// buffer, then drops the standard `{ ret; errno }` reply on the floor.
#[allow(clippy::too_many_arguments)]
fn bsd_send_recv_no_buffer_in<E>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    sockfd: BsdSockFd,
    addr: &[u8],
    mk_send: fn(cmif::SendError) -> E,
    mk_parse: fn(ParseError) -> E,
    mk_service: fn(i32, i32) -> E,
) -> Result<i32, E> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let sockfd_raw = sockfd.raw();
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&sockfd_raw)
        .add_in_auto_buffer(InputBuffer::new(addr, BufferMode::Normal))
        .build();
    req.send(&mut buf, session).map_err(mk_send)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => mk_parse(err),
        ServiceResponseFailure::Service { ret, errno } => mk_service(ret, errno),
    })?;
    Ok(outcome.ret)
}

/// Shared `accept`/`getsockname`/`getpeername` body - sends `sockfd` and an
/// out-only sockaddr buffer, then reads back the actual `socklen_t` length.
#[allow(clippy::too_many_arguments)]
fn bsd_cmd_in_sockfd_out_sockaddr<E>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    sockfd: BsdSockFd,
    addr_buf: &mut [u8],
    mk_send: fn(cmif::SendError) -> E,
    mk_parse: fn(ParseError) -> E,
    mk_service: fn(i32, i32) -> E,
) -> Result<(i32, u32), E> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let sockfd_raw = sockfd.raw();
    let req = cmif::CmifRequestBuilder::new(cmd_id)
        .with_data_value(&sockfd_raw)
        .add_out_auto_buffer(OutputBuffer::new(addr_buf, BufferMode::Normal))
        .build();
    req.send(&mut buf, session).map_err(mk_send)?;

    // SAFETY: the kernel populated the TLS IPC buffer; out_data carries one u32 with the addrlen.
    let outcome = unsafe { read_service_response(&buf, true) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => mk_parse(err),
        ServiceResponseFailure::Service { ret, errno } => mk_service(ret, errno),
    })?;

    let addrlen = outcome.extra_u32.unwrap_or(0);
    Ok((outcome.ret, addrlen))
}

/// `bsdAccept` (cmd 12). Returns the new socket fd and the actual `socklen_t`
/// length written into `addr_buf`.
pub(crate) fn accept(
    session: BorrowedSessionHandle<'_>,
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
    session: BorrowedSessionHandle<'_>,
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
    session: BorrowedSessionHandle<'_>,
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

/// `bsdListen` (cmd 18).
pub(crate) fn listen(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    backlog: i32,
) -> Result<(), ListenError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let payload = ListenIn {
        sockfd: sockfd.raw(),
        backlog,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::LISTEN)
        .with_data_value(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(ListenError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    unsafe { read_service_response(&buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => ListenError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => ListenError::Service { ret, errno },
    })?;
    Ok(())
}

/// `bsdShutdown` (cmd 22).
pub(crate) fn shutdown(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    how: i32,
) -> Result<(), ShutdownError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let payload = ShutdownIn {
        sockfd: sockfd.raw(),
        how,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::SHUTDOWN)
        .with_data_value(&payload)
        .build();
    req.send(&mut buf, session)
        .map_err(ShutdownError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    unsafe { read_service_response(&buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => ShutdownError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => ShutdownError::Service { ret, errno },
    })?;
    Ok(())
}

/// `bsdRecv` (cmd 8).
pub(crate) fn recv(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    flags: i32,
) -> Result<usize, RecvError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.raw(),
        flags,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::RECV)
        .with_data_value(&payload)
        .add_out_auto_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(RecvError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => RecvError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => RecvError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdRecvFrom` (cmd 9). Returns `(bytes_received, actual_src_addr_len)`.
pub(crate) fn recv_from(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    flags: i32,
    src_addr: &mut [u8],
) -> Result<(usize, u32), RecvFromError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.raw(),
        flags,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::RECV_FROM)
        .with_data_value(&payload)
        .add_out_auto_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .add_out_auto_buffer(OutputBuffer::new(src_addr, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(RecvFromError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; trailing payload is one u32 addrlen.
    let outcome = unsafe { read_service_response(&ipc_buf, true) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => RecvFromError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => RecvFromError::Service { ret, errno },
    })?;
    let addrlen = outcome.extra_u32.unwrap_or(0);
    Ok((outcome.ret as usize, addrlen))
}

/// `bsdSend` (cmd 10).
pub(crate) fn send(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &[u8],
    flags: i32,
) -> Result<usize, SendError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.raw(),
        flags,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::SEND)
        .with_data_value(&payload)
        .add_in_auto_buffer(InputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(SendError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SendError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SendError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdSendTo` (cmd 11).
pub(crate) fn send_to(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &[u8],
    flags: i32,
    dest_addr: &[u8],
) -> Result<usize, SendToError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.raw(),
        flags,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::SEND_TO)
        .with_data_value(&payload)
        .add_in_auto_buffer(InputBuffer::new(buf, BufferMode::Normal))
        .add_in_auto_buffer(InputBuffer::new(dest_addr, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(SendToError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SendToError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SendToError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdRead` (cmd 25).
pub(crate) fn read(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    buf: &mut [u8],
) -> Result<usize, ReadError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let fd_raw = fd.raw();
    let req = cmif::CmifRequestBuilder::new(cmds::READ)
        .with_data_value(&fd_raw)
        .add_out_auto_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(ReadError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => ReadError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => ReadError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdWrite` (cmd 24).
pub(crate) fn write(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    buf: &[u8],
) -> Result<usize, WriteError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let fd_raw = fd.raw();
    let req = cmif::CmifRequestBuilder::new(cmds::WRITE)
        .with_data_value(&fd_raw)
        .add_in_auto_buffer(InputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(WriteError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => WriteError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => WriteError::Service { ret, errno },
    })?;
    Ok(outcome.ret as usize)
}

/// `bsdGetSockOpt` (cmd 17). Returns the actual `socklen_t` written.
pub(crate) fn get_sock_opt(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    level: i32,
    optname: i32,
    optval: &mut [u8],
) -> Result<u32, GetSockOptError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockOptIn {
        sockfd: sockfd.raw(),
        level,
        optname,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::GET_SOCK_OPT)
        .with_data_value(&payload)
        .add_out_auto_buffer(OutputBuffer::new(optval, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(GetSockOptError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; one u32 socklen_t trails the prefix.
    let outcome = unsafe { read_service_response(&ipc_buf, true) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => GetSockOptError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => GetSockOptError::Service { ret, errno },
    })?;
    let optlen = outcome.extra_u32.unwrap_or(0);
    Ok(optlen)
}

/// `bsdSetSockOpt` (cmd 21).
pub(crate) fn set_sock_opt(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    level: i32,
    optname: i32,
    optval: &[u8],
) -> Result<(), SetSockOptError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockOptIn {
        sockfd: sockfd.raw(),
        level,
        optname,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::SET_SOCK_OPT)
        .with_data_value(&payload)
        .add_in_auto_buffer(InputBuffer::new(optval, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(SetSockOptError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SetSockOptError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SetSockOptError::Service { ret, errno },
    })?;
    Ok(())
}

/// `bsdFcntl` (cmd 20). libnx exposes only `F_GETFL` / `F_SETFL`; emulation of
/// other commands (e.g. returning `EOPNOTSUPP` without a server round-trip)
/// is left to the higher layer that wraps this function.
pub(crate) fn fcntl(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    cmd: i32,
    flags: i32,
) -> Result<i32, FcntlError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = FcntlIn {
        fd: fd.raw(),
        cmd,
        flags,
    };
    let req = cmif::CmifRequestBuilder::new(cmds::FCNTL)
        .with_data_value(&payload)
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(FcntlError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => FcntlError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => FcntlError::Service { ret, errno },
    })?;
    Ok(outcome.ret)
}

/// `bsdIoctl` (cmd 19) - generic case only.
///
/// `request` encodes both the direction (via `IOC_IN` / `IOC_OUT` bits) and the
/// payload size (via `IOCPARM_LEN`). This port handles the **generic** path of
/// libnx's switch table - the `SIOCGIFCONF` / `SIOCGIFMEDIA` / `SIOCGIFXMEDIA`
/// special cases (which reach into the caller's buffer to find sub-buffers)
/// are not implemented yet.
pub(crate) fn ioctl(
    session: BorrowedSessionHandle<'_>,
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

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = IoctlIn {
        fd: fd.raw(),
        request,
        bufcount: if has_inout { 1 } else { 0 },
    };
    let builder = cmif::CmifRequestBuilder::new(cmds::IOCTL).with_data_value(&payload);
    // When both directions are requested, `data` is attached once through
    // `add_inout_auto_buffer` - a single descriptor the kernel both reads
    // and writes - matching libnx's `bsdIoctl` wire shape without aliasing
    // two descriptors over the same memory.
    let builder = match (has_in, has_out) {
        (true, true) => builder.add_inout_auto_buffer(InOutBuffer::new(
            &mut data[..payload_len],
            BufferMode::Normal,
        )),
        (true, false) => {
            builder.add_in_auto_buffer(InputBuffer::new(&data[..payload_len], BufferMode::Normal))
        }
        (false, true) => builder.add_out_auto_buffer(OutputBuffer::new(
            &mut data[..payload_len],
            BufferMode::Normal,
        )),
        (false, false) => builder,
    };
    let req = builder.build();
    req.send(&mut ipc_buf, session)
        .map_err(IoctlError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => IoctlError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => IoctlError::Service { ret, errno },
    })?;
    Ok(outcome.ret)
}

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
    session: BorrowedSessionHandle<'_>,
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

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    // Each fd_set is both read and written by the kernel - libnx's
    // `bsdSelect` wire shape - so each is attached once through
    // `add_inout_auto_buffer` instead of aliasing an in-auto-buffer and an
    // out-auto-buffer over the same memory. This emits the three fd_set
    // descriptors interleaved (in, out, in, out, in, out) rather than
    // grouped (in, in, in, out, out, out), but the wire bytes are identical
    // either way: this crate never attaches pointer-buffers, so descriptor
    // order carries no addressing information the server depends on.
    let req = cmif::CmifRequestBuilder::new(cmds::SELECT)
        .with_data_value(&payload)
        .add_inout_auto_buffer(InOutBuffer::new(readfds, BufferMode::Normal))
        .add_inout_auto_buffer(InOutBuffer::new(writefds, BufferMode::Normal))
        .add_inout_auto_buffer(InOutBuffer::new(exceptfds, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(SelectError::SendRequest)?;

    // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
    let outcome = unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
        ServiceResponseFailure::Parse(err) => SelectError::ParseResponse(err),
        ServiceResponseFailure::Service { ret, errno } => SelectError::Service { ret, errno },
    })?;
    Ok(outcome.ret)
}

/// `bsdPoll` (cmd 6). `fds` must have layout matching libnx's `pollfd` array;
/// it is read as input and written back as output.
pub(crate) fn poll(
    session: BorrowedSessionHandle<'_>,
    fds: &mut [u8],
    nfds: u64,
    timeout: i32,
) -> Result<i32, PollError> {
    {
        let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

        let payload = PollIn {
            nfds,
            timeout,
            _pad: 0,
        };
        // `fds` is both read and written by the kernel - libnx's `bsdPoll`
        // wire shape - so it is attached once through
        // `add_inout_auto_buffer` instead of aliasing an in-auto-buffer and
        // an out-auto-buffer over the same memory.
        let req = cmif::CmifRequestBuilder::new(cmds::POLL)
            .with_data_value(&payload)
            .add_inout_auto_buffer(InOutBuffer::new(fds, BufferMode::Normal))
            .build();
        req.send(&mut ipc_buf, session)
            .map_err(PollError::SendRequest)?;
        // SAFETY: the kernel populated the TLS IPC buffer; no other borrow is live.
        let outcome =
            unsafe { read_service_response(&ipc_buf, false) }.map_err(|err| match err {
                ServiceResponseFailure::Parse(err) => PollError::ParseResponse(err),
                ServiceResponseFailure::Service { ret, errno } => PollError::Service { ret, errno },
            })?;
        Ok(outcome.ret)
    }
}

/// Error returned by [`register_client`].
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// Failed to send the `RegisterClient` IPC request.
    #[error("failed to send register_client request")]
    SendRequest(#[source] cmif::SendError),
    /// The CMIF response could not be parsed.
    #[error("failed to parse register_client response")]
    ParseResponse(#[source] ParseError),
}

/// Error returned by [`start_monitoring`].
#[derive(Debug, thiserror::Error)]
pub enum StartMonitoringError {
    /// Failed to send the `StartMonitoring` IPC request.
    #[error("failed to send start_monitoring request")]
    SendRequest(#[source] cmif::SendError),
    /// The CMIF response could not be parsed.
    #[error("failed to parse start_monitoring response")]
    ParseResponse(#[source] ParseError),
}

macro_rules! define_per_command_error {
    ($name:ident, $send_msg:literal, $parse_msg:literal, $svc_msg:literal) => {
        #[doc = concat!("Error returned by the corresponding BSD command.")]
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            /// Failed to send the IPC request.
            #[error($send_msg)]
            SendRequest(#[source] cmif::SendError),
            /// Failed to parse the CMIF response.
            #[error($parse_msg)]
            ParseResponse(#[source] ParseError),
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
