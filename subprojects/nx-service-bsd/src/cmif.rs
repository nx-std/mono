//! CMIF protocol operations for the BSD socket service.
//!
//! Every command maps one-to-one to a `_bsd*` entry point in libnx's
//! `bsd.c`. Each function is `pub(crate)` so [`crate::BsdService`] can compose
//! them after acquiring a session from the pool.
//!
//! Every command past the two handshake commands shares one response prefix,
//! `{ int ret; int error_code }` (libnx's `_bsdDispatchImpl`), so they share
//! one error type: [`CommandError`] names the step that failed and the command
//! it failed for. A rejected command arrives as [`CommandError::Service`],
//! carrying the condition the service reported as a [`PosixError`] rather than
//! the raw wire number — see [`crate::posix`] for why that distinction is
//! load-bearing.

use core::ffi::CStr;

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
use zerocopy::IntoBytes as _;

use crate::{
    config::BsdConfig,
    fd::BsdSockFd,
    posix::PosixError,
    proto::{
        BsdServiceConfigWire,
        CallResponse,
        CallResponseExtraU32,
        CallResponseExtraU64,
        Command,
        DuplicateSocketIn,
        FcntlIn,
        IoctlIn,
        ListenIn,
        PollIn,
        RecvMMsgIn,
        RegisterClientIn,
        SelectIn,
        SelectTimeval,
        SendMMsgIn,
        ShutdownIn,
        SockOptIn,
        SocketIn,
        SockfdFlagsIn,
        Timespec,
        Timeval,
    },
    sockaddr::RawSockAddr,
    transfer::{
        RecvFlags,
        SendFlags,
        Shutdown,
        StatusFlags,
    },
};

/// Sends `IBsdServices::RegisterClient` on `session`.
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
            version: config.version.to_wire(),
            tcp_tx_buf_size: config.tcp_tx_buf_size,
            tcp_rx_buf_size: config.tcp_rx_buf_size,
            tcp_tx_buf_max_size: config.tcp_tx_buf_max_size,
            tcp_rx_buf_max_size: config.tcp_rx_buf_max_size,
            udp_tx_buf_size: config.udp_tx_buf_size,
            udp_rx_buf_size: config.udp_rx_buf_size,
            sb_efficiency: config.sb_efficiency.to_wire(),
        },
        pid_placeholder: 0,
        tmem_size,
    };

    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(Command::RegisterClient.id())
        .with_data_value(&payload)
        .with_send_pid()
        .add_copy_handle(tmem_handle.to_raw())
        .build();
    req.send(&mut buf, session)
        .map_err(RegisterClientError::SendRequest)?;

    // The reply is a CMIF `u64 pid` payload - no `{ ret; error_code }` prefix
    // here, because the two handshake commands do not travel the dispatch path
    // that adds it.
    let resp = cmif::parse_response::<&u64>(&buf).map_err(RegisterClientError::ParseResponse)?;

    Ok(*resp.payload)
}

