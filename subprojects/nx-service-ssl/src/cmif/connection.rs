//! ISslConnection CMIF dispatch implementations.

use core::mem::size_of;

use nx_sf::service::{BufferAttr, DispatchError, DomainObject};

use crate::{
    dispatch::{dispatch_in, dispatch_in_out_u32, dispatch_no_io, dispatch_out_u32},
    proto,
    types::{
        CipherInfo, ConnSetOptionIn, GetNextAlpnProtoOut, HandshakeServerCertOut, PollIn,
        SetPrivateOptionIn, SetPrivateOptionLegacyIn,
    },
};

/// Sets the socket descriptor on the connection.
pub(crate) fn set_socket_descriptor(
    object: &DomainObject<'_>,
    sockfd: i32,
) -> Result<i32, DispatchError> {
    let result = dispatch_in_out_u32(object, proto::CONN_SET_SOCKET_DESCRIPTOR, sockfd)?;
    Ok(result as i32)
}

/// Sets the host name for TLS verification.
pub(crate) fn set_host_name(object: &DomainObject<'_>, name: &[u8]) -> Result<(), DispatchError> {
    object
        .dispatch(proto::CONN_SET_HOST_NAME)
        .in_buffer(name, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// Sets the verify option bitmask.
pub(crate) fn set_verify_option(
    object: &DomainObject<'_>,
    verify_option: u32,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_VERIFY_OPTION, verify_option)
}

/// Sets the I/O mode.
pub(crate) fn set_io_mode(object: &DomainObject<'_>, mode: u32) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_IO_MODE, mode)
}

/// Gets the socket descriptor.
pub(crate) fn get_socket_descriptor(object: &DomainObject<'_>) -> Result<i32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_SOCKET_DESCRIPTOR).map(|v| v as i32)
}

/// Gets the host name string.
pub(crate) fn get_host_name(
    object: &DomainObject<'_>,
    buffer: &mut [u8],
) -> Result<u32, DispatchError> {
    let result = object
        .dispatch(proto::CONN_GET_HOST_NAME)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Gets the verify option bitmask.
pub(crate) fn get_verify_option(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_VERIFY_OPTION)
}

/// Gets the I/O mode.
pub(crate) fn get_io_mode(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_IO_MODE)
}

/// Performs a TLS handshake without requesting server cert.
pub(crate) fn do_handshake(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONN_DO_HANDSHAKE)
}

/// Performs a TLS handshake and retrieves server cert data.
pub(crate) fn do_handshake_get_server_cert(
    object: &DomainObject<'_>,
    server_certbuf: &mut [u8],
) -> Result<HandshakeServerCertOut, DispatchError> {
    let result = object
        .dispatch(proto::CONN_DO_HANDSHAKE_GET_SERVER_CERT)
        .out_size(size_of::<HandshakeServerCertOut>())
        .out_buffer(server_certbuf, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    // SAFETY: response data is at least `size_of::<HandshakeServerCertOut>()` bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<HandshakeServerCertOut>()) })
}

/// Reads data from the TLS connection.
pub(crate) fn read(object: &DomainObject<'_>, buffer: &mut [u8]) -> Result<u32, DispatchError> {
    let result = object
        .dispatch(proto::CONN_READ)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Writes data to the TLS connection.
pub(crate) fn write(object: &DomainObject<'_>, buffer: &[u8]) -> Result<u32, DispatchError> {
    let result = object
        .dispatch(proto::CONN_WRITE)
        .out_size(size_of::<u32>())
        .in_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Gets the number of pending bytes.
pub(crate) fn pending(object: &DomainObject<'_>) -> Result<i32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_PENDING).map(|v| v as i32)
}

/// Peeks at data without consuming it.
pub(crate) fn peek(object: &DomainObject<'_>, buffer: &mut [u8]) -> Result<u32, DispatchError> {
    let result = object
        .dispatch(proto::CONN_PEEK)
        .out_size(size_of::<u32>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    Ok(u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]))
}

/// Polls the connection for events.
pub(crate) fn poll(
    object: &DomainObject<'_>,
    in_pollevent: u32,
    timeout: u32,
) -> Result<u32, DispatchError> {
    let input = PollIn {
        in_pollevent,
        timeout,
    };
    dispatch_in_out_u32(object, proto::CONN_POLL, input)
}

/// Gets the verify cert error (clears the stored value).
pub(crate) fn get_verify_cert_error(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONN_GET_VERIFY_CERT_ERROR)
}

/// Gets the needed server cert buffer size.
pub(crate) fn get_needed_server_cert_buffer_size(
    object: &DomainObject<'_>,
) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_NEEDED_SERVER_CERT_BUFFER_SIZE)
}

