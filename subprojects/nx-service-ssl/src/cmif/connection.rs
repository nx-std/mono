//! ISslConnection CMIF dispatch implementations.

use core::mem::size_of;

use nx_service_bsd::RawSockAddr;
use nx_sf::service::{
    BufferAttr,
    DispatchError,
    DomainTarget,
};
use zerocopy::IntoBytes as _;

use crate::{
    dispatch::{
        dispatch_in,
        dispatch_in_out_u32,
        dispatch_no_io,
        dispatch_out_u32,
    },
    proto,
    types::{
        CipherInfo,
        ConnSetOptionIn,
        GetNextAlpnProtoOut,
        HandshakeServerCertOut,
        IoMode,
        OptionType,
        PollEvent,
        PollIn,
        PrivateOptionType,
        RenegotiationMode,
        SessionCacheMode,
        SetPrivateOptionIn,
        SetPrivateOptionLegacyIn,
        SocketFd,
        VerifyOption,
    },
};

/// Sets the socket descriptor on the connection.
pub(crate) fn set_socket_descriptor<'d>(
    object: impl DomainTarget<'d>,
    sockfd: impl Into<SocketFd>,
) -> Result<Option<SocketFd>, DispatchError> {
    let sockfd = sockfd.into();
    let result = dispatch_in_out_u32(object, proto::CONN_SET_SOCKET_DESCRIPTOR, sockfd)?;
    Ok((result as i32).try_into().ok())
}

/// Sets the host name for TLS verification.
pub(crate) fn set_host_name<'d>(
    object: impl DomainTarget<'d>,
    name: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(proto::CONN_SET_HOST_NAME)
        .in_buffer(name, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets the verify option bitmask.
pub(crate) fn set_verify_option<'d>(
    object: impl DomainTarget<'d>,
    verify_option: VerifyOption,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_VERIFY_OPTION, verify_option.bits())
}

/// Sets the I/O mode.
pub(crate) fn set_io_mode<'d>(
    object: impl DomainTarget<'d>,
    mode: IoMode,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_IO_MODE, mode as u32)
}

/// Gets the socket descriptor.
pub(crate) fn get_socket_descriptor<'d>(
    object: impl DomainTarget<'d>,
) -> Result<Option<SocketFd>, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_SOCKET_DESCRIPTOR).map(|v| (v as i32).try_into().ok())
}

/// Gets the host name string.
pub(crate) fn get_host_name<'d>(
    object: impl DomainTarget<'d>,
    buffer: &mut [u8],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_GET_HOST_NAME)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Gets the verify option bitmask.
pub(crate) fn get_verify_option<'d>(object: impl DomainTarget<'d>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_VERIFY_OPTION)
}

/// Gets the I/O mode.
pub(crate) fn get_io_mode<'d>(object: impl DomainTarget<'d>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_IO_MODE)
}

/// Performs a TLS handshake without requesting server cert.
pub(crate) fn do_handshake<'d>(object: impl DomainTarget<'d>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONN_DO_HANDSHAKE)
}

/// Performs a TLS handshake and retrieves server cert data.
pub(crate) fn do_handshake_get_server_cert<'d>(
    object: impl DomainTarget<'d>,
    server_certbuf: &mut [u8],
) -> Result<HandshakeServerCertOut, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_DO_HANDSHAKE_GET_SERVER_CERT)
        .out_size(size_of::<HandshakeServerCertOut>())
        .out_buffer(server_certbuf, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<HandshakeServerCertOut>())
}

/// Reads data from the TLS connection.
pub(crate) fn read<'d>(
    object: impl DomainTarget<'d>,
    buffer: &mut [u8],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_READ)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Writes data to the TLS connection.
pub(crate) fn write<'d>(
    object: impl DomainTarget<'d>,
    buffer: &[u8],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_WRITE)
        .out_size(size_of::<u32>())
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Gets the number of pending bytes.
pub(crate) fn pending<'d>(object: impl DomainTarget<'d>) -> Result<i32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_PENDING).map(|v| v as i32)
}