/// Errors returned by the [`Command::RegisterClient`] handshake.
#[derive(Debug, thiserror::Error)]
pub enum RegisterClientError {
    /// The request never reached the service.
    ///
    /// Nothing was registered, so the transfer memory the caller created is
    /// still theirs to release.
    #[error("failed to send the RegisterClient request")]
    SendRequest(#[source] cmif::SendError),

    /// The service answered, but the reply did not decode.
    ///
    /// The client may have been registered regardless, so the caller tears
    /// the session down rather than retrying.
    #[error("failed to parse the RegisterClient response")]
    ParseResponse(#[source] ParseError),
}

/// Sends `IBsdServices::StartMonitoring` on `monitor_session`.
pub(crate) fn start_monitoring(
    monitor_session: BorrowedSessionHandle<'_>,
    pid: u64,
) -> Result<(), StartMonitoringError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(Command::StartMonitoring.id())
        .with_data_value(&pid)
        .with_send_pid()
        .build();
    req.send(&mut buf, monitor_session)
        .map_err(StartMonitoringError::SendRequest)?;

    cmif::parse_response::<()>(&buf).map_err(StartMonitoringError::ParseResponse)?;
    Ok(())
}

/// Errors returned by the [`Command::StartMonitoring`] handshake.
#[derive(Debug, thiserror::Error)]
pub enum StartMonitoringError {
    /// The request never reached the service on the monitor session.
    #[error("failed to send the StartMonitoring request")]
    SendRequest(#[source] cmif::SendError),

    /// The service answered, but the reply did not decode.
    #[error("failed to parse the StartMonitoring response")]
    ParseResponse(#[source] ParseError),
}

/// What the service returned for a command it accepted.
struct ServiceOutcome {
    /// The command's own return value: a descriptor, a byte count, a ready
    /// count. Never negative — a negative `ret` is what
    /// [`read_service_response`] turns into an error.
    ret: i32,
    /// The word some commands append after the response prefix: a `socklen_t`
    /// for `accept`, an `optlen` for `getsockopt`, the written length for
    /// `sysctl`. `None` when the command appends nothing.
    extra: Option<u64>,
}

impl ServiceOutcome {
    /// The return value read as a byte count.
    ///
    /// The `as` cast is exact rather than lossy: [`read_service_response`] has
    /// already rejected every negative `ret`, and a non-negative `i32` always
    /// fits the 64-bit `usize` of this target.
    fn byte_count(&self) -> usize {
        self.ret as usize
    }

    /// The address length the service reported, as a `socklen_t`.
    ///
    /// Zero when the command appended nothing. The commands that do append one
    /// write a `u32`, so the value always fits. This is the length the BSD
    /// interface reports for the *address*, not for the buffer it filled:
    /// [`RawSockAddr::from_response`] is what reconciles the two.
    fn reported_addr_len(&self) -> u32 {
        self.extra.unwrap_or(0) as u32
    }
}

/// What a command appends after the shared response prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtraWord {
    /// Nothing follows `{ ret; error_code }`.
    None,
    /// A `u32` follows — a `socklen_t` or an `optlen`.
    U32,
    /// A `u64` follows — the `size_t` `sysctl` reports.
    U64,
}

/// Reads the response prefix every dispatched command shares.
///
/// A negative `ret` is the service rejecting the command; the condition it
/// reported is classified into a [`PosixError`] here, at the wire boundary, so
/// it never travels further as a bare number.
fn read_service_response(
    buf: &IpcBuffer,
    command: Command,
    extra: ExtraWord,
) -> Result<ServiceOutcome, CommandError> {
    let (prefix, extra) = match extra {
        ExtraWord::None => {
            let resp = cmif::parse_response::<&CallResponse>(buf).map_err(|err| {
                CommandError::ParseResponse {
                    command,
                    source: err,
                }
            })?;
            (*resp.payload, None)
        }
        ExtraWord::U32 => {
            let resp = cmif::parse_response::<&CallResponseExtraU32>(buf).map_err(|err| {
                CommandError::ParseResponse {
                    command,
                    source: err,
                }
            })?;
            (resp.payload.prefix, Some(u64::from(resp.payload.extra)))
        }
        ExtraWord::U64 => {
            let resp = cmif::parse_response::<&CallResponseExtraU64>(buf).map_err(|err| {
                CommandError::ParseResponse {
                    command,
                    source: err,
                }
            })?;
            (resp.payload.prefix, Some(resp.payload.extra))
        }
    };

    if prefix.ret < 0 {
        return Err(CommandError::Service {
            command,
            source: PosixError::from(prefix.error_code),
        });
    }

    Ok(ServiceOutcome {
        ret: prefix.ret,
        extra,
    })
}

/// Errors returned by every dispatched `IBsdServices` command.
///
/// Shared rather than declared per command because the three ways a command
/// can fail are the same for all of them, and every command can produce all
/// three. The `command` field is what says which one was in flight, so
/// sharing costs no detail.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// The request never reached the service.
    ///
    /// Possible causes:
    /// - The session was closed by the server.
    /// - The request did not fit the calling thread's IPC buffer.
    ///
    /// Nothing was executed, so the command is safe to retry.
    #[error("failed to send the {command} request on the bsd session")]
    SendRequest {
        /// The command that was being sent.
        command: Command,
        /// Why the send failed.
        #[source]
        source: cmif::SendError,
    },

