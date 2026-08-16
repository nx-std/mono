//! The C boundary's view of an `ISslConnection`.

use nx_service_bsd::RawSockAddr;
use nx_sf::service::{
    DispatchError,
    ForeignDomainObject,
};

use crate::{
    cmif,
    connection::{
        AlpnProtoState,
        CipherInfo,
        IoMode,
        OptionType,
        PollEvent,
        PrivateOptionType,
        RenegotiationMode,
        SessionCacheMode,
        VerifyOption,
    },
    socket::SocketFd,
};

/// An `ISslConnection` a C caller owns.
///
/// Reached through [`nx_sf::ffi::Service::as_domain_object`], which is the only source of the
/// [`ForeignDomainObject`] this wraps. It closes nothing: the C caller holds the close obligation,
/// and this only sends commands to it.
///
/// It carries the same surface as [`SslConnection`](crate::SslConnection), because the C API it
/// stands behind exposes the same one. Each method is the same command body that type calls: those
/// take [`DomainTarget`](nx_sf::service::DomainTarget), so neither form has a copy of its own.
#[derive(Debug, Clone, Copy)]
pub struct ForeignSslConnection<'a> {
    object: ForeignDomainObject<'a>,
}

impl<'a> ForeignSslConnection<'a> {
    /// Views the `ISslConnection` object `object` addresses.
    #[inline]
    pub fn new(object: ForeignDomainObject<'a>) -> Self {
        Self { object }
    }

    /// Sets the socket descriptor. Returns the one the connection gave up, if it held one.
    pub fn set_socket_descriptor(
        &self,
        sockfd: impl Into<SocketFd>,
    ) -> Result<Option<SocketFd>, DispatchError> {
        cmif::set_socket_descriptor(self.object, sockfd)
    }

    /// Sets the host name for TLS verification.
    pub fn set_host_name(&self, name: &[u8]) -> Result<(), DispatchError> {
        cmif::set_host_name(self.object, name)
    }

    /// Sets the verify option bitmask.
    pub fn set_verify_option(&self, options: VerifyOption) -> Result<(), DispatchError> {
        cmif::set_verify_option(self.object, options)
    }

    /// Sets the I/O mode.
    pub fn set_io_mode(&self, mode: IoMode) -> Result<(), DispatchError> {
        cmif::set_io_mode(self.object, mode)
    }

    /// Gets the socket descriptor the connection holds, if it holds one.
    pub fn get_socket_descriptor(&self) -> Result<Option<SocketFd>, DispatchError> {
        cmif::get_socket_descriptor(self.object)
    }

    /// Gets the host name string. Returns the string length.
    pub fn get_host_name(&self, buffer: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::get_host_name(self.object, buffer)
    }

    /// Gets the verify option bitmask.
    pub fn get_verify_option(&self) -> Result<VerifyOption, DispatchError> {
        cmif::get_verify_option(self.object).map(VerifyOption::from_bits_retain)
    }

    /// Gets the I/O mode.
    pub fn get_io_mode(&self) -> Result<u32, DispatchError> {
        cmif::get_io_mode(self.object)
    }

    /// Performs a TLS handshake without requesting server cert.
    pub fn do_handshake(&self) -> Result<(), DispatchError> {
        cmif::do_handshake(self.object)
    }

    /// Performs a TLS handshake and retrieves server cert data.
    ///
    /// Returns `(data_size, total_certs)`.
    pub fn do_handshake_get_server_cert(
        &self,
        server_certbuf: &mut [u8],
    ) -> Result<(u32, u32), DispatchError> {
        let out = cmif::do_handshake_get_server_cert(self.object, server_certbuf)?;
        Ok((out.data_size, out.total_certs))
    }

    /// Reads data from the TLS connection. Returns bytes transferred.
    pub fn read(&self, buffer: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::read(self.object, buffer)
    }

    /// Writes data to the TLS connection. Returns bytes transferred.
    pub fn write(&self, buffer: &[u8]) -> Result<u32, DispatchError> {
        cmif::write(self.object, buffer)
    }

    /// Gets the number of pending bytes.
    pub fn pending(&self) -> Result<i32, DispatchError> {
        cmif::pending(self.object)
    }

    /// Peeks at data without consuming it. Returns bytes read.
    pub fn peek(&self, buffer: &mut [u8]) -> Result<u32, DispatchError> {
        cmif::peek(self.object, buffer)
    }

    /// Polls the connection for events.
    pub fn poll(&self, in_pollevent: PollEvent, timeout: u32) -> Result<PollEvent, DispatchError> {
        cmif::poll(self.object, in_pollevent, timeout).map(PollEvent::from_bits_retain)
    }

    /// Gets the verify cert error (clears the stored value).
    pub fn get_verify_cert_error(&self) -> Result<(), DispatchError> {
        cmif::get_verify_cert_error(self.object)
    }

    /// Gets the needed server cert buffer size.
    pub fn get_needed_server_cert_buffer_size(&self) -> Result<u32, DispatchError> {
        cmif::get_needed_server_cert_buffer_size(self.object)
    }

