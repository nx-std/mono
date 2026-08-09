//! What a caller names when it wants to reach something.
//!
//! [`ToSocketAddrs`] is this crate's counterpart to `std::net::ToSocketAddrs`,
//! and exists for the same reason: a connect call should accept the form the
//! caller already has — a literal address, an address and a port, a host name
//! and a port, a `"host:port"` string — rather than forcing each one through a
//! conversion the caller writes itself.
//!
//! # The one difference from `std`
//!
//! `std`'s trait method takes no resolver, because on a hosted platform the
//! resolver is ambient. Here it is a parameter. Nothing in this crate's
//! idiomatic API owns a process-wide `sfdnsres` session — the C surface owns
//! one, because a C caller of `getaddrinfo` has no way to pass it, and that is
//! the only layer that should. Threading the session through keeps the rest of
//! the crate injectable, which is what lets a caller decide the session's
//! lifetime.
//!
//! # Which forms resolve, and which do not
//!
//! Only a form carrying a host name reaches the resolver. An address that is
//! already numeric is returned as it stands, with no round-trip — the same
//! split `std` makes, and the reason a caller can use one trait for both
//! without paying for a lookup it does not need.

use alloc::vec;
use core::{
    iter,
    net::{
        IpAddr,
        Ipv4Addr,
        Ipv6Addr,
        SocketAddr,
        SocketAddrV4,
        SocketAddrV6,
    },
};

use nx_service_sfdnsres::SfdnsresService;

use crate::resolve::{
    LookupHost,
    hostname::{
        Hostname,
        HostnameError,
    },
    lookup_host,
    resolver::ResolveError,
};

/// A value that names one or more socket addresses to reach.
///
/// See the [module docs](self) for how this differs from
/// `std::net::ToSocketAddrs`.
pub trait ToSocketAddrs {
    /// The iterator this target yields.
    type Iter: Iterator<Item = SocketAddr>;

    /// Produces the addresses this target names.
    ///
    /// `svc` is only consulted by targets that carry a host name; a numeric
    /// target never reaches it.
    ///
    /// # Errors
    ///
    /// [`ToSocketAddrsError::Resolve`] when a lookup was needed and failed,
    /// and the parse variants when a string target is not `"host:port"`.
    fn to_socket_addrs(&self, svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError>;
}

/// Errors returned by [`ToSocketAddrs::to_socket_addrs`].
#[derive(Debug, thiserror::Error)]
pub enum ToSocketAddrsError {
    /// The string target carries no `:port` suffix.
    ///
    /// Detected before any lookup, so nothing was sent.
    #[error("the target names no port")]
    MissingPort,

    /// The string target's port is not a number in `0..=65535`.
    ///
    /// Detected before any lookup, so nothing was sent.
    #[error("the target's port is not a valid number")]
    InvalidPort(#[source] core::num::ParseIntError),

    /// The string target's host part is not a usable host name.
    ///
    /// Detected before any lookup, so nothing was sent.
    #[error("the target's host is not a valid hostname")]
    InvalidHost(#[source] HostnameError),

    /// The host name was well-formed but the resolver did not answer with an
    /// address.
    #[error("failed to resolve the target")]
    Resolve(#[source] ResolveError),
}

impl ToSocketAddrs for SocketAddr {
    type Iter = iter::Once<SocketAddr>;

    fn to_socket_addrs(&self, _svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        Ok(iter::once(*self))
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = iter::Once<SocketAddr>;

    fn to_socket_addrs(&self, _svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        Ok(iter::once(SocketAddr::V4(*self)))
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = iter::Once<SocketAddr>;

    fn to_socket_addrs(&self, _svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        Ok(iter::once(SocketAddr::V6(*self)))
    }
}

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = iter::Once<SocketAddr>;

    fn to_socket_addrs(&self, _svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        let (ip, port) = *self;
        Ok(iter::once(SocketAddr::new(ip, port)))
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = iter::Once<SocketAddr>;

    fn to_socket_addrs(&self, _svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        let (ip, port) = *self;
        Ok(iter::once(SocketAddr::from(SocketAddrV4::new(ip, port))))
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = iter::Once<SocketAddr>;

    fn to_socket_addrs(&self, _svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        let (ip, port) = *self;
        Ok(iter::once(SocketAddr::from(SocketAddrV6::new(
            ip, port, 0, 0,
        ))))
    }
}

impl ToSocketAddrs for (&Hostname, u16) {
    type Iter = vec::IntoIter<SocketAddr>;

    /// The resolving case: the name goes to `sfdnsres`, and `port` replaces
    /// the zero port the resolver returns for a lookup with no service.
    fn to_socket_addrs(&self, svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        let (host, port) = *self;
        let addrs: vec::Vec<SocketAddr> = lookup_host(host, svc)
            .map_err(ToSocketAddrsError::Resolve)?
            .map(|mut addr| {
                addr.set_port(port);
                addr
            })
            .collect();
        Ok(addrs.into_iter())
    }
}

impl ToSocketAddrs for str {
    type Iter = vec::IntoIter<SocketAddr>;

    /// Accepts `"host:port"`, where the host is either a numeric address or a
    /// name.
    ///
    /// A fully numeric target parses without a lookup, which also covers the
    /// bracketed IPv6 form `"[::1]:80"` — `SocketAddr`'s own parser owns that
    /// syntax, so this never has to.
    fn to_socket_addrs(&self, svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        if let Ok(addr) = self.parse::<SocketAddr>() {
            return Ok(vec::Vec::from([addr]).into_iter());
        }

        // Split from the right: a name contains no colon, so the last one is
        // the port separator for every form that reaches here.
        let Some((host, port)) = self.rsplit_once(':') else {
            return Err(ToSocketAddrsError::MissingPort);
        };
        let port: u16 = port.parse().map_err(ToSocketAddrsError::InvalidPort)?;
        let host: Hostname = host.parse().map_err(ToSocketAddrsError::InvalidHost)?;

        (&host, port).to_socket_addrs(svc)
    }
}

impl<T> ToSocketAddrs for &T
where
    T: ToSocketAddrs + ?Sized,
{
    type Iter = T::Iter;

    fn to_socket_addrs(&self, svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        (**self).to_socket_addrs(svc)
    }
}

/// The iterator [`lookup_host`] returns, so a caller holding one can feed it
/// where a target is expected.
impl ToSocketAddrs for LookupHost {
    type Iter = LookupHost;

    fn to_socket_addrs(&self, _svc: &SfdnsresService) -> Result<Self::Iter, ToSocketAddrsError> {
        Ok(self.clone())
    }
}