    /// The service answered, but the reply did not decode.
    ///
    /// The command may have taken effect, so whether to retry is the caller's
    /// judgement rather than automatically safe.
    #[error("failed to parse the {command} response")]
    ParseResponse {
        /// The command whose reply failed to decode.
        command: Command,
        /// Where decoding stopped.
        #[source]
        source: ParseError,
    },

    /// The service executed the command and rejected it.
    ///
    /// An ordinary socket failure — a refused connection, an exhausted
    /// buffer, a would-block on a non-blocking socket — rather than a
    /// transport fault. This is the variant callers act on.
    #[error("the bsd service rejected {command}")]
    Service {
        /// The command the service rejected.
        command: Command,
        /// The condition it reported.
        #[source]
        source: PosixError,
    },
}

/// Sends a command whose entire input is one value and whose reply is bare.
///
/// The shape libnx writes as `_bsdDispatchIn(cmd, in)` with no buffers.
fn send_value_only<T>(
    session: BorrowedSessionHandle<'_>,
    command: Command,
    payload: &T,
) -> Result<ServiceOutcome, CommandError>
where
    T: zerocopy::Immutable + zerocopy::IntoBytes,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(payload)
        .build();
    req.send(&mut buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    read_service_response(&buf, command, ExtraWord::None)
}

/// `bsdSocket`. Creates a socket and returns its descriptor.
pub(crate) fn socket(
    session: BorrowedSessionHandle<'_>,
    domain: i32,
    type_: i32,
    protocol: i32,
) -> Result<BsdSockFd, CommandError> {
    let payload = SocketIn {
        domain,
        type_,
        protocol,
    };
    let outcome = send_value_only(session, Command::Socket, &payload)?;
    // SAFETY: `read_service_response` returned `Ok`, so the service accepted
    // the command and `ret` is the descriptor it issued, not a rejection.
    Ok(BsdSockFd::from_raw_unchecked(outcome.ret))
}

/// `bsdSocketExempt`. Creates a socket exempt from the system's socket
/// accounting; identical to [`socket`] in every other respect.
pub(crate) fn socket_exempt(
    session: BorrowedSessionHandle<'_>,
    domain: i32,
    type_: i32,
    protocol: i32,
) -> Result<BsdSockFd, CommandError> {
    let payload = SocketIn {
        domain,
        type_,
        protocol,
    };
    let outcome = send_value_only(session, Command::SocketExempt, &payload)?;
    // SAFETY: `read_service_response` returned `Ok`, so the service accepted
    // the command and `ret` is the descriptor it issued, not a rejection.
    Ok(BsdSockFd::from_raw_unchecked(outcome.ret))
}

/// `bsdOpen`. Opens a path in the service's own namespace.
///
/// The path travels with its terminating NUL, which is what libnx's
/// `strlen(pathname) + 1` length amounts to.
pub(crate) fn open(
    session: BorrowedSessionHandle<'_>,
    path: &CStr,
    flags: i32,
) -> Result<BsdSockFd, CommandError> {
    let command = Command::Open;
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&flags)
        .add_in_auto_buffer(InputBuffer::new(
            path.to_bytes_with_nul(),
            BufferMode::Normal,
        ))
        .build();
    req.send(&mut buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&buf, command, ExtraWord::None)?;
    // SAFETY: `read_service_response` returned `Ok`, so the service accepted
    // the command and `ret` is the descriptor it issued, not a rejection.
    Ok(BsdSockFd::from_raw_unchecked(outcome.ret))
}

/// `bsdClose`. Releases a descriptor.
pub(crate) fn close(session: BorrowedSessionHandle<'_>, fd: BsdSockFd) -> Result<(), CommandError> {
    send_value_only(session, Command::Close, &fd.to_raw())?;
    Ok(())
}

/// `bsdDuplicateSocket`. Returns a second descriptor naming the same socket.
pub(crate) fn duplicate_socket(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
) -> Result<BsdSockFd, CommandError> {
    let payload = DuplicateSocketIn {
        sockfd: sockfd.to_raw(),
        _pad: 0,
        reserved: 0,
    };
    let outcome = send_value_only(session, Command::DuplicateSocket, &payload)?;
    // SAFETY: `read_service_response` returned `Ok`, so the service accepted
    // the command and `ret` is the descriptor it issued, not a rejection.
    Ok(BsdSockFd::from_raw_unchecked(outcome.ret))
}