/// Peeks at data without consuming it.
pub(crate) fn peek<'d>(
    object: impl DomainTarget<'d>,
    buffer: &mut [u8],
) -> Result<u32, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_PEEK)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u32>())
}

/// Polls the connection for events.
pub(crate) fn poll<'d>(
    object: impl DomainTarget<'d>,
    in_pollevent: PollEvent,
    timeout: u32,
) -> Result<u32, DispatchError> {
    let input = PollIn {
        in_pollevent: in_pollevent.bits(),
        timeout,
    };
    dispatch_in_out_u32(object, proto::CONN_POLL, input)
}

/// Gets the verify cert error (clears the stored value).
pub(crate) fn get_verify_cert_error<'d>(
    object: impl DomainTarget<'d>,
) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONN_GET_VERIFY_CERT_ERROR)
}

/// Gets the needed server cert buffer size.
pub(crate) fn get_needed_server_cert_buffer_size<'d>(
    object: impl DomainTarget<'d>,
) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_NEEDED_SERVER_CERT_BUFFER_SIZE)
}

/// Sets the session cache mode.
pub(crate) fn set_session_cache_mode<'d>(
    object: impl DomainTarget<'d>,
    mode: SessionCacheMode,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_SESSION_CACHE_MODE, mode as u32)
}

/// Gets the session cache mode.
pub(crate) fn get_session_cache_mode<'d>(
    object: impl DomainTarget<'d>,
) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_SESSION_CACHE_MODE)
}

/// Flushes the connection's session cache.
pub(crate) fn flush_session_cache<'d>(object: impl DomainTarget<'d>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONN_FLUSH_SESSION_CACHE)
}

/// Sets the renegotiation mode.
pub(crate) fn set_renegotiation_mode<'d>(
    object: impl DomainTarget<'d>,
    mode: RenegotiationMode,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_RENEGOTIATION_MODE, mode as u32)
}

/// Gets the renegotiation mode.
pub(crate) fn get_renegotiation_mode<'d>(
    object: impl DomainTarget<'d>,
) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_RENEGOTIATION_MODE)
}

/// Sets a connection option (bool flag + option type).
pub(crate) fn set_option<'d>(
    object: impl DomainTarget<'d>,
    option: OptionType,
    flag: bool,
) -> Result<(), DispatchError> {
    let input = ConnSetOptionIn {
        flag: u8::from(flag),
        _pad: [0; 3],
        option: option as u32,
    };
    dispatch_in(object, proto::CONN_SET_OPTION, input)
}

/// Gets a connection option.
pub(crate) fn get_option<'d>(
    object: impl DomainTarget<'d>,
    option: OptionType,
) -> Result<bool, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_GET_OPTION)
        .in_raw((option as u32).as_bytes())
        .out_size(size_of::<u8>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u8>() & 1 != 0)
}

/// Gets verify cert errors into a buffer.
pub(crate) fn get_verify_cert_errors<'d>(
    object: impl DomainTarget<'d>,
    errors: &mut [u32],
) -> Result<(u32, u32), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_GET_VERIFY_CERT_ERRORS)
        .out_size(size_of::<u32>() * 2)
        .out_buffer(errors.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    let [out0, out1] = *result.value::<[u32; 2]>();
    Ok((out0, out1))
}