    /// Sets the session cache mode.
    pub fn set_session_cache_mode(&self, mode: SessionCacheMode) -> Result<(), DispatchError> {
        cmif::set_session_cache_mode(self.object, mode)
    }

    /// Gets the session cache mode.
    pub fn get_session_cache_mode(&self) -> Result<u32, DispatchError> {
        cmif::get_session_cache_mode(self.object)
    }

    /// Flushes the connection's session cache.
    pub fn flush_session_cache(&self) -> Result<(), DispatchError> {
        cmif::flush_session_cache(self.object)
    }

    /// Sets the renegotiation mode.
    pub fn set_renegotiation_mode(&self, mode: RenegotiationMode) -> Result<(), DispatchError> {
        cmif::set_renegotiation_mode(self.object, mode)
    }

    /// Gets the renegotiation mode.
    pub fn get_renegotiation_mode(&self) -> Result<u32, DispatchError> {
        cmif::get_renegotiation_mode(self.object)
    }

    /// Sets a connection option.
    pub fn set_option(&self, option: OptionType, flag: bool) -> Result<(), DispatchError> {
        cmif::set_option(self.object, option, flag)
    }

    /// Gets a connection option.
    pub fn get_option(&self, option: OptionType) -> Result<bool, DispatchError> {
        cmif::get_option(self.object, option)
    }

    /// Gets verify cert errors into a buffer.
    ///
    /// Returns the two counts the command reports, which a caller is expected to compare.
    pub fn get_verify_cert_errors(&self, errors: &mut [u32]) -> Result<(u32, u32), DispatchError> {
        cmif::get_verify_cert_errors(self.object, errors)
    }

    /// Gets cipher info (4.0.0+).
    pub fn get_cipher_info(&self, out: &mut CipherInfo) -> Result<(), DispatchError> {
        cmif::get_cipher_info(self.object, out)
    }

    /// Sets the next ALPN protocol list (9.0.0+).
    pub fn set_next_alpn_proto(&self, proto_list: &[u8]) -> Result<(), DispatchError> {
        cmif::set_next_alpn_proto(self.object, proto_list)
    }

    /// Gets the next ALPN protocol (9.0.0+).
    ///
    /// Returns `(state, proto_size)`. The protocol string is written to `buffer`.
    pub fn get_next_alpn_proto(
        &self,
        buffer: &mut [u8],
    ) -> Result<(AlpnProtoState, u32), DispatchError> {
        let out = cmif::get_next_alpn_proto(self.object, buffer)?;
        Ok((AlpnProtoState::from_raw(out.state), out.proto_size))
    }

    /// Sets DTLS socket descriptor (16.0.0+). Returns the one the connection gave up, if any.
    pub fn set_dtls_socket_descriptor(
        &self,
        sockfd: impl Into<SocketFd>,
        sockaddr: &RawSockAddr,
    ) -> Result<Option<SocketFd>, DispatchError> {
        cmif::set_dtls_socket_descriptor(self.object, sockfd, sockaddr)
    }

    /// Gets DTLS handshake timeout in nanoseconds (16.0.0+).
    pub fn get_dtls_handshake_timeout(&self) -> Result<u64, DispatchError> {
        cmif::get_dtls_handshake_timeout(self.object)
    }

    /// Sets a private option (pre-17.0.0, bool+option layout).
    pub fn set_private_option_legacy(
        &self,
        option: PrivateOptionType,
        value: bool,
    ) -> Result<(), DispatchError> {
        cmif::set_private_option_legacy(self.object, option, value)
    }

    /// Sets a private option (17.0.0+, option+value layout).
    pub fn set_private_option(
        &self,
        option: PrivateOptionType,
        value: u32,
    ) -> Result<(), DispatchError> {
        cmif::set_private_option(self.object, option, value)
    }

    /// Sets SRTP ciphers (16.0.0+).
    pub fn set_srtp_ciphers(&self, ciphers: &[u16]) -> Result<(), DispatchError> {
        cmif::set_srtp_ciphers(self.object, ciphers)
    }

    /// Gets the negotiated SRTP cipher (16.0.0+).
    pub fn get_srtp_cipher(&self) -> Result<u16, DispatchError> {
        cmif::get_srtp_cipher(self.object)
    }

    /// Exports keying material (16.0.0+).
    pub fn export_keying_material(
        &self,
        outbuf: &mut [u8],
        label: &[u8],
        context: &[u8],
    ) -> Result<(), DispatchError> {
        cmif::export_keying_material(self.object, outbuf, label, context)
    }

    /// Sets I/O timeout (16.0.0+).
    pub fn set_io_timeout(&self, timeout: u32) -> Result<(), DispatchError> {
        cmif::set_io_timeout(self.object, timeout)
    }

    /// Gets I/O timeout (16.0.0+).
    pub fn get_io_timeout(&self) -> Result<u32, DispatchError> {
        cmif::get_io_timeout(self.object)
    }
}