/// Sends a command that takes a descriptor plus one in-only address buffer.
///
/// The shape shared by `bind` and `connect`.
fn send_sockfd_with_addr(
    session: BorrowedSessionHandle<'_>,
    command: Command,
    sockfd: BsdSockFd,
    addr: &RawSockAddr,
) -> Result<(), CommandError> {
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let sockfd_raw = sockfd.to_raw();
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&sockfd_raw)
        .add_in_auto_buffer(InputBuffer::new(addr.as_bytes(), BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    read_service_response(&buf, command, ExtraWord::None)?;
    Ok(())
}

/// `bsdBind`. Assigns a local address to a socket.
pub(crate) fn bind(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    addr: &RawSockAddr,
) -> Result<(), CommandError> {
    send_sockfd_with_addr(session, Command::Bind, sockfd, addr)
}

/// `bsdConnect`. Initiates a connection to a peer.
pub(crate) fn connect(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    addr: &RawSockAddr,
) -> Result<(), CommandError> {
    send_sockfd_with_addr(session, Command::Connect, sockfd, addr)
}

/// Sends a command that takes a descriptor and answers with a socket address.
///
/// The shape shared by `accept`, `getsockname` and `getpeername`. The address
/// buffer is this function's own, sized by [`RawSockAddr::CAPACITY`], so the
/// storage and the length the service reports for it are reconciled here and
/// never travel apart.
fn send_sockfd_for_addr(
    session: BorrowedSessionHandle<'_>,
    command: Command,
    sockfd: BsdSockFd,
) -> Result<(ServiceOutcome, RawSockAddr), CommandError> {
    let mut addr_buf = [0u8; RawSockAddr::CAPACITY];
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let sockfd_raw = sockfd.to_raw();
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&sockfd_raw)
        .add_out_auto_buffer(OutputBuffer::new(&mut addr_buf, BufferMode::Normal))
        .build();
    req.send(&mut buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&buf, command, ExtraWord::U32)?;
    let addr = RawSockAddr::from_response(&addr_buf, outcome.reported_addr_len());
    Ok((outcome, addr))
}

/// `bsdAccept`. Takes the next connection off a listening socket's queue,
/// returning its descriptor and the peer's address.
pub(crate) fn accept(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
) -> Result<(BsdSockFd, RawSockAddr), CommandError> {
    let (outcome, addr) = send_sockfd_for_addr(session, Command::Accept, sockfd)?;
    // SAFETY: `send_sockfd_for_addr` returned `Ok`, so the service accepted the
    // command and `ret` is the descriptor it issued for the new connection.
    Ok((BsdSockFd::from_raw_unchecked(outcome.ret), addr))
}

/// `bsdGetSockName`. Reports the socket's own address.
pub(crate) fn get_sock_name(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
) -> Result<RawSockAddr, CommandError> {
    let (_outcome, addr) = send_sockfd_for_addr(session, Command::GetSockName, sockfd)?;
    Ok(addr)
}

/// `bsdGetPeerName`. Reports the connected peer's address.
pub(crate) fn get_peer_name(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
) -> Result<RawSockAddr, CommandError> {
    let (_outcome, addr) = send_sockfd_for_addr(session, Command::GetPeerName, sockfd)?;
    Ok(addr)
}

/// `bsdListen`. Marks a socket as accepting connections.
pub(crate) fn listen(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    backlog: i32,
) -> Result<(), CommandError> {
    let payload = ListenIn {
        sockfd: sockfd.to_raw(),
        backlog,
    };
    send_value_only(session, Command::Listen, &payload)?;
    Ok(())
}

/// `bsdShutdown`. Disables further sends, receives, or both, on one socket.
pub(crate) fn shutdown(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    how: Shutdown,
) -> Result<(), CommandError> {
    let payload = ShutdownIn {
        sockfd: sockfd.to_raw(),
        how: how.to_wire(),
    };
    send_value_only(session, Command::Shutdown, &payload)?;
    Ok(())
}