/// Sets the session cache mode.
pub(crate) fn set_session_cache_mode(
    object: &DomainObject<'_>,
    mode: u32,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_SESSION_CACHE_MODE, mode)
}

/// Gets the session cache mode.
pub(crate) fn get_session_cache_mode(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_SESSION_CACHE_MODE)
}

/// Flushes the connection's session cache.
pub(crate) fn flush_session_cache(object: &DomainObject<'_>) -> Result<(), DispatchError> {
    dispatch_no_io(object, proto::CONN_FLUSH_SESSION_CACHE)
}

/// Sets the renegotiation mode.
pub(crate) fn set_renegotiation_mode(
    object: &DomainObject<'_>,
    mode: u32,
) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_RENEGOTIATION_MODE, mode)
}

/// Gets the renegotiation mode.
pub(crate) fn get_renegotiation_mode(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_RENEGOTIATION_MODE)
}

/// Sets a connection option (bool flag + option type).
pub(crate) fn set_option(
    object: &DomainObject<'_>,
    option: u32,
    flag: bool,
) -> Result<(), DispatchError> {
    let input = ConnSetOptionIn {
        flag: u8::from(flag),
        _pad: [0; 3],
        option,
    };
    dispatch_in(object, proto::CONN_SET_OPTION, input)
}

/// Gets a connection option.
pub(crate) fn get_option(object: &DomainObject<'_>, option: u32) -> Result<bool, DispatchError> {
    // SAFETY: `option` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const option).cast::<u8>(), size_of::<u32>()) };
    let result = object
        .dispatch(proto::CONN_GET_OPTION)
        .in_raw(in_bytes)
        .out_size(size_of::<u8>())
        .send()?;
    Ok(result.data[0] & 1 != 0)
}

/// Gets verify cert errors into a buffer.
pub(crate) fn get_verify_cert_errors(
    object: &DomainObject<'_>,
    errors: &mut [u32],
) -> Result<(u32, u32), DispatchError> {
    // SAFETY: `errors` is a valid `&mut [u32]` slice; viewing it as bytes for
    // the OUT buffer is sound.
    let errors_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            errors.as_mut_ptr().cast::<u8>(),
            core::mem::size_of_val(errors),
        )
    };
    let result = object
        .dispatch(proto::CONN_GET_VERIFY_CERT_ERRORS)
        .out_size(size_of::<u32>() * 2)
        .out_buffer(errors_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    let out0 = u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]);
    let out1 = u32::from_le_bytes([
        result.data[4],
        result.data[5],
        result.data[6],
        result.data[7],
    ]);
    Ok((out0, out1))
}