/// Gets cipher info (4.0.0+).
pub(crate) fn get_cipher_info<'d>(
    object: impl DomainTarget<'d>,
    out: &mut CipherInfo,
) -> Result<(), DispatchError> {
    let val: u32 = 1;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(proto::CONN_GET_CIPHER_INFO)
        .in_raw(val.as_bytes())
        .out_buffer(out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets the next ALPN protocol list (9.0.0+).
pub(crate) fn set_next_alpn_proto<'d>(
    object: impl DomainTarget<'d>,
    proto_list: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(proto::CONN_SET_NEXT_ALPN_PROTO)
        .in_buffer(proto_list, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets the next ALPN protocol (9.0.0+).
pub(crate) fn get_next_alpn_proto<'d>(
    object: impl DomainTarget<'d>,
    buffer: &mut [u8],
) -> Result<GetNextAlpnProtoOut, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_GET_NEXT_ALPN_PROTO)
        .out_size(size_of::<GetNextAlpnProtoOut>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(*result.value::<GetNextAlpnProtoOut>())
}

/// Sets DTLS socket descriptor (16.0.0+).
pub(crate) fn set_dtls_socket_descriptor<'d>(
    object: impl DomainTarget<'d>,
    sockfd: impl Into<SocketFd>,
    sockaddr: &RawSockAddr,
) -> Result<Option<SocketFd>, DispatchError> {
    let sockfd = sockfd.into();
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_SET_DTLS_SOCKET_DESCRIPTOR)
        .in_raw(sockfd.as_bytes())
        .out_size(size_of::<i32>())
        .in_buffer(sockaddr.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok((*result.value::<i32>()).try_into().ok())
}

/// Gets DTLS handshake timeout in nanoseconds (16.0.0+).
pub(crate) fn get_dtls_handshake_timeout<'d>(
    object: impl DomainTarget<'d>,
) -> Result<u64, DispatchError> {
    let mut out: u64 = 0;
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(proto::CONN_GET_DTLS_HANDSHAKE_TIMEOUT)
        .out_buffer(out.as_mut_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)?;
    Ok(out)
}

/// Sets a private option (pre-17.0.0, bool+option layout).
pub(crate) fn set_private_option_legacy<'d>(
    object: impl DomainTarget<'d>,
    option: PrivateOptionType,
    value: bool,
) -> Result<(), DispatchError> {
    let input = SetPrivateOptionLegacyIn {
        value: u8::from(value),
        _pad: [0; 3],
        option: option as u32,
    };
    dispatch_in(object, proto::CONN_SET_PRIVATE_OPTION, input)
}

/// Sets a private option (17.0.0+, option+value layout).
pub(crate) fn set_private_option<'d>(
    object: impl DomainTarget<'d>,
    option: PrivateOptionType,
    value: u32,
) -> Result<(), DispatchError> {
    let input = SetPrivateOptionIn {
        option: option as u32,
        value,
    };
    dispatch_in(object, proto::CONN_SET_PRIVATE_OPTION, input)
}

/// Sets SRTP ciphers (16.0.0+).
pub(crate) fn set_srtp_ciphers<'d>(
    object: impl DomainTarget<'d>,
    ciphers: &[u16],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(proto::CONN_SET_SRTP_CIPHERS)
        .in_buffer(ciphers.as_bytes(), BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Gets the negotiated SRTP cipher (16.0.0+).
pub(crate) fn get_srtp_cipher<'d>(object: impl DomainTarget<'d>) -> Result<u16, DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    let result = object
        .request(proto::CONN_GET_SRTP_CIPHER)
        .out_size(size_of::<u16>())
        .send(&mut ipc_buf)?;
    Ok(*result.value::<u16>())
}

/// Exports keying material (16.0.0+).
pub(crate) fn export_keying_material<'d>(
    object: impl DomainTarget<'d>,
    outbuf: &mut [u8],
    label: &[u8],
    context: &[u8],
) -> Result<(), DispatchError> {
    let mut ipc_buf = nx_sys_thread_tls::ipc_buffer();

    object
        .request(proto::CONN_EXPORT_KEYING_MATERIAL)
        .out_buffer(outbuf, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(label, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(context, BufferAttr::HIPC_MAP_ALIAS)
        .send(&mut ipc_buf)
        .map(|_| ())
}

/// Sets I/O timeout (16.0.0+).
pub(crate) fn set_io_timeout<'d>(
    object: impl DomainTarget<'d>,
    timeout: u32,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_IO_TIMEOUT, timeout)
}

/// Gets I/O timeout (16.0.0+).
pub(crate) fn get_io_timeout<'d>(object: impl DomainTarget<'d>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_IO_TIMEOUT)
}