/// `bsdShutdownAllSockets`. Applies `how` to every socket this client owns.
pub(crate) fn shutdown_all_sockets(
    session: BorrowedSessionHandle<'_>,
    how: Shutdown,
) -> Result<(), CommandError> {
    send_value_only(session, Command::ShutdownAllSockets, &how.to_wire())?;
    Ok(())
}

/// `bsdRecv`. Receives from a connected socket, returning the byte count.
pub(crate) fn recv(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    flags: RecvFlags,
) -> Result<usize, CommandError> {
    let command = Command::Recv;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.to_raw(),
        flags: flags.bits(),
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_out_auto_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.byte_count())
}

/// `bsdRecvFrom`. Receives and reports the sender's address.
///
/// The data buffer stays the caller's, because only the caller knows how much
/// it wants; the address buffer is this function's own, for the reasons
/// [`crate::sockaddr`] gives.
pub(crate) fn recv_from(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    flags: RecvFlags,
) -> Result<(usize, RawSockAddr), CommandError> {
    let command = Command::RecvFrom;
    let mut src_addr = [0u8; RawSockAddr::CAPACITY];
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.to_raw(),
        flags: flags.bits(),
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_out_auto_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .add_out_auto_buffer(OutputBuffer::new(&mut src_addr, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::U32)?;
    let addr = RawSockAddr::from_response(&src_addr, outcome.reported_addr_len());
    Ok((outcome.byte_count(), addr))
}

/// `bsdSend`. Sends on a connected socket, returning the accepted byte count.
pub(crate) fn send(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &[u8],
    flags: SendFlags,
) -> Result<usize, CommandError> {
    let command = Command::Send;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.to_raw(),
        flags: flags.bits(),
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_in_auto_buffer(InputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.byte_count())
}

/// `bsdSendTo`. Sends to an explicit address, returning the accepted byte
/// count.
pub(crate) fn send_to(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &[u8],
    flags: SendFlags,
    dest_addr: &RawSockAddr,
) -> Result<usize, CommandError> {
    let command = Command::SendTo;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockfdFlagsIn {
        sockfd: sockfd.to_raw(),
        flags: flags.bits(),
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_in_auto_buffer(InputBuffer::new(buf, BufferMode::Normal))
        .add_in_auto_buffer(InputBuffer::new(dest_addr.as_bytes(), BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.byte_count())
}

/// `bsdRead`. Reads from a descriptor, returning the byte count.
pub(crate) fn read(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    buf: &mut [u8],
) -> Result<usize, CommandError> {
    let command = Command::Read;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let fd_raw = fd.to_raw();
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&fd_raw)
        .add_out_auto_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.byte_count())
}

/// `bsdWrite`. Writes to a descriptor, returning the accepted byte count.
pub(crate) fn write(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    buf: &[u8],
) -> Result<usize, CommandError> {
    let command = Command::Write;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let fd_raw = fd.to_raw();
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&fd_raw)
        .add_in_auto_buffer(InputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.byte_count())
}

