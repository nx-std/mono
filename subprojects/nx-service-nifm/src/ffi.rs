//! The C boundary's view of an `IRequest`.
//!
//! This module defines no `__nx_*` symbol of its own. What it holds is the shape another crate's
//! C entry points address: a request object a C caller created and closes, which this crate can
//! send commands to without owning. That is built for a C boundary, so a pure-Rust link should not
//! pay for it, and the `ffi` feature is what keeps it out of one.

use nx_sf::service::{
    DispatchError,
    ForeignDomainObject,
};

use crate::{
    cmif::request,
    types::SocketFd,
};

/// An `IRequest` a C caller owns.
///
/// Reached through `nx_sf::ffi::Service::as_domain_object`, which is the only source of the
/// [`ForeignDomainObject`] this wraps. It closes nothing: the C caller created the request and
/// closes it, and this only sends commands to it.
///
/// It carries the socket-descriptor commands rather than the request's whole surface, and the
/// bound is not only about demand: the rest of [`NifmRequest`](crate::NifmRequest) reads the two readable events and
/// the state they cache, and a foreign service struct records neither. What is left is what an
/// object id alone can answer. Each command here is the same function [`NifmRequest`](crate::NifmRequest) calls: the
/// bodies take [`DomainTarget`](nx_sf::service::DomainTarget), so neither form has a copy of its
/// own.
#[derive(Debug, Clone, Copy)]
pub struct ForeignNifmRequest<'a> {
    object: ForeignDomainObject<'a>,
}

impl<'a> ForeignNifmRequest<'a> {
    /// Views the `IRequest` object `object` addresses.
    #[inline]
    pub fn new(object: ForeignDomainObject<'a>) -> Self {
        Self { object }
    }

    /// `RegisterSocketDescriptor` (cmd 24, `[3.0.0+]`).
    pub fn register_socket_descriptor(
        &self,
        sockfd: impl Into<SocketFd>,
    ) -> Result<(), DispatchError> {
        request::register_socket_descriptor(self.object, sockfd)
    }

    /// `UnregisterSocketDescriptor` (cmd 25, `[3.0.0+]`).
    pub fn unregister_socket_descriptor(
        &self,
        sockfd: impl Into<SocketFd>,
    ) -> Result<(), DispatchError> {
        request::unregister_socket_descriptor(self.object, sockfd)
    }
}
