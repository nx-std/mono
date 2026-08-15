//! The C boundary's view of an `ISslConnection`.
//!
//! This module defines no `__nx_*` symbol of its own. What it holds is the shape another crate's
//! C entry points address: a connection object a C caller created and closes, which this crate can
//! send commands to without owning. That is built for a C boundary, so a pure-Rust link should not
//! pay for it, and the `ffi` feature is what keeps it out of one.

use nx_service_bsd::RawSockAddr;
use nx_sf::service::{
    DispatchError,
    ForeignDomainObject,
};

use crate::{
    cmif,
    types::SocketFd,
};

/// An `ISslConnection` a C caller owns.
///
/// Reached through `nx_sf::ffi::Service::as_domain_object`, which is the only source of the
/// [`ForeignDomainObject`] this wraps. It closes nothing: the C caller created the connection and
/// closes it, and this only sends commands to it.
///
/// It carries the socket-descriptor commands rather than the connection's whole surface, because
/// those are the ones a caller holding somebody else's connection has a reason to send. Each is
/// the same function [`SslConnection`](crate::SslConnection) calls: the command bodies take
/// [`DomainTarget`](nx_sf::service::DomainTarget), so neither form has a copy of its own.
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

    /// Gets the socket descriptor the connection holds, if it holds one.
    pub fn get_socket_descriptor(&self) -> Result<Option<SocketFd>, DispatchError> {
        cmif::get_socket_descriptor(self.object)
    }

    /// Sets DTLS socket descriptor (16.0.0+). Returns the one the connection gave up, if any.
    pub fn set_dtls_socket_descriptor(
        &self,
        sockfd: impl Into<SocketFd>,
        sockaddr: &RawSockAddr,
    ) -> Result<Option<SocketFd>, DispatchError> {
        cmif::set_dtls_socket_descriptor(self.object, sockfd, sockaddr)
    }
}