/// `bsdGetSockOpt`. Reads a socket option, returning its value.
///
/// The option's type is what says how many bytes it occupies, so `T` supplies
/// both the buffer and its length and there is nothing for a caller to size
/// wrongly. `std` reaches for `MaybeUninit` and an `unsafe fn` to say the same
/// thing; the `FromBytes` bound says it without either, since a type every
/// byte pattern is valid for can simply be zeroed and filled.
///
/// The trailing length the service reports is read to keep the response layout
/// right and then discarded, as `std` discards its `option_len`.
pub(crate) fn get_sock_opt<T>(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    level: i32,
    optname: i32,
) -> Result<T, CommandError>
where
    T: zerocopy::FromBytes + zerocopy::Immutable + zerocopy::IntoBytes,
{
    let command = Command::GetSockOpt;
    let mut value = T::new_zeroed();
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockOptIn {
        sockfd: sockfd.to_raw(),
        level,
        optname,
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_out_auto_buffer(OutputBuffer::new(value.as_mut_bytes(), BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    read_service_response(&ipc_buf, command, ExtraWord::U32)?;
    Ok(value)
}

/// `bsdSetSockOpt`. Writes a socket option.
///
/// Takes the option's own type, for the same reason [`get_sock_opt`] returns
/// one: the length is the type's, not the caller's to get right.
pub(crate) fn set_sock_opt<T>(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    level: i32,
    optname: i32,
    optval: &T,
) -> Result<(), CommandError>
where
    T: zerocopy::Immutable + zerocopy::IntoBytes,
{
    let command = Command::SetSockOpt;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SockOptIn {
        sockfd: sockfd.to_raw(),
        level,
        optname,
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_in_auto_buffer(InputBuffer::new(optval.as_bytes(), BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(())
}

/// A `fcntl` operation the BSD service implements.
///
/// The service answers only these two. libnx rejects every other `fcntl`
/// command with "operation not supported" before sending anything, and making
/// the pair a type moves that rejection out to the edge that still holds the
/// C command number — this crate never has to represent a command it cannot
/// send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FcntlOp {
    /// `F_GETFL` — read the descriptor's status flags.
    GetFlags,
    /// `F_SETFL` — replace the descriptor's status flags with these.
    SetFlags(StatusFlags),
}

impl FcntlOp {
    /// The `(cmd, flags)` pair this operation is sent as.
    ///
    /// A read sends zero flags, which is what libnx substitutes rather than
    /// forwarding whatever the caller happened to pass alongside one.
    const fn to_wire(self) -> (i32, i32) {
        /// `F_GETFL`, as newlib numbers it.
        const F_GETFL: i32 = 3;
        /// `F_SETFL`, as newlib numbers it.
        const F_SETFL: i32 = 4;

        match self {
            Self::GetFlags => (F_GETFL, 0),
            Self::SetFlags(flags) => (F_SETFL, flags.bits()),
        }
    }
}

/// `bsdFcntl`. Reads or replaces a descriptor's status flags.
///
/// Returns the descriptor's status flags for [`FcntlOp::GetFlags`], and an
/// empty set for [`FcntlOp::SetFlags`].
///
/// Bits the service reports that [`StatusFlags`] has no name for are kept
/// rather than dropped: they are the descriptor's own state, and a caller that
/// reads flags in order to write them back must not lose them on the way
/// through.
pub(crate) fn fcntl(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    op: FcntlOp,
) -> Result<StatusFlags, CommandError> {
    let (cmd, flags) = op.to_wire();
    let payload = FcntlIn {
        fd: fd.to_raw(),
        cmd,
        flags,
    };
    let outcome = send_value_only(session, Command::Fcntl, &payload)?;
    Ok(StatusFlags::from_bits_retain(outcome.ret))
}

/// Direction and payload length are packed into an `ioctl` request code,
/// libc-style.
mod ioc {
    /// The request carries a payload to the service.
    pub(super) const IN: i32 = 0x4000_0000_u32 as i32;

    /// The request expects a payload back from the service.
    pub(super) const OUT: i32 = 0x2000_0000_u32 as i32;

    /// Mask selecting the payload length packed into bits 16..=28.
    const PARM_MASK: i32 = 0x1FFF;

    /// The payload length the request code declares.
    pub(super) const fn parm_len(request: i32) -> i32 {
        (request >> 16) & PARM_MASK
    }
}

/// `bsdIoctl`, for every request whose payload is one flat block.
///
/// The direction bits and the payload length are read out of `request`: a
/// request with neither direction bit set carries no payload at all.
///
/// Requests that instead point at a second buffer — `SIOCGIFCONF`,
/// `SIOCGIFMEDIA`, `SIOCGIFXMEDIA` — go through [`ioctl_with_entries`],
/// because finding that buffer means reading a pointer out of the caller's
/// bytes, which belongs to the C-facing layer that owns those bytes rather
/// than here.
pub(crate) fn ioctl(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    request: i32,
    data: &mut [u8],
) -> Result<i32, CommandError> {
    let command = Command::Ioctl;

    let has_in = (request & ioc::IN) != 0;
    let has_out = (request & ioc::OUT) != 0;
    // Clamped to what the caller actually provided. The length in the request
    // code is the C struct's size, so a caller that passed a shorter slice
    // would otherwise have the service read or write past its end.
    let payload_len = if has_in || has_out {
        core::cmp::min(ioc::parm_len(request) as usize, data.len())
    } else {
        0
    };

    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = IoctlIn {
        fd: fd.to_raw(),
        request,
        // libnx counts one buffer for every request on this path, including
        // the ones carrying no payload, so this does not track the direction
        // bits.
        bufcount: 1,
    };
    let builder = cmif::CmifRequestBuilder::new(command.id()).with_data_value(&payload);
    // When both directions are requested, `data` is attached once through
    // `add_inout_auto_buffer` - a single descriptor the kernel both reads and
    // writes - matching libnx's wire shape without aliasing two descriptors
    // over the same memory.
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
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.ret)
}

/// `bsdIoctl`, for the requests whose payload is a header plus a list.
///
/// `SIOCGIFCONF` answers with a header and an array of interface records;
/// `SIOCGIFMEDIA` and `SIOCGIFXMEDIA` answer with a header and an array of
/// media words. In C the header holds a pointer to that array and libnx
/// follows it to find the second buffer; here the caller passes both, so no
/// pointer is ever read out of caller-supplied bytes.
///
/// Both buffers travel in each direction, as they do in libnx: the service
/// reads the header to learn how much room the list has, then writes both back.
pub(crate) fn ioctl_with_entries(
    session: BorrowedSessionHandle<'_>,
    fd: BsdSockFd,
    request: i32,
    header: &mut [u8],
    entries: &mut [u8],
) -> Result<i32, CommandError> {
    let command = Command::Ioctl;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = IoctlIn {
        fd: fd.to_raw(),
        request,
        bufcount: 2,
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_inout_auto_buffer(InOutBuffer::new(header, BufferMode::Normal))
        .add_inout_auto_buffer(InOutBuffer::new(entries, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.ret)
}

/// `bsdSysctl`. Reads or writes a kernel networking parameter.
///
/// `name` is the MIB naming the parameter, `new_value` is what to write (empty
/// for a plain read), and `old_value` receives the previous value. Returns the
/// length the service wrote into `old_value`.
pub(crate) fn sysctl(
    session: BorrowedSessionHandle<'_>,
    name: &[i32],
    new_value: &[u8],
    old_value: &mut [u8],
) -> Result<u64, CommandError> {
    let command = Command::Sysctl;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let req = cmif::CmifRequestBuilder::new(command.id())
        .add_in_auto_buffer(InputBuffer::new(name.as_bytes(), BufferMode::Normal))
        .add_in_auto_buffer(InputBuffer::new(new_value, BufferMode::Normal))
        .add_out_auto_buffer(OutputBuffer::new(old_value, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::U64)?;
    Ok(outcome.extra.unwrap_or(0))
}

/// Optional timeout for [`BsdService::select`](crate::BsdService::select)
/// (mirrors libnx's `BsdSelectTimeval`).
#[derive(Debug, Clone, Copy)]
pub struct SelectTimeout {
    /// Seconds component.
    pub sec: i64,
    /// Microseconds component.
    pub usec: i64,
}

/// `bsdSelect`. Waits for readiness across three descriptor sets.
///
/// Each `fd_set` buffer is opaque to this crate; callers are expected to use
/// libnx's `fd_set` byte layout. Pass empty slices for fd_sets that should be
/// transmitted as null, and `None` for `timeout` to send the libnx
/// `is_null = true` sentinel.
pub(crate) fn select(
    session: BorrowedSessionHandle<'_>,
    nfds: i32,
    readfds: &mut [u8],
    writefds: &mut [u8],
    exceptfds: &mut [u8],
    timeout: Option<SelectTimeout>,
) -> Result<i32, CommandError> {
    let command = Command::Select;
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

    // Each fd_set is both read and written by the kernel - libnx's wire shape
    // - so each is attached once through `add_inout_auto_buffer` instead of
    // aliasing an in-auto-buffer and an out-auto-buffer over the same memory.
    // This emits the three fd_set descriptors interleaved (in, out, in, out,
    // in, out) rather than grouped (in, in, in, out, out, out), but the wire
    // bytes are identical either way: this crate never attaches
    // pointer-buffers, so descriptor order carries no addressing information
    // the server depends on.
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_inout_auto_buffer(InOutBuffer::new(readfds, BufferMode::Normal))
        .add_inout_auto_buffer(InOutBuffer::new(writefds, BufferMode::Normal))
        .add_inout_auto_buffer(InOutBuffer::new(exceptfds, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.ret)
}

/// `bsdPoll`. Waits for readiness across a descriptor array.
///
/// `fds` must have the layout of libnx's `pollfd` array; it is read as input
/// and written back as output.
pub(crate) fn poll(
    session: BorrowedSessionHandle<'_>,
    fds: &mut [u8],
    nfds: u64,
    timeout: i32,
) -> Result<i32, CommandError> {
    let command = Command::Poll;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = PollIn {
        nfds,
        timeout,
        _pad: 0,
    };
    // `fds` is both read and written by the kernel - libnx's wire shape - so
    // it is attached once through `add_inout_auto_buffer` instead of aliasing
    // an in-auto-buffer and an out-auto-buffer over the same memory.
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_inout_auto_buffer(InOutBuffer::new(fds, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.ret)
}

/// How long [`BsdService::recv_mmsg`](crate::BsdService::recv_mmsg) waits for
/// its first message.
///
/// Separate from [`SelectTimeout`] because the command carries a `timespec`
/// rather than a `timeval`: the fractional part is nanoseconds, not
/// microseconds, and conflating the two would be a thousandfold error.
#[derive(Debug, Clone, Copy)]
pub struct RecvTimeout {
    /// Whole seconds.
    pub sec: i64,
    /// Nanoseconds past `sec`.
    pub nsec: i64,
}

/// `bsdRecvMMsg`. Receives up to `vlen` messages in one request.
///
/// `buf` carries the caller's `mmsghdr` array together with the message
/// buffers it points at, and the service writes the received messages back
/// into it.
///
/// The command exists from `[3.0.0]`, but only its `[7.0.0+]` form is sent
/// here, matching libnx; on older firmware the service rejects it. Deciding
/// whether the running firmware is new enough is the caller's, since this
/// crate has no dependency on the runtime that knows the version.
pub(crate) fn recv_mmsg(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    vlen: i32,
    flags: RecvFlags,
    timeout: RecvTimeout,
) -> Result<i32, CommandError> {
    let command = Command::RecvMMsg;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = RecvMMsgIn {
        sockfd: sockfd.to_raw(),
        vlen,
        flags: flags.bits(),
        _pad: 0,
        timeout: Timespec {
            tv_sec: timeout.sec,
            tv_nsec: timeout.nsec,
        },
    };
    // A map-alias buffer rather than an auto-select one, as in libnx: the
    // message array is far larger than the pointer-buffer a session offers.
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_output_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.ret)
}

/// `bsdSendMMsg`. Sends up to `vlen` messages in one request.
///
/// `buf` carries the caller's `mmsghdr` array; the service writes each
/// message's accepted length back into it, which is why the buffer travels as
/// an output even though the payload is outbound. Same firmware note as
/// [`recv_mmsg`].
pub(crate) fn send_mmsg(
    session: BorrowedSessionHandle<'_>,
    sockfd: BsdSockFd,
    buf: &mut [u8],
    vlen: i32,
    flags: SendFlags,
) -> Result<i32, CommandError> {
    let command = Command::SendMMsg;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let payload = SendMMsgIn {
        sockfd: sockfd.to_raw(),
        vlen,
        flags: flags.bits(),
    };
    let req = cmif::CmifRequestBuilder::new(command.id())
        .with_data_value(&payload)
        .add_output_buffer(OutputBuffer::new(buf, BufferMode::Normal))
        .build();
    req.send(&mut ipc_buf, session)
        .map_err(|err| CommandError::SendRequest {
            command,
            source: err,
        })?;

    let outcome = read_service_response(&ipc_buf, command, ExtraWord::None)?;
    Ok(outcome.ret)
}
