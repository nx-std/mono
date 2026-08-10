//! CMIF protocol operations for the sfdnsres service.
//!
//! One function per `sfdnsres` command in the interface.
//! Commands that exchange serialized hostent / addrinfo wire data encode their
//! typed inputs and decode their responses through the [`crate::wire`] codec,
//! so callers send and receive owned, structurally-valid Rust types rather
//! than raw byte buffers.
//!
//! The CMIF request → send → parse lifecycle is funnelled through the single
//! [`invoke`] helper, so every layout conversion lives in one audited place.
//! Nothing here is `unsafe`: the IPC buffer arrives as a checked borrow and
//! the response is decoded through `zerocopy`.

use alloc::vec;
use core::{
    ffi::CStr,
    mem::size_of,
    net::{
        IpAddr,
        SocketAddr,
    },
};

use nx_sf::{
    cmif,
    hipc::{
        BufferMode,
        InputBuffer,
        OutputBuffer,
    },
    service::BorrowedSessionHandle,
};
use zerocopy::{
    FromBytes,
    Immutable,
    IntoBytes,
};

use crate::{
    netdb::{
        AddrInfoError,
        AddrInfoFailure,
        HostError,
        HostFailure,
        ResolverErrno,
    },
    proto::{
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
        NameInfoFlags,
    },
    wire::{
        self,
        AddrInfoHints,
        AddrInfoList,
        HostEntry,
        NameInfo,
        WireError,
    },
};

/// Encoded `0` for "no cancel token".
const NO_CANCEL: u32 = 0;

/// BSD `AF_INET` address-family tag for an IPv4 reverse lookup.
const AF_INET: u32 = 2;

/// BSD `AF_INET6` address-family tag for an IPv6 reverse lookup.
const AF_INET6: u32 = 28;

/// Scratch-buffer length for a serialized `hostent` reply (cmds 2 and 3).
///
/// Matches the size the C resolver reserves for a `gethostbyname` /
/// `gethostbyaddr` response.
const HOSTENT_BUF_LEN: usize = 0x1000;

/// Scratch-buffer length for a serialized `addrinfo` chain (cmd 6).
///
/// Matches the size the C resolver reserves for a `getaddrinfo` response.
const ADDRINFO_BUF_LEN: usize = 0x4000;

/// Scratch-buffer length for a `getnameinfo` host name reply (`NI_MAXHOST`,
/// terminator included).
const NI_MAXHOST: usize = 1025;

/// Scratch-buffer length for a `getnameinfo` service name reply (`NI_MAXSERV`).
const NI_MAXSERV: usize = 32;