/// Gets cipher info (4.0.0+).
pub(crate) fn get_cipher_info(
    object: &DomainObject<'_>,
    out: &mut CipherInfo,
) -> Result<(), DispatchError> {
    let val: u32 = 1;
    // SAFETY: `val` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const val).cast::<u8>(), size_of::<u32>()) };
    // SAFETY: `out` is a valid `&mut CipherInfo`; viewing its bytes for the
    // OUT buffer is sound.
    let out_bytes = unsafe {
        core::slice::from_raw_parts_mut(
            (out as *mut CipherInfo).cast::<u8>(),
            size_of::<CipherInfo>(),
        )
    };
    object
        .dispatch(proto::CONN_GET_CIPHER_INFO)
        .in_raw(in_bytes)
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// Sets the next ALPN protocol list (9.0.0+).
pub(crate) fn set_next_alpn_proto(
    object: &DomainObject<'_>,
    proto_list: &[u8],
) -> Result<(), DispatchError> {
    object
        .dispatch(proto::CONN_SET_NEXT_ALPN_PROTO)
        .in_buffer(proto_list, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// Gets the next ALPN protocol (9.0.0+).
pub(crate) fn get_next_alpn_proto(
    object: &DomainObject<'_>,
    buffer: &mut [u8],
) -> Result<GetNextAlpnProtoOut, DispatchError> {
    let result = object
        .dispatch(proto::CONN_GET_NEXT_ALPN_PROTO)
        .out_size(size_of::<GetNextAlpnProtoOut>())
        .out_buffer(buffer, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    // SAFETY: response data is at least `size_of::<GetNextAlpnProtoOut>()` bytes.
    Ok(unsafe { core::ptr::read_unaligned(result.data.as_ptr().cast::<GetNextAlpnProtoOut>()) })
}

/// Sets DTLS socket descriptor (16.0.0+).
pub(crate) fn set_dtls_socket_descriptor(
    object: &DomainObject<'_>,
    sockfd: i32,
    sockaddr: &[u8],
) -> Result<i32, DispatchError> {
    // SAFETY: `sockfd` is a `Copy` value on the stack, valid until `.send()`
    // returns; viewing its bytes as a slice is sound.
    let in_bytes =
        unsafe { core::slice::from_raw_parts((&raw const sockfd).cast::<u8>(), size_of::<i32>()) };
    let result = object
        .dispatch(proto::CONN_SET_DTLS_SOCKET_DESCRIPTOR)
        .in_raw(in_bytes)
        .out_size(size_of::<i32>())
        .in_buffer(sockaddr, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    let raw = u32::from_le_bytes([
        result.data[0],
        result.data[1],
        result.data[2],
        result.data[3],
    ]);
    Ok(raw as i32)
}

/// Gets DTLS handshake timeout in nanoseconds (16.0.0+).
pub(crate) fn get_dtls_handshake_timeout(object: &DomainObject<'_>) -> Result<u64, DispatchError> {
    let mut out: u64 = 0;
    // SAFETY: `out` is a valid local u64; viewing its bytes for the OUT buffer
    // is sound, and the slice borrows `out`.
    let out_bytes =
        unsafe { core::slice::from_raw_parts_mut((&raw mut out).cast::<u8>(), size_of::<u64>()) };
    object
        .dispatch(proto::CONN_GET_DTLS_HANDSHAKE_TIMEOUT)
        .out_buffer(out_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()?;
    Ok(out)
}

/// Sets a private option (pre-17.0.0, bool+option layout).
pub(crate) fn set_private_option_legacy(
    object: &DomainObject<'_>,
    option: u32,
    value: bool,
) -> Result<(), DispatchError> {
    let input = SetPrivateOptionLegacyIn {
        value: u8::from(value),
        _pad: [0; 3],
        option,
    };
    dispatch_in(object, proto::CONN_SET_PRIVATE_OPTION, input)
}

/// Sets a private option (17.0.0+, option+value layout).
pub(crate) fn set_private_option(
    object: &DomainObject<'_>,
    option: u32,
    value: u32,
) -> Result<(), DispatchError> {
    let input = SetPrivateOptionIn { option, value };
    dispatch_in(object, proto::CONN_SET_PRIVATE_OPTION, input)
}

/// Sets SRTP ciphers (16.0.0+).
pub(crate) fn set_srtp_ciphers(
    object: &DomainObject<'_>,
    ciphers: &[u16],
) -> Result<(), DispatchError> {
    // SAFETY: `ciphers` is a valid `&[u16]` slice; viewing it as bytes for
    // the IN buffer is sound.
    let cipher_bytes = unsafe {
        core::slice::from_raw_parts(
            ciphers.as_ptr().cast::<u8>(),
            core::mem::size_of_val(ciphers),
        )
    };
    object
        .dispatch(proto::CONN_SET_SRTP_CIPHERS)
        .in_buffer(cipher_bytes, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// Gets the negotiated SRTP cipher (16.0.0+).
pub(crate) fn get_srtp_cipher(object: &DomainObject<'_>) -> Result<u16, DispatchError> {
    let result = object
        .dispatch(proto::CONN_GET_SRTP_CIPHER)
        .out_size(size_of::<u16>())
        .send()?;
    Ok(u16::from_le_bytes([result.data[0], result.data[1]]))
}

/// Exports keying material (16.0.0+).
pub(crate) fn export_keying_material(
    object: &DomainObject<'_>,
    outbuf: &mut [u8],
    label: &[u8],
    context: &[u8],
) -> Result<(), DispatchError> {
    object
        .dispatch(proto::CONN_EXPORT_KEYING_MATERIAL)
        .out_buffer(outbuf, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(label, BufferAttr::HIPC_MAP_ALIAS)
        .in_buffer(context, BufferAttr::HIPC_MAP_ALIAS)
        .send()
        .map(|_| ())
}

/// Sets I/O timeout (16.0.0+).
pub(crate) fn set_io_timeout(object: &DomainObject<'_>, timeout: u32) -> Result<(), DispatchError> {
    dispatch_in(object, proto::CONN_SET_IO_TIMEOUT, timeout)
}

/// Gets I/O timeout (16.0.0+).
pub(crate) fn get_io_timeout(object: &DomainObject<'_>) -> Result<u32, DispatchError> {
    dispatch_out_u32(object, proto::CONN_GET_IO_TIMEOUT)
}
