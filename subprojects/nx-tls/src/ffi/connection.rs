//! The commands an `ISslConnection` answers, from C.
//!
//! Each takes a pointer to a libnx `SslConnection`, which is a service struct and nothing else, and
//! addresses the object it names. The C caller created the connection through
//! [`sslContextCreateConnection`](super::context::__nx_tls__sslContextCreateConnection) and closes
//! it through [`sslConnectionClose`], so nothing here closes one except that.
//!
//! The three socket hand-offs live in [`super::socket`] rather than here, because they answer in
//! `errno` rather than a result code and translate a descriptor on the way in.

use core::ffi::{
    c_char,
    c_int,
    c_void,
};

use nx_service_ssl::{
    CipherInfo,
    IoMode,
    OptionType,
    PollEvent,
    PrivateOptionType,
    RenegotiationMode,
    SessionCacheMode,
    SocketFd,
    VerifyOption,
};
use nx_sf::error::{
    ResultCode,
    ToResultCode as _,
};

use super::{
    buffer,
    firmware,
    object,
    result,
};

/// Closes a connection.
///
/// # Safety
///
/// `connection` must be null or point to a writable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionClose(connection: *mut c_void) {
    // SAFETY: the caller guarantees a writable `SslConnection`, or null, which nothing else
    // addresses for the length of this call.
    if let Some(service) = unsafe { object::service_at(connection) } {
        service.close();
    }
}

/// Hands a socket descriptor the BSD service issued to a TLS connection.
///
/// This is the service-tier command, which takes the descriptor the *service* knows.
/// [`socketSslConnectionSetSocketDescriptor`](super::socket::__nx_tls__socketSslConnectionSetSocketDescriptor)
/// is the wrapper a socket program calls, which translates a process descriptor and delegates
/// here.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out_sockfd` must
/// be null or point to a writable `c_int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetSocketDescriptor(
    connection: *mut c_void,
    sockfd: c_int,
    out_sockfd: *mut c_int,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(sockfd) = SocketFd::try_from(sockfd) else {
        return result::bad_input();
    };

    // SAFETY: the caller guarantees a writable `c_int` at `out_sockfd`, or null.
    unsafe { report_descriptor(connection.set_socket_descriptor(sockfd), out_sockfd) }
}

/// Takes the socket descriptor back from a TLS connection.
///
/// The service-tier counterpart of the set command above, reporting the descriptor the *service*
/// knows rather than one from the process's table.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `sockfd` must be
/// null or point to a writable `c_int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetSocketDescriptor(
    connection: *mut c_void,
    sockfd: *mut c_int,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees a writable `c_int` at `sockfd`, or null.
    unsafe { report_descriptor(connection.get_socket_descriptor(), sockfd) }
}

/// Hands a datagram socket descriptor the BSD service issued to a TLS connection (16.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `buf` must point to
/// `size` readable bytes holding a socket address, and `out_sockfd` must be null or point to a
/// writable `c_int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetDtlsSocketDescriptor(
    connection: *mut c_void,
    sockfd: c_int,
    buf: *const c_void,
    size: usize,
    out_sockfd: *mut c_int,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(sockfd) = SocketFd::try_from(sockfd) else {
        return result::bad_input();
    };

    // SAFETY: the caller guarantees `size` readable bytes at `buf`, which is this function's own
    // precondition; the length is what bounds the read.
    let Some(addr) = (unsafe { nx_sys_net::ffi::abi::borrow_sockaddr(buf, size as u32) }) else {
        return result::bad_input();
    };

    // SAFETY: the caller guarantees a writable `c_int` at `out_sockfd`, or null.
    unsafe {
        report_descriptor(
            connection.set_dtls_socket_descriptor(sockfd, &addr),
            out_sockfd,
        )
    }
}

/// Sets the host name TLS verification checks the certificate against.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `host` to
/// `host_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetHostName(
    connection: *mut c_void,
    host: *const c_char,
    host_len: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    if host.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `host_len` readable bytes at a non-null `host`.
    let host = unsafe { buffer::bytes(host.cast(), host_len) };

    result::report(connection.set_host_name(host))
}

/// Sets the verify option bitmask.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetVerifyOption(
    connection: *mut c_void,
    verify_option: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    result::report(connection.set_verify_option(VerifyOption::from_bits_retain(verify_option)))
}