/// Error returned by every `sfdnsres` CMIF command.
///
/// The four failure points — building the request, sending it, parsing the
/// response, and decoding the serialized wire format — are common to all
/// commands (they share one CMIF lifecycle), so a single error type covers
/// the whole command surface.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// Failed to send the IPC request.
    #[error("failed to send request")]
    SendRequest(#[source] cmif::SendError),
    /// Failed to parse the CMIF response.
    #[error("failed to parse response")]
    ParseResponse(#[source] cmif::ParseError),
    /// Failed to decode the serialized response wire format.
    #[error("failed to decode the response wire format")]
    Decode(#[source] WireError),
}

/// Runs one CMIF command end-to-end: build the request, send it, parse the
/// response.
///
/// `configure` receives a builder already loaded with the command id and the
/// `In` payload; it adds the send-PID flag and whatever in/out buffers the
/// command needs. The response data area is decoded as `Out` (`Out = ()` for
/// commands that carry no response payload).
fn invoke<'a, In, Out, F>(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    input: &'a In,
    configure: F,
) -> Result<Out, CommandError>
where
    In: IntoBytes + Immutable,
    Out: FromBytes + Default,
    F: FnOnce(cmif::CmifRequestBuilder<'a>) -> cmif::CmifRequestBuilder<'a>,
{
    let mut buf = nx_sys_thread_tls::ipc_buffer();

    let builder = cmif::CmifRequestBuilder::new(cmd_id).with_data_value(input);
    let req = configure(builder).build();
    req.send(&mut buf, session)
        .map_err(CommandError::SendRequest)?;

    let resp =
        cmif::parse_response_bytes(&buf, size_of::<Out>()).map_err(CommandError::ParseResponse)?;

    // `resp.payload` is exactly `size_of::<Out>()` bytes, so the read is
    // infallible; the fallback is unreachable.
    Ok(Out::read_from_bytes(resp.payload).unwrap_or_default())
}

/// Result of `GetHostByNameRequest` (cmd 2).
#[derive(Debug, Clone)]
pub struct GetHostByNameResult {
    /// What the resolver refused with, or `None` when it answered.
    ///
    /// Pairing the verdict with its POSIX code in one optional field is what
    /// keeps "succeeded, but here is an error code" from being representable.
    pub failure: Option<HostFailure>,
    /// The decoded host entry; empty when the resolver reported a failure.
    pub host: HostEntry,
}

/// Resolves a host name into a decoded host entry.
///
/// The command allocates its own scratch buffer for the serialized hostent and
/// decodes it into an owned [`HostEntry`]. On a resolver failure (`h_errno`
/// non-zero) the resolver writes no record, so `host` is the empty entry.
///
/// `name` is sent NUL-terminated, which is the length the service reads; the `&CStr`
/// carries its own terminator. Pass `None` to send a zero-length buffer
/// (the service accepts a null pointer).
pub fn get_host_by_name(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    use_nsd: bool,
    name: Option<&CStr>,
) -> Result<GetHostByNameResult, CommandError> {
    let input = GetHostByNameIn {
        use_nsd: u32::from(use_nsd),
        cancel_handle: cancel_token(cancel_handle),
        pid_placeholder: 0,
    };

    let name_bytes = name.map(CStr::to_bytes_with_nul).unwrap_or_default();
    let mut out_buffer = vec![0u8; HOSTENT_BUF_LEN];

    let out: GetHostByNameOut = invoke(session, CMD_GET_HOST_BY_NAME, &input, |builder| {
        builder
            .with_send_pid()
            .add_in_map_alias(InputBuffer::new(name_bytes, BufferMode::Normal))
            .add_out_map_alias(OutputBuffer::new(&mut out_buffer, BufferMode::Normal))
    })?;

    Ok(GetHostByNameResult {
        failure: HostError::from_wire(out.h_errno).map(|kind| HostFailure {
            kind,
            errno: ResolverErrno::from_wire(out.errno),
        }),
        host: decode_host_entry(&out_buffer, out.serialized_size)?,
    })
}

/// Result of `GetHostByAddrRequest` (cmd 3).
#[derive(Debug, Clone)]
pub struct GetHostByAddrResult {
    /// What the resolver refused with, or `None` when it answered.
    ///
    /// Pairing the verdict with its POSIX code in one optional field is what
    /// keeps "succeeded, but here is an error code" from being representable.
    pub failure: Option<HostFailure>,
    /// The decoded host entry; empty when the resolver reported a failure.
    pub host: HostEntry,
}

/// Reverse-resolves an IP address into a decoded host entry.
///
/// The address family tag and octet length sent to the resolver are derived
/// from `addr`: an `IpAddr::V4` sends its 4 octets tagged `AF_INET`, an
/// `IpAddr::V6` its 16 octets tagged `AF_INET6`. The command allocates its own
/// scratch buffer and decodes the serialized hostent into an owned
/// [`HostEntry`].
pub fn get_host_by_addr(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    addr: IpAddr,
) -> Result<GetHostByAddrResult, CommandError> {
    // Hold the octets in a fixed buffer; only the `addr_len` prefix is sent.
    let mut octets = [0u8; 16];
    let (addr_type, addr_len) = match addr {
        IpAddr::V4(v4) => {
            octets[..4].copy_from_slice(&v4.octets());
            (AF_INET, 4)
        }
        IpAddr::V6(v6) => {
            octets.copy_from_slice(&v6.octets());
            (AF_INET6, 16)
        }
    };

    let input = GetHostByAddrIn {
        addr_len: addr_len as u32,
        addr_type,
        cancel_handle: cancel_token(cancel_handle),
        _padding: 0,
        pid_placeholder: 0,
    };

    let mut out_buffer = vec![0u8; HOSTENT_BUF_LEN];

    let out: GetHostByAddrOut = invoke(session, CMD_GET_HOST_BY_ADDR, &input, |builder| {
        builder
            .add_in_map_alias(InputBuffer::new(&octets[..addr_len], BufferMode::Normal))
            .add_out_map_alias(OutputBuffer::new(&mut out_buffer, BufferMode::Normal))
    })?;

    Ok(GetHostByAddrResult {
        failure: HostError::from_wire(out.h_errno).map(|kind| HostFailure {
            kind,
            errno: ResolverErrno::from_wire(out.errno),
        }),
        host: decode_host_entry(&out_buffer, out.serialized_size)?,
    })
}

/// Writes the textual description of an `h_errno` value into `out_str`.
pub fn get_host_string_error(
    session: BorrowedSessionHandle<'_>,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), CommandError> {
    string_error_impl(session, CMD_GET_HOST_STRING_ERROR, err, out_str)
}

/// Writes the textual description of a `getaddrinfo` error code into `out_str`.
pub fn get_gai_string_error(
    session: BorrowedSessionHandle<'_>,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), CommandError> {
    string_error_impl(session, CMD_GET_GAI_STRING_ERROR, err, out_str)
}

/// Result of `GetAddrInfoRequest` (cmd 6).
#[derive(Debug, Clone)]
pub struct GetAddrInfoResult {
    /// What the resolver refused with, or `None` when it answered.
    pub failure: Option<AddrInfoFailure>,
    /// The decoded address records; empty when the resolver returned none.
    pub addrs: AddrInfoList,
}

/// Performs a `getaddrinfo`-style resolution.
///
/// `node`, `service` are sent NUL-terminated (the `&CStr` carries its own
/// terminator) or `None` for a null pointer with zero length. `hints` is the
/// typed lookup template; it is serialized into the request buffer here. The
/// command allocates its own scratch buffer and decodes the serialized
/// addrinfo chain into an owned [`AddrInfoList`].
pub fn get_addr_info(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    use_nsd: bool,
    node: Option<&CStr>,
    service: Option<&CStr>,
    hints: &AddrInfoHints,
) -> Result<GetAddrInfoResult, CommandError> {
    let input = GetAddrInfoIn {
        use_nsd: u32::from(use_nsd),
        cancel_handle: cancel_token(cancel_handle),
        pid_placeholder: 0,
    };

    let node_bytes = node.map(CStr::to_bytes_with_nul).unwrap_or_default();
    let svc_bytes = service.map(CStr::to_bytes_with_nul).unwrap_or_default();
    let hints_buf = wire::encode_hints(hints);
    let mut out_buffer = vec![0u8; ADDRINFO_BUF_LEN];

    let out: GetAddrInfoOut = invoke(session, CMD_GET_ADDR_INFO, &input, |builder| {
        builder
            .with_send_pid()
            .add_in_map_alias(InputBuffer::new(node_bytes, BufferMode::Normal))
            .add_in_map_alias(InputBuffer::new(svc_bytes, BufferMode::Normal))
            .add_in_map_alias(InputBuffer::new(&hints_buf, BufferMode::Normal))
            .add_out_map_alias(OutputBuffer::new(&mut out_buffer, BufferMode::Normal))
    })?;

    let len = (out.serialized_size as usize).min(out_buffer.len());
    let addrs = wire::decode_addrinfo_list(&out_buffer[..len]).map_err(CommandError::Decode)?;

    Ok(GetAddrInfoResult {
        failure: AddrInfoError::from_wire(out.ret).map(|kind| AddrInfoFailure {
            kind,
            errno: ResolverErrno::from_wire(out.errno),
        }),
        addrs,
    })
}

/// Result of `GetNameInfoRequest` (cmd 7).
#[derive(Debug, Clone)]
pub struct GetNameInfoResult {
    /// What the resolver refused with, or `None` when it answered.
    pub failure: Option<AddrInfoFailure>,
    /// The decoded host and service names.
    pub name: NameInfo,
}

/// Performs a `getnameinfo`-style reverse lookup.
///
/// `addr` is serialized into the raw BSD `sockaddr` form the service expects.
/// The command allocates its own `host` / `serv` scratch buffers and decodes
/// each into the returned [`NameInfo`], clamping at the first NUL byte.
pub fn get_name_info(
    session: BorrowedSessionHandle<'_>,
    cancel_handle: Option<CancelHandle>,
    flags: NameInfoFlags,
    addr: &SocketAddr,
) -> Result<GetNameInfoResult, CommandError> {
    let input = GetNameInfoIn {
        flags: flags.to_raw(),
        cancel_handle: cancel_token(cancel_handle),
        pid_placeholder: 0,
    };

    let sockaddr = wire::encode_sockaddr(addr);
    let mut host = vec![0u8; NI_MAXHOST];
    let mut serv = vec![0u8; NI_MAXSERV];

    let out: GetNameInfoOut = invoke(session, CMD_GET_NAME_INFO, &input, |builder| {
        builder
            .with_send_pid()
            .add_in_map_alias(InputBuffer::new(&sockaddr, BufferMode::Normal))
            .add_out_map_alias(OutputBuffer::new(&mut host, BufferMode::Normal))
            .add_out_map_alias(OutputBuffer::new(&mut serv, BufferMode::Normal))
    })?;

    Ok(GetNameInfoResult {
        failure: AddrInfoError::from_wire(out.ret).map(|kind| AddrInfoFailure {
            kind,
            errno: ResolverErrno::from_wire(out.errno),
        }),
        name: wire::decode_nameinfo(&host, &serv),
    })
}

/// Allocates a fresh cancel-token from the service.
pub fn get_cancel_handle(session: BorrowedSessionHandle<'_>) -> Result<CancelHandle, CommandError> {
    // The input carries a `u64 pid_placeholder` so the request still
    // carries an 8-byte payload alongside the send-PID flag.
    let raw: u32 = invoke(session, CMD_GET_CANCEL_HANDLE, &0u64, |builder| {
        builder.with_send_pid()
    })?;

    Ok(CancelHandle::from_raw(raw))
}

/// Cancels any pending resolver call tagged with `handle`.
pub fn cancel(
    session: BorrowedSessionHandle<'_>,
    handle: CancelHandle,
) -> Result<(), CommandError> {
    let input = CancelIn {
        cancel_handle: handle.to_raw(),
        _padding: 0,
        pid_placeholder: 0,
    };

    invoke(session, CMD_CANCEL, &input, |builder| {
        builder.with_send_pid()
    })
}

#[inline]
fn cancel_token(handle: Option<CancelHandle>) -> u32 {
    match handle {
        Some(h) => h.to_raw(),
        None => NO_CANCEL,
    }
}

/// Decodes the prefix of a `hostent` scratch buffer the resolver actually
/// wrote.
///
/// `size` is the resolver-reported byte count, clamped to `buffer.len()` so a
/// bogus oversized count can never yield an out-of-bounds slice. A resolver
/// failure leaves nothing written, so an empty prefix decodes to the empty
/// [`HostEntry`] rather than a decode error.
fn decode_host_entry(buffer: &[u8], size: u32) -> Result<HostEntry, CommandError> {
    let len = (size as usize).min(buffer.len());
    let prefix = &buffer[..len];
    if prefix.is_empty() {
        return Ok(HostEntry::default());
    }
    wire::decode_hostent(prefix).map_err(CommandError::Decode)
}

/// Shared implementation of the two error-string commands (cmds 4 and 5):
/// send a `u32` error code, receive its textual description into `out_str`.
fn string_error_impl(
    session: BorrowedSessionHandle<'_>,
    cmd_id: u32,
    err: u32,
    out_str: &mut [u8],
) -> Result<(), CommandError> {
    invoke(session, cmd_id, &err, |builder| {
        builder.add_out_map_alias(OutputBuffer::new(out_str, BufferMode::Normal))
    })
}