/// Sets the I/O mode.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetIoMode(
    connection: *mut c_void,
    mode: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(mode) = IoMode::try_from(mode) else {
        return result::bad_input();
    };

    result::report(connection.set_io_mode(mode))
}

/// Reads the host name back.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `host` to `host_len`
/// writable bytes, and `out` must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetHostName(
    connection: *mut c_void,
    host: *mut c_char,
    host_len: u32,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees `host_len` writable bytes at `host`, exclusively held for this
    // call.
    let host = unsafe { buffer::bytes_mut(host.cast(), host_len) };

    match connection.get_host_name(host) {
        Ok(len) => {
            // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
            unsafe { buffer::write_out(out, len) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reads the verify option bitmask back.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetVerifyOption(
    connection: *mut c_void,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    match connection.get_verify_option() {
        Ok(options) => {
            // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
            unsafe { buffer::write_out(out, options.bits()) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reads the I/O mode back.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetIoMode(
    connection: *mut c_void,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
    unsafe { report_u32(connection.get_io_mode(), out) }
}

/// Performs the TLS handshake, optionally collecting the server's certificate chain.
///
/// A null certificate buffer selects the handshake that asks for no chain, which is how the C API
/// spells the two commands as one entry point.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `server_certbuf` must be
/// null or point to `server_certbuf_size` writable bytes, and each out-pointer must be null or
/// point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionDoHandshake(
    connection: *mut c_void,
    out_size: *mut u32,
    total_certs: *mut u32,
    server_certbuf: *mut c_void,
    server_certbuf_size: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    if server_certbuf.is_null() {
        return result::report(connection.do_handshake());
    }

    // SAFETY: the caller guarantees `server_certbuf_size` writable bytes at a non-null
    // `server_certbuf`, exclusively held for this call.
    let certs = unsafe { buffer::bytes_mut(server_certbuf, server_certbuf_size) };

    match connection.do_handshake_get_server_cert(certs) {
        Ok((data_size, count)) => {
            // SAFETY: the caller guarantees writable `u32`s at the out-pointers, or null.
            unsafe {
                buffer::write_out(out_size, data_size);
                buffer::write_out(total_certs, count);
            }
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Points at one certificate inside the chain a handshake collected.
///
/// This sends no request: the buffer is already in the caller's memory, and what the C API adds is
/// a reader for the header-and-entries layout the service wrote there. Every offset is checked
/// against the buffer before it becomes a pointer.
///
/// # Safety
///
/// `certbuf` must point to `certbuf_size` readable bytes holding what a handshake wrote, and each
/// out-pointer must be null or point to a writable value of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetServerCertDetail(
    certbuf: *const c_void,
    certbuf_size: u32,
    cert_index: u32,
    cert: *mut *mut c_void,
    cert_size: *mut u32,
) -> ResultCode {
    /// What the service stamps at the front of a certificate chain.
    const MAGIC: u64 = 0x4E4D_4344_5F43_4552;

    if certbuf.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `certbuf_size` readable bytes at a non-null `certbuf`.
    let bytes = unsafe { buffer::bytes(certbuf, certbuf_size) };

    let Some(header) = read_header(bytes) else {
        return result::bad_input();
    };
    if header.magicnum != MAGIC || cert_index >= header.cert_total {
        return result::bad_input();
    }

    let Some(entry) = read_entry(bytes, cert_index) else {
        return result::bad_input();
    };
    let end = entry.offset.checked_add(entry.size);
    if end.is_none_or(|end| end as usize > bytes.len()) {
        return result::bad_input();
    }

    // SAFETY: the offset and the size were just checked to land inside the caller's buffer, so the
    // result points into that same allocation.
    let at = unsafe { certbuf.cast::<u8>().add(entry.offset as usize) };
    // SAFETY: the caller guarantees writable values at the out-pointers, or null.
    unsafe {
        buffer::write_out(cert, at.cast_mut().cast::<c_void>());
        buffer::write_out(cert_size, entry.size);
    }
    result::OK
}

/// Reads data off the TLS connection.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `buffer` to `size`
/// writable bytes, and `out_size` must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionRead(
    connection: *mut c_void,
    buffer: *mut c_void,
    size: u32,
    out_size: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees `size` writable bytes at `buffer`, exclusively held for this
    // call.
    let buffer = unsafe { buffer::bytes_mut(buffer, size) };

    // SAFETY: the caller guarantees a writable `u32` at `out_size`, or null.
    unsafe { report_u32(connection.read(buffer), out_size) }
}

/// Writes data to the TLS connection.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `buffer` to `size`
/// readable bytes, and `out_size` must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionWrite(
    connection: *mut c_void,
    buffer: *const c_void,
    size: u32,
    out_size: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees `size` readable bytes at `buffer`.
    let buffer = unsafe { buffer::bytes(buffer, size) };

    // SAFETY: the caller guarantees a writable `u32` at `out_size`, or null.
    unsafe { report_u32(connection.write(buffer), out_size) }
}

/// Reports how many bytes are ready to read.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `i32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionPending(
    connection: *mut c_void,
    out: *mut i32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    match connection.pending() {
        Ok(pending) => {
            // SAFETY: the caller guarantees a writable `i32` at `out`, or null.
            unsafe { buffer::write_out(out, pending) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reads data without consuming it.
///
/// # Safety
///
/// As [`__nx_tls__sslConnectionRead`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionPeek(
    connection: *mut c_void,
    buffer: *mut c_void,
    size: u32,
    out_size: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees `size` writable bytes at `buffer`, exclusively held for this
    // call.
    let buffer = unsafe { buffer::bytes_mut(buffer, size) };

    // SAFETY: the caller guarantees a writable `u32` at `out_size`, or null.
    unsafe { report_u32(connection.peek(buffer), out_size) }
}

/// Waits for the connection to become readable or writable.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out_pollevent`
/// must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionPoll(
    connection: *mut c_void,
    in_pollevent: u32,
    out_pollevent: *mut u32,
    timeout: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    match connection.poll(PollEvent::from_bits_retain(in_pollevent), timeout) {
        Ok(events) => {
            // SAFETY: the caller guarantees a writable `u32` at `out_pollevent`, or null.
            unsafe { buffer::write_out(out_pollevent, events.bits()) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reads and clears the stored certificate verification error.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetVerifyCertError(
    connection: *mut c_void,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    result::report(connection.get_verify_cert_error())
}

/// Reports how large a buffer the server's certificate chain needs.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetNeededServerCertBufferSize(
    connection: *mut c_void,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
    unsafe { report_u32(connection.get_needed_server_cert_buffer_size(), out) }
}

/// Sets the session cache mode.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetSessionCacheMode(
    connection: *mut c_void,
    mode: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(mode) = SessionCacheMode::try_from(mode) else {
        return result::bad_input();
    };

    result::report(connection.set_session_cache_mode(mode))
}

/// Reads the session cache mode back.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetSessionCacheMode(
    connection: *mut c_void,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
    unsafe { report_u32(connection.get_session_cache_mode(), out) }
}

/// Drops this connection's cached session.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionFlushSessionCache(
    connection: *mut c_void,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    result::report(connection.flush_session_cache())
}

/// Sets the renegotiation mode.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetRenegotiationMode(
    connection: *mut c_void,
    mode: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(mode) = RenegotiationMode::try_from(mode) else {
        return result::bad_input();
    };

    result::report(connection.set_renegotiation_mode(mode))
}

/// Reads the renegotiation mode back.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetRenegotiationMode(
    connection: *mut c_void,
    out: *mut u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
    unsafe { report_u32(connection.get_renegotiation_mode(), out) }
}

/// Sets a connection option.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetOption(
    connection: *mut c_void,
    option: u32,
    flag: bool,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(option) = OptionType::try_from(option) else {
        return result::bad_input();
    };

    result::report(connection.set_option(option, flag))
}

/// Reads a connection option back.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `bool`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetOption(
    connection: *mut c_void,
    option: u32,
    out: *mut bool,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(option) = OptionType::try_from(option) else {
        return result::bad_input();
    };

    match connection.get_option(option) {
        Ok(flag) => {
            // SAFETY: the caller guarantees a writable `bool` at `out`, or null.
            unsafe { buffer::write_out(out, flag) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Collects the certificate verification errors into the caller's buffer.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `errors` to `count`
/// writable `u32`s, and each out-pointer must be null or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetVerifyCertErrors(
    connection: *mut c_void,
    out0: *mut u32,
    out1: *mut u32,
    errors: *mut u32,
    count: u32,
) -> ResultCode {
    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    if errors.is_null() || count == 0 {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `count` writable `u32`s at a non-null `errors`, exclusively
    // held for this call.
    let errors = unsafe { core::slice::from_raw_parts_mut(errors, count as usize) };

    match connection.get_verify_cert_errors(errors) {
        Ok((count_0, count_1)) => {
            // SAFETY: the caller guarantees writable `u32`s at the out-pointers, or null.
            unsafe {
                buffer::write_out(out0, count_0);
                buffer::write_out(out1, count_1);
            }
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reads the negotiated cipher (4.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must point to
/// a writable [`CipherInfo`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetCipherInfo(
    connection: *mut c_void,
    out: *mut CipherInfo,
) -> ResultCode {
    if !firmware::offers_cipher_info() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    if out.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees a writable `CipherInfo` at a non-null `out`, exclusively held
    // for this call.
    let out = unsafe { &mut *out };

    result::report(connection.get_cipher_info(out))
}

/// Offers a list of ALPN protocols (9.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `buffer` to `size`
/// readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetNextAlpnProto(
    connection: *mut c_void,
    buffer: *const u8,
    size: u32,
) -> ResultCode {
    if !firmware::offers_alpn() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    if buffer.is_null() {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `size` readable bytes at a non-null `buffer`.
    let protos = unsafe { buffer::bytes(buffer.cast(), size) };

    result::report(connection.set_next_alpn_proto(protos))
}

/// Reads the ALPN protocol that was negotiated (9.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `buffer` to `size`
/// writable bytes, and each out-pointer must be null or point to a writable value of its type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetNextAlpnProto(
    connection: *mut c_void,
    state: *mut u32,
    out: *mut u32,
    buffer: *mut u8,
    size: u32,
) -> ResultCode {
    if !firmware::offers_alpn() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees `size` writable bytes at `buffer`, exclusively held for this
    // call.
    let buffer = unsafe { buffer::bytes_mut(buffer.cast(), size) };

    match connection.get_next_alpn_proto(buffer) {
        Ok((negotiated, len)) => {
            // SAFETY: the caller guarantees writable values at the out-pointers, or null.
            unsafe {
                buffer::write_out(state, negotiated as u32);
                buffer::write_out(out, len);
            }
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reads the DTLS handshake timeout in nanoseconds (16.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetDtlsHandshakeTimeout(
    connection: *mut c_void,
    out: *mut u64,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    match connection.get_dtls_handshake_timeout() {
        Ok(timeout) => {
            // SAFETY: the caller guarantees a writable `u64` at `out`, or null.
            unsafe { buffer::write_out(out, timeout) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Sets a private option (16.0.0+).
///
/// The payload changed shape in `[17.0.0]`: before it, the option carried a flag, and after it a
/// value. Both are one C entry point, so the firmware decides which is sent.
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetPrivateOption(
    connection: *mut c_void,
    option: u32,
    value: u32,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    let Ok(option) = PrivateOptionType::try_from(option) else {
        return result::bad_input();
    };

    if firmware::offers_private_option_value() {
        result::report(connection.set_private_option(option, value))
    } else {
        result::report(connection.set_private_option_legacy(option, value != 0))
    }
}

/// Offers a list of SRTP ciphers (16.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `ciphers` to
/// `count` readable `u16`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetSrtpCiphers(
    connection: *mut c_void,
    ciphers: *const u16,
    count: u32,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    if ciphers.is_null() || count == 0 {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees `count` readable `u16`s at a non-null `ciphers`.
    let ciphers = unsafe { core::slice::from_raw_parts(ciphers, count as usize) };

    result::report(connection.set_srtp_ciphers(ciphers))
}

/// Reads the SRTP cipher that was negotiated (16.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u16`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetSrtpCipher(
    connection: *mut c_void,
    out: *mut u16,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    match connection.get_srtp_cipher() {
        Ok(cipher) => {
            // SAFETY: the caller guarantees a writable `u16` at `out`, or null.
            unsafe { buffer::write_out(out, cipher) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Derives keying material from the finished handshake (16.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, `outbuf` to
/// `outbuf_size` writable bytes, `label` to `label_size` readable bytes, and `context` to
/// `context_size` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionExportKeyingMaterial(
    connection: *mut c_void,
    outbuf: *mut u8,
    outbuf_size: u32,
    label: *const c_char,
    label_size: u32,
    context: *const c_void,
    context_size: u32,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    if outbuf.is_null() || outbuf_size == 0 {
        return result::bad_input();
    }

    // SAFETY: the caller guarantees the three buffers at the three lengths, with the output
    // exclusively held for this call.
    let (outbuf, label, context) = unsafe {
        (
            buffer::bytes_mut(outbuf.cast(), outbuf_size),
            buffer::bytes(label.cast(), label_size),
            buffer::bytes(context, context_size),
        )
    };

    result::report(connection.export_keying_material(outbuf, label, context))
}

/// Sets the I/O timeout (16.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionSetIoTimeout(
    connection: *mut c_void,
    timeout: u32,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    result::report(connection.set_io_timeout(timeout))
}

/// Reads the I/O timeout back (16.0.0+).
///
/// # Safety
///
/// `connection` must be null or point to a readable libnx `SslConnection`, and `out` must be null
/// or point to a writable `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_tls__sslConnectionGetIoTimeout(
    connection: *mut c_void,
    out: *mut u32,
) -> ResultCode {
    if !firmware::offers_dtls() {
        return result::incompat_sys_ver();
    }

    // SAFETY: the caller guarantees a readable `SslConnection`, or null.
    let Some(connection) = (unsafe { object::connection_at(connection) }) else {
        return result::not_initialized();
    };

    // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
    unsafe { report_u32(connection.get_io_timeout(), out) }
}

/// The header the service writes at the front of a certificate chain.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ServerCertDetailHeader {
    magicnum: u64,
    cert_total: u32,
    _pad: u32,
}

/// One entry in the table that follows the header.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct ServerCertDetailEntry {
    size: u32,
    offset: u32,
}

/// Reads the chain header, if the buffer is long enough to hold one.
fn read_header(bytes: &[u8]) -> Option<ServerCertDetailHeader> {
    let header = bytes.get(..size_of::<ServerCertDetailHeader>())?;

    // SAFETY: the slice was just checked to be exactly the struct's size, and every bit pattern of
    // its four integer fields is a valid value. The read is unaligned because the caller's buffer
    // carries no alignment guarantee.
    Some(unsafe {
        header
            .as_ptr()
            .cast::<ServerCertDetailHeader>()
            .read_unaligned()
    })
}

/// Reads the entry at `index`, if the buffer is long enough to hold one there.
fn read_entry(bytes: &[u8], index: u32) -> Option<ServerCertDetailEntry> {
    let start = size_of::<ServerCertDetailHeader>()
        .checked_add(size_of::<ServerCertDetailEntry>().checked_mul(index as usize)?)?;
    let entry = bytes.get(start..start.checked_add(size_of::<ServerCertDetailEntry>())?)?;

    // SAFETY: as `read_header`: the slice is exactly the struct's size, every bit pattern of its
    // two integers is valid, and the read is unaligned because the caller's buffer carries no
    // alignment guarantee.
    Some(unsafe {
        entry
            .as_ptr()
            .cast::<ServerCertDetailEntry>()
            .read_unaligned()
    })
}

/// Reports a command that answers with a socket descriptor, writing it through the out-pointer.
///
/// A connection that held none answers with a negative sentinel, which the command's return type
/// has already turned into an absence. What goes back is `-1`: the value the sentinel had is not
/// carried, and any negative number says the same thing to a C caller testing the sign.
///
/// # Safety
///
/// `out` must be null or point to a writable `c_int`.
unsafe fn report_descriptor(
    outcome: Result<Option<SocketFd>, nx_sf::service::DispatchError>,
    out: *mut c_int,
) -> ResultCode {
    match outcome {
        Ok(reported) => {
            // SAFETY: the caller guarantees a writable `c_int` at `out`, or null.
            unsafe { buffer::write_out(out, reported.map_or(-1, SocketFd::to_raw)) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}

/// Reports a command that answers with a `u32`, writing it through the caller's out-pointer.
///
/// Enough commands here share that shape that spelling it out at each would bury what they differ
/// in, which is the command itself.
///
/// # Safety
///
/// `out` must be null or point to a writable `u32`.
unsafe fn report_u32(
    outcome: Result<u32, nx_sf::service::DispatchError>,
    out: *mut u32,
) -> ResultCode {
    match outcome {
        Ok(value) => {
            // SAFETY: the caller guarantees a writable `u32` at `out`, or null.
            unsafe { buffer::write_out(out, value) };
            result::OK
        }
        Err(err) => err.to_rc(),
    }
}
